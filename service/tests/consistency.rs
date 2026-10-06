//! Service consistency tests (SERVICE_CONSISTENCY_PLAN.md).
//!
//! Each test reproduces one defect S# of the plan against the current service. A test that still
//! fails on the current code is marked `#[ignore = "S#: …"]` and the phase that fixes the defect
//! removes the marker. Run the pending ones with `cargo test --test consistency -- --ignored`.

use meritrank_service::aug_graph::AugGraph;
use meritrank_service::data::{
  AugGraphOp, EdgeResult, OpReadMutualScores, OpWriteCalculate, OpWriteEdge,
  ReqData, Request, ResEdges, ResScores, Response,
};
use meritrank_service::settings::Settings;
use meritrank_service::state_manager::{GraphProcessor, MultiGraphProcessor};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio::time::timeout;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn settings(num_walks: usize) -> Settings {
  Settings {
    num_walks,
    zero_opinion_factor: 0.0,
    ..Settings::default()
  }
}

fn edge(
  src: &str,
  dst: &str,
  amount: f64,
) -> OpWriteEdge {
  OpWriteEdge {
    src: src.into(),
    dst: dst.into(),
    amount,
    magnitude: 0,
  }
}

async fn request(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  data: ReqData,
) -> Response {
  proc
    .process_request(&Request {
      subgraph: subgraph.into(),
      data,
    })
    .await
}

async fn write(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  src: &str,
  dst: &str,
  amount: f64,
) {
  request(proc, subgraph, ReqData::WriteEdge(edge(src, dst, amount))).await;
}

async fn sync(
  proc: &MultiGraphProcessor,
  stamp: u64,
) {
  request(proc, "", ReqData::Sync(stamp)).await;
}

async fn edges(
  proc: &MultiGraphProcessor,
  subgraph: &str,
) -> Vec<(String, String, f64)> {
  match request(proc, subgraph, ReqData::ReadEdges).await {
    Response::Edges(ResEdges { edges }) => edges
      .into_iter()
      .map(|e: EdgeResult| (e.src, e.dst, e.weight))
      .collect(),
    other => panic!("expected edges, got {:?}", other),
  }
}

fn weight_of(
  edges: &[(String, String, f64)],
  src: &str,
  dst: &str,
) -> Option<f64> {
  edges
    .iter()
    .find(|(s, d, _)| s == src && d == dst)
    .map(|(_, _, w)| *w)
}

/// Weight of `src → dst` in the copy currently published by a single processor.
fn published_weight(
  proc: &GraphProcessor,
  src: &str,
  dst: &str,
) -> Option<f64> {
  let arc = proc.shared.load_full();
  let g = arc.read();
  let s = g.nodes.get_by_name(src)?.id;
  let d = g.nodes.get_by_name(dst)?.id;
  g.mr.graph.edge_weight(s, d).ok().flatten()
}

/// Sends `Stamp(k)` and waits until a copy with stamp ≥ k is published.
async fn stamp_and_wait(
  proc: &GraphProcessor,
  notify: &Notify,
  k: u64,
) {
  let wait = async {
    proc.op_sender.send(AugGraphOp::Stamp(k)).await.unwrap();
    loop {
      let notified = notify.notified();
      if proc.shared.load().read().stamp >= k {
        return;
      }
      notified.await;
    }
  };
  timeout(Duration::from_secs(5), wait)
    .await
    .unwrap_or_else(|_| panic!("stamp {k} was never published"));
}

/// Like `stamp_and_wait`, but tolerates S14 (a lagging copy published over a newer one): keeps
/// sending further stamps until one is visible, so tests of other defects are not blocked by it.
async fn flush(
  proc: &GraphProcessor,
  notify: &Notify,
  next_stamp: &mut u64,
) {
  for _ in 0..20 {
    *next_stamp += 1;
    let k = *next_stamp;
    proc.op_sender.send(AugGraphOp::Stamp(k)).await.unwrap();
    let wait = async {
      loop {
        let notified = notify.notified();
        if proc.shared.load().read().stamp >= k {
          return;
        }
        notified.await;
      }
    };
    if timeout(Duration::from_millis(100), wait).await.is_ok() {
      return;
    }
  }
  panic!("no stamp became visible after 20 attempts (S14: a lagging copy keeps being published)");
}

