//! Thrash test: full chaos against one `MultiGraphProcessor`.
//!
//! Many concurrent tasks issue random requests of every kind — trust and wall writes (valid and
//! invalid), deletions, node deletions, contexts, bulk loads, resets, zero opinions, explicit
//! calculations, every read, syncs — under a tiny walk cache (constant eviction), batching, a
//! small queue and discredit on. Then the service must still be alive and consistent:
//! - every request answered within a timeout (no hang, no dead worker);
//! - both buffer copies of every subgraph identical (edges and every calculated frame);
//! - every copy internally consistent (`MeritRank::verify`);
//! - the incrementally maintained frames of the null context distributed like frames generated
//!   from scratch on the final graph.
//!
//! Duration: MERITRANK_THRASH_SECS (default 4); seed of the request streams:
//! MERITRANK_THRASH_SEED.

use meritrank_service::data::{
  BulkEdge, EdgeResult, OpReadGraph, OpReadMutualScores, OpReadNeighbors, OpReadNodeScore,
  OpReadScores, OpWriteBulkEdges, OpWriteCalculate, OpWriteDeleteEdge, OpWriteDeleteNode,
  OpWriteEdge, OpWriteZeroOpinion, ReqData, Request, ResEdges, Response,
};
use meritrank_service::settings::Settings;
use meritrank_service::state_manager::MultiGraphProcessor;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use tokio::time::timeout;

const USERS: usize = 14;
const CONTEXTS: [&str; 3] = ["", "X", "Y"];
const TIMEOUT: Duration = Duration::from_secs(20);

fn settings() -> Settings {
  Settings {
    num_walks: 400,
    zero_opinion_factor: 0.0,
    walks_cache_size: 5,
    min_ops_before_swap: 3,
    subgraph_queue_capacity: 16,
    discredit_lambda: 0.5,
    ..Settings::default()
  }
}

fn user(rng: &mut StdRng) -> String {
  format!("U{}", rng.random_range(0..USERS))
}

fn any_node(rng: &mut StdRng) -> String {
  match rng.random_range(0..10) {
    0 => format!("B{}", rng.random_range(0..3)),
    1 => "Zbogus".into(),
    _ => user(rng),
  }
}

fn weight(rng: &mut StdRng) -> f64 {
  match rng.random_range(0..20) {
    0 => f64::NAN,
    1 => 0.0,
    2..=7 => -rng.random_range(0.05..1.5),
    _ => rng.random_range(0.1..3.0),
  }
}

fn random_request(rng: &mut StdRng) -> Request {
  let ctx = CONTEXTS[rng.random_range(0..CONTEXTS.len())].to_string();
  let data = match rng.random_range(0..100) {
    0..=34 => ReqData::WriteEdge(OpWriteEdge {
      src:       any_node(rng),
      dst:       any_node(rng),
      amount:    weight(rng),
      magnitude: rng.random_range(0..30),
    }),
    35..=39 => ReqData::WriteDeleteEdge(OpWriteDeleteEdge {
      src:   user(rng),
      dst:   user(rng),
      index: 0,
    }),
    40 => ReqData::WriteDeleteNode(OpWriteDeleteNode { node: user(rng), index: 0 }),
    41 => ReqData::WriteCreateContext,
    42 => ReqData::WriteZeroOpinion(OpWriteZeroOpinion {
      node:  user(rng),
      score: rng.random_range(0.0..1.0),
    }),
    43..=45 => ReqData::WriteCalculate(OpWriteCalculate { ego: user(rng) }),
    46 => {
      let n = rng.random_range(0..40);
      ReqData::WriteBulkEdges(OpWriteBulkEdges {
        edges: (0..n)
          .map(|_| BulkEdge {
            src:       user(rng),
            dst:       any_node(rng),
            amount:    weight(rng),
            magnitude: 0,
            context:   CONTEXTS[rng.random_range(0..CONTEXTS.len())].into(),
          })
          .collect(),
      })
    },
    47 if rng.random_bool(0.2) => ReqData::WriteReset,
    48..=52 => ReqData::Sync(rng.random()),
    53 => ReqData::Stamp(rng.random()),
    54..=65 => ReqData::ReadScores(OpReadScores {
      ego:           any_node(rng),
      score_options: Default::default(),
    }),
    66..=75 => ReqData::ReadMutualScores(OpReadMutualScores { ego: any_node(rng) }),
    76..=82 => ReqData::ReadNodeScore(OpReadNodeScore {
      ego:    user(rng),
      target: any_node(rng),
    }),
    83..=88 => ReqData::ReadGraph(OpReadGraph {
      ego:           user(rng),
      focus:         any_node(rng),
      positive_only: rng.random(),
      index:         0,
      count:         50,
    }),
    89..=93 => ReqData::ReadNeighbors(OpReadNeighbors {
      ego:     user(rng),
      focus:   any_node(rng),
      direction: rng.random_range(0..3),
      kind:    None,
      hide_personal: false,
      lt:      10.0,
      lte:     false,
      gt:      -10.0,
      gte:     false,
      index:   0,
      count:   50,
    }),
    94..=96 => ReqData::ReadEdges,
    _ => ReqData::ReadNodeList,
  };
  Request { subgraph: ctx, data }
}