// ---------------------------------------------------------------------------
// S1: the two buffer copies must apply writes in one order
// ---------------------------------------------------------------------------

/// Concurrent writers to one edge, then walks: every published copy must hold the same weight and
/// bit-identical scores. Before phase 2, `FanoutSender::send` awaited two queues separately, so
/// two writers could interleave (a1, a2, b2, b1), and each copy drew its own random walks.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn s1_buffer_copies_are_replicas() {
  for trial in 0..40 {
    let notify = Arc::new(Notify::new());
    let proc = GraphProcessor::new(
      AugGraph::new(settings(10)),
      1,
      1,
      Arc::clone(&notify),
      None,
      0,
    );

    let mut js = JoinSet::new();
    for i in 1..=16 {
      let sender = proc.op_sender.clone();
      js.spawn(async move {
        sender
          .send(AugGraphOp::WriteEdge(edge("U1", "U2", i as f64)))
          .await
      });
    }
    while let Some(r) = js.join_next().await {
      r.unwrap().unwrap();
    }

    // Random walks too: both copies must draw the same streams.
    for (s, d) in [("U2", "U3"), ("U3", "U1"), ("U2", "U4")] {
      proc
        .op_sender
        .send(AugGraphOp::WriteEdge(edge(s, d, 1.0)))
        .await
        .unwrap();
    }
    proc
      .op_sender
      .send(AugGraphOp::WriteCalculate(OpWriteCalculate {
        ego: "U1".into(),
      }))
      .await
      .unwrap();
    proc
      .op_sender
      .send(AugGraphOp::WriteEdge(edge("U3", "U4", 2.0)))
      .await
      .unwrap();

    // Each flush publishes a copy; consecutive publications alternate between the two copies.
    let mut stamp = 0;
    let mut seen = vec![];
    for _ in 0..6 {
      flush(&proc, &notify, &mut stamp).await;
      let scores = proc.read(|g| {
        let u1 = g.nodes.get_by_name("U1").unwrap().id;
        g.mr.get_all_scores(u1, None).unwrap()
      });
      seen.push((published_weight(&proc, "U1", "U2"), scores));
    }
    seen.dedup();
    assert!(
      seen.len() == 1,
      "trial {trial}: buffer copies diverged: {} distinct published states",
      seen.len()
    );
    proc.shutdown().ok();
  }
}

// ---------------------------------------------------------------------------
// S2 (D14): concurrent writes to one context keep one order in both copies; no fan-out
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn s2_user_edges_reach_contexts_in_one_order() {
  for trial in 0..100 {
    let proc = Arc::new(MultiGraphProcessor::new(settings(10)));
    for ctx in ["X", "Y", "Z"] {
      request(&proc, ctx, ReqData::WriteCreateContext).await;
    }

    let mut js = JoinSet::new();
    for i in 1..=16 {
      let p = Arc::clone(&proc);
      js.spawn(async move { write(&p, "X", "U1", "U2", i as f64).await });
    }
    while let Some(r) = js.join_next().await {
      r.unwrap();
    }
    sync(&proc, 1_000 + trial).await;

    // Isolated contexts: the writes reached X only.
    assert!(weight_of(&edges(&proc, "X").await, "U1", "U2").is_some(), "trial {trial}");
    for ctx in ["", "Y", "Z"] {
      assert_eq!(weight_of(&edges(&proc, ctx).await, "U1", "U2"), None, "trial {trial}: {ctx:?}");
    }
  }
}

// ---------------------------------------------------------------------------
// S3: mr_sync must be a barrier whatever stamp the caller passes
// ---------------------------------------------------------------------------

/// Every PostgreSQL backend starts its stamp counter at 0. After any client synced with a large
/// stamp, a new backend's `sync(1)` must still wait for its own preceding write.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s3_sync_waits_for_preceding_writes() {
  // A ring the ego walks around, so that a new edge out of the ego repairs many walks and the
  // write takes long enough to be observed in flight.
  let proc = MultiGraphProcessor::new(settings(100_000));
  for i in 0..10 {
    write(&proc, "", &format!("U{i}"), &format!("U{}", (i + 1) % 10), 1.0)
      .await;
  }
  request(
    &proc,
    "",
    ReqData::WriteCalculate(OpWriteCalculate {
      ego: "U0".into(),
    }),
  )
  .await;
  sync(&proc, 100).await; // another backend, further along

  write(&proc, "", "U0", "U5", 1.0).await;
  sync(&proc, 1).await; // a fresh backend's first sync

  let now = edges(&proc, "").await;
  assert!(
    weight_of(&now, "U0", "U5").is_some(),
    "sync returned before the preceding write was applied"
  );
}

// ---------------------------------------------------------------------------
// S5: a reader that loaded a copy just before a swap must not stall
// ---------------------------------------------------------------------------

/// A reader loads the published Arc, then takes its read lock. If a swap happens in between, the
/// worker write-locks that copy and holds the lock while waiting for the next operation, so the
/// read waits until somebody writes again. Here nobody does.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s5_stale_reader_does_not_stall() {
  let notify = Arc::new(Notify::new());
  let proc = GraphProcessor::new(
    AugGraph::new(settings(10)),
    10,
    1,
    Arc::clone(&notify),
    None,
    0,
  );

  for k in 1..=50 {
    // Catch the copy that is published only transiently during the swap sequence.
    let resting = proc.shared.load_full();
    let shared = Arc::clone(&proc.shared);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_spin = Arc::clone(&stop);
    let spinner = std::thread::spawn(move || {
      while !stop_spin.load(Ordering::Relaxed) {
        let cur = shared.load_full();
        if !Arc::ptr_eq(&cur, &resting) {
          return Some(cur);
        }
      }
      None
    });

    stamp_and_wait(&proc, &notify, k).await;
    tokio::time::sleep(Duration::from_millis(20)).await; // let the worker go idle
    stop.store(true, Ordering::Relaxed);
    let caught = spinner.join().unwrap();

    if let Some(stale) = caught {
      let got = stale.try_read_for(Duration::from_millis(500)).is_some();
      assert!(got, "iteration {k}: a stale reader stalled behind the idle worker");
    }
  }
  proc.shutdown().ok();
}

// ---------------------------------------------------------------------------
// S6: no batching threshold may stall the worker or a barrier
// ---------------------------------------------------------------------------

/// `queue_len = 1, min_ops_before_swap = 2`: the worker waits on the back queue for a second
/// operation while the sender is blocked on the full front queue.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s6_small_queue_and_batch_do_not_deadlock() {
  let notify = Arc::new(Notify::new());
  let proc = GraphProcessor::new(
    AugGraph::new(settings(10)),
    1,
    2,
    Arc::clone(&notify),
    None,
    0,
  );
  let sends = async {
    for k in 1..=4 {
      proc.op_sender.send(AugGraphOp::Stamp(k)).await.unwrap();
    }
  };
  assert!(
    timeout(Duration::from_secs(3), sends).await.is_ok(),
    "sender deadlocked with queue_len = 1, min_ops_before_swap = 2"
  );
}

/// A lone barrier must publish even when the batch threshold is not reached (fixed in phase 2:
/// `Stamp` is urgent and ends a batch).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s6_lone_sync_publishes_below_batch_threshold() {
  let proc = MultiGraphProcessor::new(Settings {
    min_ops_before_swap: 3,
    ..settings(10)
  });
  write(&proc, "", "U1", "U2", 1.0).await;
  assert!(
    timeout(Duration::from_secs(3), sync(&proc, 1)).await.is_ok(),
    "mr_sync never returned below the batch threshold"
  );
}