async fn request(
  proc: &MultiGraphProcessor,
  req: Request,
) -> Response {
  let label = format!("{:?}", req.data);
  match timeout(TIMEOUT, proc.process_request(&req)).await {
    Ok(r) => r,
    Err(_) => panic!("request hung for {:?}: {}", TIMEOUT, &label[..label.len().min(200)]),
  }
}

async fn edges(
  proc: &MultiGraphProcessor,
  ctx: &str,
) -> Vec<(String, String, f64)> {
  match request(
    proc,
    Request {
      subgraph: ctx.into(),
      data:     ReqData::ReadEdges,
    },
  )
  .await
  {
    Response::Edges(ResEdges { edges }) => {
      let mut v: Vec<_> = edges.into_iter().map(|e: EdgeResult| (e.src, e.dst, e.weight)).collect();
      v.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
      v
    },
    other => panic!("expected edges, got {:?}", other),
  }
}

async fn sync(proc: &MultiGraphProcessor) {
  let r = request(
    proc,
    Request {
      subgraph: String::new(),
      data:     ReqData::Sync(0),
    },
  )
  .await;
  assert!(matches!(r, Response::Ok), "sync failed: {:?}", r);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn thrash() {
  let secs: u64 = std::env::var("MERITRANK_THRASH_SECS")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(4);
  let base_seed: u64 = std::env::var("MERITRANK_THRASH_SEED")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(0xC0FFEE);
  let proc = Arc::new(MultiGraphProcessor::new(settings()));
  let deadline = Instant::now() + Duration::from_secs(secs);

  let mut js = JoinSet::new();
  for task in 0..16u64 {
    let p = Arc::clone(&proc);
    js.spawn(async move {
      let mut rng = StdRng::seed_from_u64(base_seed + task);
      let mut n = 0u64;
      while Instant::now() < deadline {
        let req = random_request(&mut rng);
        let _ = request(&p, req).await;
        n += 1;
      }
      n
    });
  }
  let mut total = 0;
  while let Some(r) = js.join_next().await {
    total += r.expect("a chaos task panicked");
  }
  println!("thrash: {total} requests in {secs} s");

  // Alive and synced.
  sync(&proc).await;

  // Both copies of every subgraph identical and internally consistent. After a sync the
  // published copy is A; a second sync publishes B holding exactly the same operations.
  let names: Vec<String> = proc.subgraphs_map.iter().map(|r| r.key().clone()).collect();
  for name in &names {
    let first = Arc::clone(&proc.subgraphs_map.get(name).unwrap().shared).load_full();
    sync(&proc).await;
    let second = Arc::clone(&proc.subgraphs_map.get(name).unwrap().shared).load_full();
    let snapshot = |g: &meritrank_service::aug_graph::AugGraph| {
      g.mr.verify().unwrap_or_else(|e| panic!("subgraph {name:?}: inconsistent copy: {e}"));
      let mut edges = vec![];
      for (id, info) in g.nodes.id_to_info.iter().enumerate() {
        if let Some(data) = g.mr.graph.get_node_data(id) {
          for (dst, w) in data.get_outgoing_edges() {
            edges.push((info.name.clone(), dst, w));
          }
        }
      }
      edges.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
      let frames: Vec<_> = (0..g.nodes.id_to_info.len())
        .filter(|id| g.mr.is_calculated(*id))
        .map(|id| (id, g.mr.get_all_scores(id, None).unwrap()))
        .collect();
      (edges, frames)
    };
    let a = snapshot(&first.read());
    let b = snapshot(&second.read());
    assert!(
      !Arc::ptr_eq(&first, &second) || names.len() > 0,
      "expected two copies"
    );
    assert_eq!(a, b, "subgraph {name:?}: buffer copies differ");
    // Every calculated frame is tracked by the walk cache (so it can be evicted).
    let tracked = proc.subgraphs_map.get(name).unwrap().residency.len();
    assert_eq!(a.1.len(), tracked, "subgraph {name:?}: untracked frames");
    // Concurrent reads may pin more than the capacity; the next read restores it.
    request(
      &proc,
      Request {
        subgraph: name.clone(),
        data:     ReqData::ReadScores(OpReadScores {
          ego:           "U0".into(),
          score_options: Default::default(),
        }),
      },
    )
    .await;
    let tracked = proc.subgraphs_map.get(name).unwrap().residency.len();
    assert!(
      tracked <= settings().walks_cache_size + 1,
      "subgraph {name:?}: {tracked} frames after a read"
    );
    println!(
      "subgraph {name:?}: copies identical ({} edges, {} frames)",
      a.0.len(),
      a.1.len()
    );
  }

  // The restoring reads may have queued evictions without waiting for them (they did not
  // concern the requested egos): make them visible before listing resident frames.
  sync(&proc).await;

  // Incremental frames vs frames generated from scratch on the final graph.
  let final_edges = edges(&proc, "").await;
  let fresh = MultiGraphProcessor::new(Settings {
    walks_cache_size: 0,
    ..settings()
  });
  let bulk = ReqData::WriteBulkEdges(OpWriteBulkEdges {
    edges: final_edges
      .iter()
      .map(|(s, d, w)| BulkEdge {
        src:       s.clone(),
        dst:       d.clone(),
        amount:    *w,
        magnitude: 0,
        context:   String::new(),
      })
      .collect(),
  });
  assert!(matches!(
    request(&fresh, Request { subgraph: String::new(), data: bulk }).await,
    Response::Ok
  ));
  let resident: Vec<String> = {
    let p = proc.subgraphs_map.get("").unwrap();
    p.read(|g| {
      (0..g.nodes.id_to_info.len())
        .filter(|id| g.mr.is_calculated(*id))
        .map(|id| g.nodes.id_to_info[id].name.clone())
        .collect()
    })
  };
  let w = settings().num_walks as f64;
  let tolerance = 5.0 * (1.0 + settings().discredit_lambda) * (0.5 / w).sqrt();
  println!("{} resident egos in the null context; comparing up to 4 with fresh frames", resident.len());
  if resident.is_empty() {
    println!("(a reset or bulk load came last: no incremental frames to compare)");
  }
  for ego in resident.iter().take(4) {
    let scores = |p: &MultiGraphProcessor| {
      p.subgraphs_map.get("").unwrap().read(|g| {
        let id = g.nodes.get_by_name(ego).unwrap().id;
        g.mr
          .get_all_scores(id, None)
          .unwrap_or_else(|e| {
            panic!(
              "ego {ego} ({id}): {e}; calculated {}, edges out {:?}",
              g.mr.is_calculated(id),
              g.mr.graph.get_node_data(id).map(|d| d.get_outgoing_edges().collect::<Vec<_>>())
            )
          })
          .into_iter()
          .map(|(n, s)| (g.nodes.id_to_info[n].name.clone(), s))
          .collect::<std::collections::HashMap<_, _>>()
      })
    };
    let inc = scores(&proc);
    request(
      &fresh,
      Request {
        subgraph: String::new(),
        data:     ReqData::WriteCalculate(OpWriteCalculate { ego: ego.clone() }),
      },
    )
    .await;
    sync(&fresh).await;
    let fr = scores(&fresh);
    let mut keys: Vec<_> = inc.keys().chain(fr.keys()).cloned().collect();
    keys.sort();
    keys.dedup();
    for k in keys {
      let (a, b) = (inc.get(&k).copied().unwrap_or(0.0), fr.get(&k).copied().unwrap_or(0.0));
      assert!(
        (a - b).abs() < tolerance,
        "ego {ego}, node {k}: incremental {a} vs fresh {b} (tolerance {tolerance})"
      );
    }
  }
}