// ---------------------------------------------------------------------------
// S7: reverse scores must reflect the current graph after a sync
// ---------------------------------------------------------------------------

/// U1's reverse score for U2 is U1's score in U2's frame. Once U2 stops trusting U1 (and U1 is
/// unreachable from U2), it must drop to 0 after a sync. Today the first read caches it for an
/// hour and later reads return the cached value.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s7_reverse_score_is_fresh_after_sync() {
  let proc = MultiGraphProcessor::new(settings(2_000));
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "", "U2", "U1", 1.0).await;
  sync(&proc, 1).await;

  let reverse = |resp: Response| -> f64 {
    match resp {
      Response::Scores(ResScores { scores }) => scores
        .into_iter()
        .find(|s| s.target == "U2")
        .map(|s| s.reverse_score)
        .expect("U2 in U1's mutual scores"),
      other => panic!("expected scores, got {:?}", other),
    }
  };
  let mutual = || ReqData::ReadMutualScores(OpReadMutualScores {
    ego: "U1".into(),
  });

  let before = reverse(request(&proc, "", mutual()).await);
  assert!(before > 0.0, "setup: U1 must score in U2's frame, got {before}");

  write(&proc, "", "U2", "U1", 0.0).await;
  write(&proc, "", "U2", "U3", 1.0).await;
  sync(&proc, 2).await;

  let after = reverse(request(&proc, "", mutual()).await);
  assert!(
    after == 0.0,
    "stale reverse score after sync: {after} (was {before})"
  );
}

// ---------------------------------------------------------------------------
// S14: the published state must never move backwards
// ---------------------------------------------------------------------------

/// `FanoutSender` fills queue A before queue B, so B can lag. After publishing A the worker
/// catches B up only with what is already in B's queue and publishes it at once: the published
/// copy is then older than the one before it, and stays so until the next write. Here a writer
/// keeps adding distinct edges while a reader watches the published edge count.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn s14_publication_is_monotonic() {
  let notify = Arc::new(Notify::new());
  let proc = GraphProcessor::new(
    AugGraph::new(settings(10)),
    2,
    1,
    Arc::clone(&notify),
    None,
    0,
  );

  let shared = Arc::clone(&proc.shared);
  let stop = Arc::new(AtomicBool::new(false));
  let stop_reader = Arc::clone(&stop);
  let reader = std::thread::spawn(move || {
    let mut last = 0usize;
    while !stop_reader.load(Ordering::Relaxed) {
      let arc = shared.load_full();
      let count = arc
        .read()
        .mr
        .graph
        .get_node_data(0)
        .map(|d| d.pos_edges.len())
        .unwrap_or(0);
      if count < last {
        return Some((last, count));
      }
      last = count;
    }
    None
  });

  let mut js = JoinSet::new();
  for w in 0..8 {
    let sender = proc.op_sender.clone();
    js.spawn(async move {
      for i in 0..50 {
        let dst = format!("U{}", 1 + w * 50 + i);
        sender
          .send(AugGraphOp::WriteEdge(edge("U0", &dst, 1.0)))
          .await
          .unwrap();
      }
    });
  }
  while let Some(r) = js.join_next().await {
    r.unwrap();
  }
  tokio::time::sleep(Duration::from_millis(50)).await;
  stop.store(true, Ordering::Relaxed);
  let regression = reader.join().unwrap();
  assert!(
    regression.is_none(),
    "published state moved backwards: edge count {:?}",
    regression
  );
}

// ---------------------------------------------------------------------------
// S12: the same seed and the same operations give the same walks (phase 1)
// ---------------------------------------------------------------------------

/// Two graphs built from settings with the same seed and fed the same operations — as the two
/// buffer copies are — end with identical scores; a different seed gives different ones.
#[test]
fn s12_same_seed_same_scores() {
  let run = |seed: u64| {
    let mut g = AugGraph::new(Settings {
      seed,
      ..settings(2_000)
    });
    for i in 0..10 {
      g.set_edge(format!("U{i}"), format!("U{}", (i + 1) % 10), 1.0, 0);
      g.set_edge(format!("U{i}"), format!("U{}", (i + 4) % 10), 0.5, 0);
    }
    g.calculate("U0".into());
    g.calculate("U3".into());
    g.set_edge("U2".into(), "U7".into(), 2.0, 0);
    g.set_edge("U5".into(), "U6".into(), 0.0, 0);
    let id = |g: &AugGraph, n: &str| g.nodes.get_by_name(n).unwrap().id;
    let (u0, u3) = (id(&g, "U0"), id(&g, "U3"));
    (
      g.mr.get_all_scores(u0, None).unwrap(),
      g.mr.get_all_scores(u3, None).unwrap(),
    )
  };
  assert_eq!(run(11), run(11));
  assert_ne!(run(11), run(12));
}

// ---------------------------------------------------------------------------
// Phase 3: barrier, context seeding, batch calculation
// ---------------------------------------------------------------------------

/// A context created implicitly by a write (not by `WriteCreateContext`) holds exactly its own
/// writes (D14: no seeding from the null context), and `mr_sync` covers it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn implicit_context_is_seeded_and_synced() {
  let proc = MultiGraphProcessor::new(settings(100));
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "", "U2", "U3", 2.0).await;
  write(&proc, "Q", "B1", "U1", 1.0).await;
  sync(&proc, 1).await;

  let q = edges(&proc, "Q").await;
  assert_eq!(q.len(), 1, "Q must hold only its own edge: {:?}", q);
  assert!(weight_of(&q, "B1", "U1").is_some(), "Q lost its own edge: {:?}", q);
}

/// Concurrent writes to the null context while a context is being created: once synced, the new
/// context is empty and the null context has every edge.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn context_created_during_writes_agrees_with_null_context() {
  for trial in 0..20 {
    let proc = Arc::new(MultiGraphProcessor::new(settings(10)));
    let mut js = JoinSet::new();
    for i in 0..32 {
      let p = Arc::clone(&proc);
      js.spawn(async move {
        write(&p, "", &format!("U{}", i % 7), &format!("U{}", (i + 1) % 7), 1.0 + i as f64)
          .await
      });
    }
    let p = Arc::clone(&proc);
    js.spawn(async move {
      request(&p, "N", ReqData::WriteCreateContext).await;
    });
    while let Some(r) = js.join_next().await {
      r.unwrap();
    }
    sync(&proc, 1).await;
    assert_eq!(edges(&proc, "").await.len(), 7, "trial {trial}");
    assert!(edges(&proc, "N").await.is_empty(), "trial {trial}: N inherited edges");
  }
}

/// The first mutual-scores read already returns reverse scores: the peers are calculated in one
/// batch before the read.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mutual_scores_have_reverse_scores_on_first_read() {
  let proc = MultiGraphProcessor::new(settings(2_000));
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "", "U2", "U1", 1.0).await;
  write(&proc, "", "U1", "U3", 1.0).await;
  write(&proc, "", "U3", "U1", 1.0).await;
  sync(&proc, 1).await;
  match request(
    &proc,
    "",
    ReqData::ReadMutualScores(OpReadMutualScores {
      ego: "U1".into(),
    }),
  )
  .await
  {
    Response::Scores(ResScores { scores }) => {
      assert_eq!(scores.iter().filter(|s| s.target != "U1").count(), 2);
      for s in scores.iter().filter(|s| s.target != "U1") {
        assert!(s.reverse_score > 0.0, "no reverse score for {}", s.target);
      }
    },
    other => panic!("expected scores, got {:?}", other),
  }
}

// ---------------------------------------------------------------------------
// Phase 4: residency (own LRU with pins), two-phase reads
// ---------------------------------------------------------------------------

fn cache_settings(
  num_walks: usize,
  capacity: usize,
) -> Settings {
  Settings {
    walks_cache_size: capacity,
    ..settings(num_walks)
  }
}

fn mutual(
  resp: Response,
) -> Vec<meritrank_service::data::ScoreResult> {
  match resp {
    Response::Scores(ResScores { scores }) => scores,
    other => panic!("expected scores, got {:?}", other),
  }
}

fn null_ctx(proc: &MultiGraphProcessor) -> dashmap::mapref::one::Ref<'_, String, GraphProcessor> {
  proc.subgraphs_map.get("").unwrap()
}

/// More peers than the walk cache holds: mutual scores are read in portions and every row still
/// gets its reverse score; afterwards the residency is back within capacity (+ the pinned ego).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mutual_scores_in_portions_beyond_capacity() {
  let capacity = 3;
  let proc = MultiGraphProcessor::new(cache_settings(500, capacity));
  for i in 1..=9 {
    write(&proc, "", "U0", &format!("U{i}"), 1.0).await;
    write(&proc, "", &format!("U{i}"), "U0", 1.0).await;
  }
  sync(&proc, 1).await;
  let rows = mutual(
    request(
      &proc,
      "",
      ReqData::ReadMutualScores(OpReadMutualScores {
        ego: "U0".into(),
      }),
    )
    .await,
  );
  let peers: Vec<_> = rows.iter().filter(|r| r.target != "U0").collect();
  assert_eq!(peers.len(), 9, "rows: {:?}", rows.iter().map(|r| &r.target).collect::<Vec<_>>());
  for r in peers {
    assert!(r.reverse_score > 0.0, "no reverse score for {}", r.target);
  }
  assert!(null_ctx(&proc).residency.len() <= capacity + 1);
}

/// Reading many egos one after another keeps the walk storage bounded by the capacity: evicted
/// egos release their blocks and later egos reuse them.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn walk_memory_bounded_by_capacity() {
  let (w, capacity) = (200, 4);
  let proc = MultiGraphProcessor::new(cache_settings(w, capacity));
  for i in 0..30 {
    write(&proc, "", &format!("U{i}"), &format!("U{}", (i + 1) % 30), 1.0).await;
  }
  sync(&proc, 1).await;
  for i in 0..30 {
    request(
      &proc,
      "",
      ReqData::ReadScores(meritrank_service::data::OpReadScores {
        ego:           format!("U{i}"),
        score_options: Default::default(),
      }),
    )
    .await;
  }
  sync(&proc, 2).await;
  let allocated = null_ctx(&proc).read(|g| g.mr.allocated_walks());
  assert!(
    allocated <= (capacity + 2) * w,
    "walk storage grew to {} slots for capacity {}",
    allocated,
    capacity
  );
  // The last read may leave capacity + 1 (its ego + a full portion) until the next plan.
  assert!(null_ctx(&proc).residency.len() <= capacity + 1);
}

/// A peer evicted from the cache is recalculated when a reverse score needs it, so the reverse
/// score reflects a change made while it was evicted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reverse_score_fresh_after_eviction() {
  let proc = MultiGraphProcessor::new(cache_settings(1_000, 2));
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "", "U2", "U1", 1.0).await;
  // U3..U7: a separate ring that never reaches U1 or U2; reading them evicts U2.
  for i in 3..8 {
    write(&proc, "", &format!("U{i}"), &format!("U{}", 3 + (i - 2) % 5), 1.0).await;
  }
  sync(&proc, 1).await;
  let read_mutual = || ReqData::ReadMutualScores(OpReadMutualScores {
    ego: "U1".into(),
  });
  let before = mutual(request(&proc, "", read_mutual()).await);
  assert!(before.iter().any(|r| r.target == "U2" && r.reverse_score > 0.0));

  // Push U2 out of the cache.
  for i in 3..8 {
    request(
      &proc,
      "",
      ReqData::ReadScores(meritrank_service::data::OpReadScores {
        ego:           format!("U{i}"),
        score_options: Default::default(),
      }),
    )
    .await;
  }
  assert!(
    !null_ctx(&proc).residency.is_resident(null_ctx(&proc).read(|g| g.nodes.get_by_name("U2").unwrap().id)),
    "setup: U2 should have been evicted"
  );
  write(&proc, "", "U2", "U1", 0.0).await;
  write(&proc, "", "U2", "U3", 1.0).await;
  sync(&proc, 2).await;

  let after = mutual(request(&proc, "", read_mutual()).await);
  let u2 = after.iter().find(|r| r.target == "U2").expect("U2 row");
  assert_eq!(u2.reverse_score, 0.0, "stale reverse score after eviction");
}

/// Many concurrent first reads of one ego (which also pull in its peers' frames for reverse
/// scores) calculate every frame once and all see scores.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_first_reads_calculate_once() {
  let w = 300;
  let proc = Arc::new(MultiGraphProcessor::new(cache_settings(w, 10)));
  for i in 0..10 {
    write(&proc, "", &format!("U{i}"), &format!("U{}", (i + 1) % 10), 1.0).await;
  }
  sync(&proc, 1).await;
  let mut js = JoinSet::new();
  for _ in 0..32 {
    let p = Arc::clone(&proc);
    js.spawn(async move {
      mutual(
        request(
          &p,
          "",
          ReqData::ReadScores(meritrank_service::data::OpReadScores {
            ego:           "U0".into(),
            score_options: Default::default(),
          }),
        )
        .await,
      )
      .len()
    });
  }
  while let Some(n) = js.join_next().await {
    assert!(n.unwrap() > 0, "a concurrent first read saw no scores");
  }
  // Each resident ego holds exactly one block: concurrent requests did not calculate twice.
  let resident = null_ctx(&proc).residency.len();
  assert_eq!(null_ctx(&proc).read(|g| g.mr.allocated_walks()), resident * w);
}

// ---------------------------------------------------------------------------
// Review fixes (codex GPT-6 Astra review of the track and the walls feature)
// ---------------------------------------------------------------------------

/// A panic while applying an operation does not kill the subgraph worker: later operations are
/// still applied and published. (A NaN weight is rejected at the service entry; sent straight
/// into a processor queue it reaches the core's panic.)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panicking_operation_does_not_kill_the_worker() {
  let notify = Arc::new(Notify::new());
  let proc = GraphProcessor::new(AugGraph::new(settings(10)), 8, 1, Arc::clone(&notify), None, 0);
  proc
    .op_sender
    .send(AugGraphOp::WriteEdge(edge("U1", "U2", f64::NAN)))
    .await
    .unwrap();
  proc.op_sender.send(AugGraphOp::WriteEdge(edge("U1", "U3", 1.0))).await.unwrap();
  let mut stamp = 0;
  flush(&proc, &notify, &mut stamp).await;
  assert_eq!(published_weight(&proc, "U1", "U3"), Some(1.0));
  proc.shutdown().ok();
}

/// Two concurrent bulk loads replace the state one after the other: the result is one of them,
/// never their union.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_bulk_loads_do_not_merge() {
  use meritrank_service::data::{BulkEdge, OpWriteBulkEdges};
  let bulk = |edges: &[(&str, &str)]| {
    ReqData::WriteBulkEdges(OpWriteBulkEdges {
      edges: edges
        .iter()
        .map(|(s, d)| BulkEdge {
          src:       (*s).into(),
          dst:       (*d).into(),
          amount:    1.0,
          magnitude: 0,
          context:   String::new(),
        })
        .collect(),
    })
  };
  for _ in 0..20 {
    let proc = Arc::new(MultiGraphProcessor::new(settings(10)));
    let a: Vec<(&str, &str)> = vec![("U1", "U2"), ("U2", "U3")];
    let b: Vec<(&str, &str)> = vec![("U7", "U8"), ("U8", "U9")];
    let (p1, p2) = (Arc::clone(&proc), Arc::clone(&proc));
    let (ra, rb) = (bulk(&a), bulk(&b));
    let t1 = tokio::spawn(async move { request(&p1, "", ra).await });
    let t2 = tokio::spawn(async move { request(&p2, "", rb).await });
    let (r1, r2) = (t1.await.unwrap(), t2.await.unwrap());
    assert!(matches!(r1, Response::Ok) && matches!(r2, Response::Ok), "{:?} {:?}", r1, r2);
    sync(&proc, 1).await;
    let now: Vec<(String, String)> =
      edges(&proc, "").await.into_iter().map(|(s, d, _)| (s, d)).collect();
    let as_pairs = |v: &[(&str, &str)]| {
      v.iter().map(|(s, d)| (s.to_string(), d.to_string())).collect::<Vec<_>>()
    };
    let mut sorted = now.clone();
    sorted.sort();
    assert!(sorted == as_pairs(&a) || sorted == as_pairs(&b), "merged state: {:?}", sorted);
  }
}

/// D14: no owners — a former object's reverse score is the ego's score in the object's own
/// frame; with more such peers than the walk cache, every row still gets it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn object_reverse_scores_survive_portions() {
  let proc = MultiGraphProcessor::new(cache_settings(2_000, 1));
  for (o, u) in [("B1", "U1"), ("B2", "U2"), ("B3", "U3")] {
    write(&proc, "", o, u, 1.0).await; // object → user
    write(&proc, "", "U0", o, 1.0).await; // the ego likes the object
    write(&proc, "", u, "U0", 1.0).await; // the user trusts the ego
  }
  sync(&proc, 1).await;
  let rows = mutual(
    request(
      &proc,
      "",
      ReqData::ReadScores(meritrank_service::data::OpReadScores {
        ego:           "U0".into(),
        score_options: Default::default(),
      }),
    )
    .await,
  );
  for o in ["B1", "B2", "B3"] {
    let r = rows.iter().find(|r| r.target == o).unwrap_or_else(|| panic!("{o} row: {:?}", rows));
    // Bx → Ux → U0: the ego is reached from the object's own frame.
    assert!(r.reverse_score > 0.0, "{o}: reverse score lost: {:?}", r);
  }
}

/// Reads cancelled while their frames are being calculated leave no pins behind: the cache can
/// still evict back to its capacity.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelled_reads_leave_no_pins() {
  let capacity = 2;
  let proc = Arc::new(MultiGraphProcessor::new(cache_settings(20_000, capacity)));
  for i in 0..12 {
    write(&proc, "", &format!("U{i}"), &format!("U{}", (i + 1) % 12), 1.0).await;
  }
  sync(&proc, 1).await;
  for i in 0..12 {
    let p = Arc::clone(&proc);
    let read = async move {
      request(
        &p,
        "",
        ReqData::ReadMutualScores(OpReadMutualScores {
          ego: format!("U{i}"),
        }),
      )
      .await
    };
    let _ = timeout(Duration::from_micros(300), read).await; // cancelled mid-flight
  }
  sync(&proc, 2).await;
  // A normal read plans again: with no leaked pins, the cache returns to its capacity.
  request(
    &proc,
    "",
    ReqData::ReadScores(meritrank_service::data::OpReadScores {
      ego:           "U0".into(),
      score_options: Default::default(),
    }),
  )
  .await;
  let len = null_ctx(&proc).residency.len();
  assert!(len <= capacity + 1, "{len} frames resident after cancelled reads (capacity {capacity})");
}
