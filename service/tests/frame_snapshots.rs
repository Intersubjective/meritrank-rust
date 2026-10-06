//! D14 part II: reverse-score snapshots (JOURNAL.md D14).
//!
//! A reverse score of a peer comes from its resident frame, else from a valid snapshot (the copy
//! of an evicted frame or an admitted on-demand sample), else from a fresh sample taken by the
//! read itself. Fresh frames are seeded per ego, so a sample equals what a calculation would give.
//!
//! - A1 equivalence: a small walk cache with snapshots answers exactly as an unbounded cache (and
//!   as a small cache without snapshots) when the on-demand walk count equals NUM_WALKS;
//! - A2 warm reads touch no peer frame and sample nothing;
//! - A3 strict invalidation: a write in a snapshot's footprint, a wall of its owner; zero opinion
//!   needs none; reset and bulk load clear; late admissions are validated; A3' the heuristic;
//! - A4 eviction keeps the content: no recalculation, no new revision;
//! - A5 concurrent reads equal a single-threaded replay at the (epoch, seq) they were read at, and
//!   the two buffer copies hold the same snapshots;
//! - resources: quota and admission budget never make an answer incomplete.

use std::sync::Arc;

use bincode::{config::standard, encode_to_vec};
use meritrank_core::NodeId;
use meritrank_service::aug_graph::{
  read_scope, AugGraph, ReverseSource, MUTATION_LOG_CAPACITY, STALENESS_DELTA,
};
use meritrank_service::data::{
  AdmitBatch, AugGraphOp, BulkEdge, OpReadGraph, OpReadMutualScores, OpReadNeighbors,
  OpReadNodeScore, OpReadScores, OpWriteBulkEdges, OpWriteEdge, OpWriteZeroOpinion, ReqData,
  Request, ResScores, Response, ScoreResult, NEIGHBORS_ALL,
};
use meritrank_service::settings::Settings;
use meritrank_service::state_manager::MultiGraphProcessor;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::task::JoinSet;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn settings(
  num_walks: usize,
  cache: usize,
  staleness: f64,
) -> Settings {
  Settings {
    num_walks,
    on_demand_num_walks: num_walks,
    walks_cache_size: cache,
    snapshot_staleness: staleness,
    zero_opinion_factor: 0.2,
    discredit_lambda: 1.0,
    seed: 0xD14,
    ..Settings::default()
  }
}

async fn request(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  data: ReqData,
) -> Response {
  proc.process_request(&Request { subgraph: subgraph.into(), data }).await
}

async fn write(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  src: &str,
  dst: &str,
  amount: f64,
) {
  let r = request(
    proc,
    subgraph,
    ReqData::WriteEdge(OpWriteEdge { src: src.into(), dst: dst.into(), amount, magnitude: 0 }),
  )
  .await;
  assert!(matches!(r, Response::Ok), "write {src}->{dst}: {:?}", r);
}

async fn zero_opinion(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  node: &str,
  score: f64,
) {
  request(
    proc,
    subgraph,
    ReqData::WriteZeroOpinion(OpWriteZeroOpinion { node: node.into(), score }),
  )
  .await;
}

async fn sync(proc: &MultiGraphProcessor) {
  assert!(matches!(request(proc, "", ReqData::Sync(0)).await, Response::Ok));
}

fn rows(r: Response) -> Vec<ScoreResult> {
  match r {
    Response::Scores(ResScores { scores }) => scores,
    other => panic!("expected scores, got {:?}", other),
  }
}

fn mutual(ego: &str) -> ReqData {
  ReqData::ReadMutualScores(OpReadMutualScores { ego: ego.into() })
}

/// Bytes of a response: equal bytes = equal scores, bit for bit.
fn bytes(r: &Response) -> Vec<u8> {
  encode_to_vec(r, standard()).unwrap()
}

fn id(
  g: &AugGraph,
  name: &str,
) -> NodeId {
  g.nodes.get_by_name(name).unwrap_or_else(|| panic!("no node {name}")).id
}

fn edge_op(
  src: &str,
  dst: &str,
  amount: f64,
) -> AugGraphOp {
  AugGraphOp::WriteEdge(OpWriteEdge { src: src.into(), dst: dst.into(), amount, magnitude: 0 })
}

/// Three clusters of eight nodes (each points at the next three of its cluster), a hub linked
/// both ways with two nodes of every cluster, walls, and zero opinions.
fn fixture_edges() -> Vec<(String, String, f64)> {
  let mut e = vec![];
  for c in 0..3 {
    for i in 0..8 {
      let n = c * 8 + i;
      for k in 1..=3 {
        let m = c * 8 + (i + k) % 8;
        e.push((format!("N{n}"), format!("N{m}"), 1.0 + ((n + k) % 4) as f64));
      }
    }
    for i in [0usize, 3] {
      e.push(("H".into(), format!("N{}", c * 8 + i), 1.0));
      e.push((format!("N{}", c * 8 + i), "H".into(), 2.0));
    }
  }
  e.push(("N1".into(), "N5".into(), -0.5));
  e.push(("H".into(), "N22".into(), -1.0));
  e
}

async fn load_fixture(
  proc: &MultiGraphProcessor,
  ctx: &str,
) {
  for (s, d, w) in fixture_edges() {
    write(proc, ctx, &s, &d, w).await;
  }
  zero_opinion(proc, ctx, "N2", 0.5).await;
  zero_opinion(proc, ctx, "H", 0.3).await;
  sync(proc).await;
}

fn reads() -> Vec<ReqData> {
  let mut v = vec![];
  for ego in ["H", "N0", "N9", "N17"] {
    v.push(mutual(ego));
    v.push(ReqData::ReadScores(OpReadScores {
      ego:           ego.into(),
      score_options: Default::default(),
    }));
    v.push(ReqData::ReadScores(OpReadScores {
      ego:           ego.into(),
      score_options: meritrank_service::data::FilterOptions {
        index: 5,
        count: 5,
        ..Default::default()
      },
    }));
    v.push(ReqData::ReadNodeScore(OpReadNodeScore { ego: ego.into(), target: "N12".into() }));
    v.push(ReqData::ReadNeighbors(OpReadNeighbors {
      ego:           ego.into(),
      focus:         "H".into(),
      direction:     NEIGHBORS_ALL,
      kind:          None,
      hide_personal: false,
      lt:            100.0,
      lte:           false,
      gt:            -100.0,
      gte:           false,
      index:         0,
      count:         100,
    }));
    v.push(ReqData::ReadGraph(OpReadGraph {
      ego:           ego.into(),
      focus:         "N12".into(),
      positive_only: false,
      index:         0,
      count:         100,
    }));
  }
  v
}

// ---------------------------------------------------------------------------
// A1: equivalence
// ---------------------------------------------------------------------------

/// With ON_DEMAND = NUM_WALKS and strict mode, a cache of 3 with snapshots, a cache of 3 without
/// them and an unbounded cache give byte-identical answers, cold and warm, in the null and in a
/// named context.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a1_small_cache_with_snapshots_equals_unbounded_cache() {
  let reference = MultiGraphProcessor::new(settings(300, 0, 0.0));
  let snap = MultiGraphProcessor::new(settings(300, 3, 0.0));
  let nosnap = MultiGraphProcessor::new(Settings { snapshots_mb: 0, ..settings(300, 3, 0.0) });
  for ctx in ["", "K"] {
    for p in [&reference, &snap, &nosnap] {
      load_fixture(p, ctx).await;
    }
    for pass in 0..2 {
      for (i, r) in reads().into_iter().enumerate() {
        let a = request(&reference, ctx, r.clone()).await;
        let b = request(&snap, ctx, r.clone()).await;
        let c = request(&nosnap, ctx, r.clone()).await;
        assert!(!matches!(a, Response::Fail), "ctx {ctx:?} read {i}");
        assert_eq!(bytes(&a), bytes(&b), "ctx {ctx:?} pass {pass} read {i}: snapshots differ\n{a:?}\n{b:?}");
        assert_eq!(bytes(&a), bytes(&c), "ctx {ctx:?} pass {pass} read {i}: no-snapshot differs");
        sync(&snap).await; // publish admissions between reads
      }
    }
  }
  let stats = snap.snapshot_stats("").unwrap();
  assert!(stats.reverse_from_snapshot > 0, "{stats:?}");
  assert!(stats.reverse_sampled > 0, "{stats:?}");
}

/// With fewer on-demand walks the reverse scores are estimates of the same quantities: they agree
/// with the unbounded cache within sampling error (clusters are not compared).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a1_fewer_on_demand_walks_agree_within_sampling_error() {
  let (w, od) = (4_000, 400);
  let reference = MultiGraphProcessor::new(settings(w, 0, 0.0));
  let snap =
    MultiGraphProcessor::new(Settings { on_demand_num_walks: od, ..settings(w, 3, 0.0) });
  load_fixture(&reference, "").await;
  load_fixture(&snap, "").await;
  let a = rows(request(&reference, "", mutual("H")).await);
  let b = rows(request(&snap, "", mutual("H")).await);
  assert_eq!(a.len(), b.len());
  for (x, y) in a.iter().zip(&b) {
    assert_eq!(x.target, y.target);
    assert_eq!(x.score.to_bits(), y.score.to_bits(), "forward scores come from the ego's frame");
    // Per-walk contributions lie in [−λ, 1] (λ = 1) and the zero opinion scales them by 0.8:
    // the difference of two means has sd ≤ 0.8·2·0.5·sqrt(1/W + 1/n).
    let sd = 0.8 * 2.0 * 0.5 * (1.0 / w as f64 + 1.0 / od as f64).sqrt();
    assert!(
      (x.reverse_score - y.reverse_score).abs() <= 5.0 * sd,
      "{}: {} vs {}",
      x.target,
      x.reverse_score,
      y.reverse_score
    );
  }
}

// ---------------------------------------------------------------------------
// A2: warm reads
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a2_warm_read_touches_no_peer_frames() {
  let proc = MultiGraphProcessor::new(settings(200, 2, 0.0));
  for i in 0..10 {
    write(&proc, "", "H", &format!("P{i}"), 1.0).await;
    write(&proc, "", &format!("P{i}"), "H", 1.0).await;
  }
  sync(&proc).await;
  let cold = rows(request(&proc, "", mutual("H")).await);
  assert_eq!(cold.iter().filter(|r| r.target != "H").count(), 10);
  sync(&proc).await; // the cold read's samples are admitted and published

  let before = proc.snapshot_stats("").unwrap();
  assert_eq!(before.snapshot_count, 10, "{before:?}");
  for _ in 0..6 {
    let warm = rows(request(&proc, "", mutual("H")).await);
    assert_eq!(warm.len(), cold.len());
  }
  let after = proc.snapshot_stats("").unwrap();
  assert_eq!(after.reverse_sampled, before.reverse_sampled, "warm reads sampled");
  assert_eq!(after.graph.frames_calculated, before.graph.frames_calculated, "warm reads calculated");
  assert!(after.reverse_from_snapshot >= before.reverse_from_snapshot + 60);

  // The read itself, run on the published copy: it touches the ego's frame only.
  let (frames, sampled, ego) = proc
    .read_subgraph("", |g| {
      let (_, report) = read_scope(true, || {
        g.read_mutual_scores(OpReadMutualScores { ego: "H".into() })
      });
      (report.frames, report.sampled.len(), id(g, "H"))
    })
    .unwrap();
  assert_eq!(frames, vec![ego]);
  assert_eq!(sampled, 0);
  let resident = proc.subgraphs_map.get("").unwrap().residency.len();
  assert!(resident <= 3, "{resident} frames resident with capacity 2");
}

// ---------------------------------------------------------------------------
// A3: strict invalidation
// ---------------------------------------------------------------------------

/// E likes A and B; A → C and B → D are separate branches, so footprint(A) = {A, C} and
/// footprint(B) = {B, D}. Returns the processor after a cold and a published read.
async fn branches(staleness: f64) -> MultiGraphProcessor {
  let proc = MultiGraphProcessor::new(settings(300, 1, staleness));
  write(&proc, "", "E", "A", 1.0).await;
  write(&proc, "", "E", "B", 1.0).await;
  write(&proc, "", "A", "C", 1.0).await;
  write(&proc, "", "B", "D", 1.0).await;
  sync(&proc).await;
  rows(request(&proc, "", mutual("E")).await);
  sync(&proc).await;
  let s = proc.snapshot_stats("").unwrap();
  assert_eq!(s.snapshot_count, 4, "A, B, C, D: {s:?}");
  proc
}

fn snapshot_egos(
  proc: &MultiGraphProcessor,
) -> Vec<String> {
  proc
    .read_subgraph("", |g| {
      let mut v: Vec<String> = g
        .snapshots
        .egos()
        .into_iter()
        .map(|e| g.nodes.get_by_id(e).unwrap().name.clone())
        .collect();
      v.sort();
      v
    })
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a3_write_in_footprint_invalidates_only_those_snapshots() {
  let proc = branches(0.0).await;
  let before = rows(request(&proc, "", mutual("E")).await);
  let inv0 = proc.snapshot_stats("").unwrap().graph.snapshots_invalidated;

  write(&proc, "", "C", "X", 1.0).await; // C is in footprint(A) and footprint(C)
  sync(&proc).await;
  assert_eq!(snapshot_egos(&proc), vec!["B", "D"]);
  assert_eq!(proc.snapshot_stats("").unwrap().graph.snapshots_invalidated, inv0 + 2);

  let sampled0 = proc.snapshot_stats("").unwrap().reverse_sampled;
  let after = rows(request(&proc, "", mutual("E")).await);
  assert_eq!(proc.snapshot_stats("").unwrap().reverse_sampled, sampled0 + 2, "A and C resampled");
  // B's and D's rows are untouched; A's reverse score is that of a fresh sample of A now.
  for t in ["B", "D"] {
    let x = before.iter().find(|r| r.target == t).unwrap();
    let y = after.iter().find(|r| r.target == t).unwrap();
    assert_eq!(x.reverse_score.to_bits(), y.reverse_score.to_bits(), "{t}");
  }
  let a_row = after.iter().find(|r| r.target == "A").unwrap();
  let expected = proc
    .read_subgraph("", |g| {
      let (a, e) = (id(g, "A"), id(g, "E"));
      let s = g.fresh_sample(a).unwrap();
      g.with_zero_opinion(e, s.score(e, g.settings.discredit_lambda, g.settings.blame_decay))
    })
    .unwrap();
  assert_eq!(a_row.reverse_score.to_bits(), expected.to_bits());

  // Once admitted, the new snapshot of A sees the new edge.
  sync(&proc).await;
  let sees_x = proc
    .read_subgraph("", |g| g.snapshots.get(id(g, "A")).unwrap().in_footprint(id(g, "X")))
    .unwrap();
  assert!(sees_x, "A's new snapshot must include X");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a3_wall_invalidates_only_its_owner() {
  let proc = branches(0.0).await;
  write(&proc, "", "B", "Z", -0.5).await; // a wall of B on a new node
  sync(&proc).await;
  assert_eq!(snapshot_egos(&proc), vec!["A", "C", "D"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a3_zero_opinion_needs_no_invalidation() {
  let proc = branches(0.0).await;
  let before = rows(request(&proc, "", mutual("E")).await);
  let s0 = proc.snapshot_stats("").unwrap();
  zero_opinion(&proc, "", "E", 0.9).await;
  sync(&proc).await;
  let after = rows(request(&proc, "", mutual("E")).await);
  let s1 = proc.snapshot_stats("").unwrap();
  assert_eq!(s1.graph.snapshots_invalidated, s0.graph.snapshots_invalidated);
  assert_eq!(s1.reverse_sampled, s0.reverse_sampled);
  for t in ["A", "B", "C", "D"] {
    let x = before.iter().find(|r| r.target == t).unwrap();
    let y = after.iter().find(|r| r.target == t).unwrap();
    // reverse = 0.8·raw + 0.2·z(E): the zero opinion of E moved from 0 to 0.9.
    assert!((y.reverse_score - x.reverse_score - 0.2 * 0.9).abs() < 1e-12, "{t}: {x:?} {y:?}");
  }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a3_reset_and_bulk_load_clear_snapshots() {
  let proc = branches(0.0).await;
  request(&proc, "", ReqData::WriteReset).await;
  sync(&proc).await;
  assert_eq!(proc.snapshot_stats("").unwrap().snapshot_count, 0);

  let proc = branches(0.0).await;
  let r = request(
    &proc,
    "",
    ReqData::WriteBulkEdges(OpWriteBulkEdges {
      edges: vec![BulkEdge {
        src:       "E".into(),
        dst:       "A".into(),
        amount:    1.0,
        magnitude: 0,
        context:   String::new(),
      }],
    }),
  )
  .await;
  assert!(matches!(r, Response::Ok));
  assert_eq!(proc.snapshot_stats("").unwrap().snapshot_count, 0);
}

/// Admission validates a sample against everything applied since it was taken: a write in its
/// footprint, a wall of its owner, an epoch change, a truncated log, a resident frame.
#[test]
fn a3_late_admission_is_validated() {
  let s = settings(200, 2, 0.0);
  let build = || {
    let mut g = AugGraph::with_stream(s.clone(), "");
    let mut seq = 0;
    for (a, b) in [("E", "A"), ("A", "C"), ("B", "D"), ("D", "B")] {
      seq += 1;
      g.apply_seq_op(seq, &edge_op(a, b, 1.0));
    }
    (g, seq)
  };
  let admit = |g: &mut AugGraph, seq: &mut u64, base: u64, epoch: u64, who: &str| {
    let sample = g.fresh_sample(id(g, who)).unwrap();
    *seq += 1;
    g.apply_seq_op(
      *seq,
      &AugGraphOp::AdmitSnapshots(AdmitBatch {
        epoch,
        base_seq: base,
        samples: Arc::new(vec![sample]),
      }),
    );
  };

  // Accepted: nothing touched A's footprint (B → D is outside it).
  let (mut g, mut seq) = build();
  let base = seq;
  let sample = g.fresh_sample(id(&g, "A")).unwrap();
  seq += 1;
  g.apply_seq_op(seq, &edge_op("B", "D", 2.0));
  seq += 1;
  let epoch = g.epoch;
  g.apply_seq_op(
    seq,
    &AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: base, samples: Arc::new(vec![sample]) }),
  );
  assert!(g.snapshots.contains(id(&g, "A")));
  assert_eq!(g.counters.snapshots_admitted, 1);

  // Rejected: a write in A's footprint after the sample was taken.
  let (mut g, mut seq) = build();
  let base = seq;
  let sample = g.fresh_sample(id(&g, "A")).unwrap();
  seq += 1;
  g.apply_seq_op(seq, &edge_op("C", "X", 1.0));
  seq += 1;
  let epoch = g.epoch;
  g.apply_seq_op(
    seq,
    &AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: base, samples: Arc::new(vec![sample]) }),
  );
  assert!(!g.snapshots.contains(id(&g, "A")));
  assert_eq!(g.counters.snapshots_rejected, 1);

  // Rejected: a wall of A.
  let (mut g, mut seq) = build();
  let base = seq;
  let sample = g.fresh_sample(id(&g, "A")).unwrap();
  seq += 1;
  g.apply_seq_op(seq, &edge_op("A", "Q", -0.3));
  seq += 1;
  let epoch = g.epoch;
  g.apply_seq_op(
    seq,
    &AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: base, samples: Arc::new(vec![sample]) }),
  );
  assert!(!g.snapshots.contains(id(&g, "A")));

  // Rejected: another epoch (a sample from a replaced processor).
  let (mut g, mut seq) = build();
  let base = seq;
  let epoch = g.epoch + 1;
  admit(&mut g, &mut seq, base, epoch, "A");
  assert!(!g.snapshots.contains(id(&g, "A")));

  // Rejected: the log no longer covers the sample's base.
  let (mut g, mut seq) = build();
  let base = seq;
  let sample = g.fresh_sample(id(&g, "A")).unwrap();
  for i in 0..=MUTATION_LOG_CAPACITY {
    seq += 1;
    g.apply_seq_op(seq, &edge_op("B", "D", 1.0 + (i % 7) as f64));
  }
  seq += 1;
  let epoch = g.epoch;
  g.apply_seq_op(
    seq,
    &AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: base, samples: Arc::new(vec![sample]) }),
  );
  assert!(!g.snapshots.contains(id(&g, "A")));

  // Rejected: A became resident meanwhile (its frame is authoritative).
  let (mut g, mut seq) = build();
  let base = seq;
  let sample = g.fresh_sample(id(&g, "A")).unwrap();
  seq += 1;
  let a = id(&g, "A");
  g.apply_seq_op(seq, &AugGraphOp::EnsureCalculated(vec![a]));
  seq += 1;
  let epoch = g.epoch;
  g.apply_seq_op(
    seq,
    &AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: base, samples: Arc::new(vec![sample]) }),
  );
  assert!(!g.snapshots.contains(a));
}

/// Mutations are logged even when no snapshot exists, so a sample in flight is still checked.
#[test]
fn a3_mutations_are_logged_without_snapshots() {
  let mut g = AugGraph::with_stream(settings(100, 2, 0.0), "");
  g.apply_seq_op(1, &edge_op("E", "A", 1.0));
  g.apply_seq_op(2, &edge_op("A", "C", 1.0));
  assert!(g.snapshots.is_empty());
  let sample = g.fresh_sample(id(&g, "A")).unwrap();
  g.apply_seq_op(3, &edge_op("A", "C", 0.0));
  let epoch = g.epoch;
  g.apply_seq_op(
    4,
    &AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: 2, samples: Arc::new(vec![sample]) }),
  );
  assert!(!g.snapshots.contains(id(&g, "A")));
}

// ---------------------------------------------------------------------------
// A3': the staleness heuristic
// ---------------------------------------------------------------------------

/// With c > 0, every change of a footprint node S adds (1+λ)·α·tv·visits(S)/n to the drift; the
/// snapshot serves while the drift stays within the threshold, and is dropped beyond it. With
/// c = 0 the first such change drops it.
#[test]
fn a3_staleness_drift_and_threshold() {
  for c in [0.0, 1.0] {
    let s = settings(1_000, 1, c);
    let mut g = AugGraph::with_stream(s.clone(), "");
    let mut seq = 0u64;
    let mut op = |g: &mut AugGraph, o: AugGraphOp| {
      seq += 1;
      g.apply_seq_op(seq, &o);
      seq
    };
    // P → hub with many out-edges: one more edge of the hub changes its distribution a little.
    op(&mut g, edge_op("P", "HUB", 1.0));
    for i in 0..40 {
      op(&mut g, edge_op("HUB", &format!("L{i}"), 1.0));
    }
    let base = op(&mut g, AugGraphOp::Barrier);
    let p = id(&g, "P");
    let sample = g.fresh_sample(p).unwrap();
    let epoch = g.epoch;
    op(&mut g, AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: base, samples: Arc::new(vec![sample]) }));
    assert!(g.snapshots.contains(p));
    let hub = id(&g, "HUB");
    let visits = g.snapshots.get(p).unwrap().visits_of(hub) as f64;
    let n = g.snapshots.get(p).unwrap().n as f64;
    let threshold = g.snapshot_threshold(1_000);
    let lambda = s.discredit_lambda;
    let expected_threshold =
      c * (1.0 + lambda) * ((2.0 / STALENESS_DELTA).ln() / (2.0 * 1_000.0)).sqrt();
    assert!((threshold - expected_threshold).abs() < 1e-12);

    let mut expected_drift = 0.0;
    let mut k = 40usize;
    loop {
      // Adding one more edge of weight 1 to a hub of k edges: TV = 1/(k+1).
      op(&mut g, edge_op("HUB", &format!("L{k}"), 1.0));
      let tv = 1.0 / (k as f64 + 1.0);
      k += 1;
      expected_drift += (1.0 + lambda) * s.alpha * tv * visits / n;
      match g.snapshots.get(p) {
        Some(snap) => {
          assert!(c > 0.0, "strict mode must drop the snapshot at the first change");
          assert!((snap.drift - expected_drift).abs() < 1e-9, "{} vs {}", snap.drift, expected_drift);
          assert!(snap.drift <= threshold);
        },
        None => {
          assert!(c == 0.0 || expected_drift > threshold, "dropped early: {expected_drift} <= {threshold}");
          break;
        },
      }
      assert!(k < 10_000, "never dropped");
    }
  }
}

// ---------------------------------------------------------------------------
// A4: eviction is not invalidation
// ---------------------------------------------------------------------------

#[test]
fn a4_eviction_keeps_the_frame_as_a_snapshot() {
  let mut g = AugGraph::with_stream(settings(300, 1, 0.0), "");
  for (seq, (a, b)) in [("E", "P"), ("P", "E"), ("P", "Q"), ("Q", "E")].iter().enumerate() {
    g.apply_seq_op(seq as u64 + 1, &edge_op(a, b, 1.0));
  }
  let (p, e) = (id(&g, "P"), id(&g, "E"));
  // The reading ego is resident, as `ego_read` makes it.
  g.apply_seq_op(5, &AugGraphOp::EnsureCalculated(vec![e, p]));
  let frame_score = g.mr.get_node_score(p, e).unwrap();
  let revision = g.revision(p);
  let calculated = g.counters.frames_calculated;
  assert_eq!(g.reverse_diag(p).source, ReverseSource::Frame);

  g.apply_seq_op(6, &AugGraphOp::ClearEgo(p));
  assert!(!g.mr.is_calculated(p));
  let snap = g.snapshots.get(p).expect("the evicted frame is kept as a snapshot");
  assert_eq!(snap.n, 300);
  assert_eq!(snap.raw(e).to_bits(), frame_score.to_bits());
  assert_eq!(g.revision(p), revision, "eviction is not a new estimate");
  assert_eq!(g.counters.snapshots_captured, 1);

  let diag = g.reverse_diag(p);
  assert_eq!(diag.source, ReverseSource::Snapshot);
  assert_eq!(diag.captured_seq, Some(6));
  assert_eq!(diag.age_ops, Some(0));

  // A read of the reverse score samples and calculates nothing.
  let (rows, report) = read_scope(true, || {
    g.read_node_score(OpReadNodeScore { ego: "E".into(), target: "P".into() })
  });
  assert!(report.sampled.is_empty());
  assert_eq!(g.counters.frames_calculated, calculated);
  assert_eq!(rows[0].reverse_score.to_bits(), g.with_zero_opinion(e, frame_score).to_bits());
}

/// A fresh calculation of an ego whose snapshot is valid reproduces the snapshot exactly (fresh
/// frames are seeded per ego).
#[test]
fn a4_recalculation_reproduces_the_snapshot() {
  let mut g = AugGraph::with_stream(settings(300, 1, 0.0), "");
  for (seq, (a, b)) in [("E", "P"), ("P", "E"), ("P", "Q"), ("Q", "E")].iter().enumerate() {
    g.apply_seq_op(seq as u64 + 1, &edge_op(a, b, 1.0));
  }
  let p = id(&g, "P");
  g.apply_seq_op(5, &AugGraphOp::EnsureCalculated(vec![p]));
  let first = g.mr.frame_sample(p).unwrap();
  g.apply_seq_op(6, &AugGraphOp::ClearEgo(p));
  g.apply_seq_op(7, &AugGraphOp::EnsureCalculated(vec![p]));
  assert_eq!(g.mr.frame_sample(p).unwrap(), first);
  assert_eq!(g.fresh_sample(p).unwrap().counters, first.counters, "ON_DEMAND = NUM_WALKS here");
}

// ---------------------------------------------------------------------------
// Revisions
// ---------------------------------------------------------------------------

#[test]
fn revisions_change_only_with_the_estimate() {
  let mut g = AugGraph::with_stream(settings(200, 1, 0.0), "");
  let mut seq = 0;
  let mut op = |g: &mut AugGraph, o: AugGraphOp| {
    seq += 1;
    g.apply_seq_op(seq, &o);
    seq
  };
  op(&mut g, edge_op("E", "P", 1.0));
  op(&mut g, edge_op("P", "Q", 1.0));
  let p = id(&g, "P");
  op(&mut g, AugGraphOp::EnsureCalculated(vec![p]));
  let r1 = g.revision(p);
  op(&mut g, AugGraphOp::ClearEgo(p));
  assert_eq!(g.revision(p), r1, "eviction");
  op(&mut g, AugGraphOp::WriteZeroOpinion(OpWriteZeroOpinion { node: "E".into(), score: 0.4 }));
  assert_eq!(g.revision(p), r1, "zero opinion");
  op(&mut g, edge_op("Q", "R", 1.0)); // in P's footprint
  let r2 = g.revision(p);
  assert!(r2 > r1, "invalidation");
  let base = op(&mut g, AugGraphOp::Barrier);
  let sample = g.fresh_sample(p).unwrap();
  let epoch = g.epoch;
  op(&mut g, AugGraphOp::AdmitSnapshots(AdmitBatch { epoch, base_seq: base, samples: Arc::new(vec![sample]) }));
  assert!(g.revision(p) > r2, "admission");
}

// ---------------------------------------------------------------------------
// A5: concurrency and replicas
// ---------------------------------------------------------------------------

/// Concurrent writes and reads with a cache of 1: every response equals the same read run on a
/// single-threaded replay of the recorded operations up to the sequence it was read at; after the
/// last sync both buffer copies hold the same snapshots and counters.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a5_concurrent_reads_equal_replay() {
  let s = Settings { record_ops: true, ..settings(100, 1, 0.0) };
  let proc = Arc::new(MultiGraphProcessor::new(s.clone()));
  for i in 0..12 {
    write(&proc, "", &format!("U{i}"), &format!("U{}", (i + 1) % 12), 1.0).await;
    write(&proc, "", &format!("U{i}"), &format!("U{}", (i + 5) % 12), 0.5).await;
  }
  sync(&proc).await;

  let mut js = JoinSet::new();
  for t in 0..4u64 {
    let p = Arc::clone(&proc);
    js.spawn(async move {
      let mut rng = StdRng::seed_from_u64(t);
      for _ in 0..10 {
        let (a, b) = (rng.random_range(0..12), rng.random_range(0..12));
        if a != b {
          write(&p, "", &format!("U{a}"), &format!("U{b}"), rng.random_range(0.0..2.0)).await;
        }
      }
      vec![]
    });
  }
  for t in 0..4u64 {
    let p = Arc::clone(&proc);
    js.spawn(async move {
      let mut rng = StdRng::seed_from_u64(100 + t);
      let mut seen = vec![];
      for _ in 0..10 {
        let ego = format!("U{}", rng.random_range(0..12));
        let req = Request { subgraph: String::new(), data: mutual(&ego) };
        let (resp, at) = p.process_request_traced(&req).await;
        seen.push((ego, bytes(&resp), at.expect("an ego read is traced")));
      }
      seen
    });
  }
  let mut seen = vec![];
  while let Some(r) = js.join_next().await {
    seen.extend(r.unwrap());
  }
  sync(&proc).await;
  sync(&proc).await;

  let ops = proc.recorded_ops("");
  for (ego, resp, (epoch, at)) in seen {
    let mut g = AugGraph::with_stream(s.clone(), "");
    g.epoch = epoch;
    for (seq, op) in ops.iter().filter(|(q, _)| *q <= at) {
      g.apply_seq_op(*seq, op);
    }
    let (rows, _) = read_scope(true, || g.read_mutual_scores(OpReadMutualScores { ego: ego.clone() }));
    assert_eq!(bytes(&Response::Scores(ResScores { scores: rows })), resp, "ego {ego} at {at}");
  }

  let [a, b] = proc.subgraphs_map.get("").unwrap().copies();
  let (a, b) = (a.read(), b.read());
  assert_eq!(a.applied_seq, b.applied_seq);
  assert_eq!(a.snapshots.egos(), b.snapshots.egos());
  for e in a.snapshots.egos() {
    assert_eq!(a.snapshots.get(e), b.snapshots.get(e));
  }
  assert_eq!(a.counters, b.counters);
  assert_eq!(a.snapshots.bytes(), b.snapshots.bytes());
}

// ---------------------------------------------------------------------------
// Resources never make an answer incomplete
// ---------------------------------------------------------------------------

/// A quota far below the working set: snapshots are evicted first-in first-out, and a read still
/// answers exactly (it samples what is missing).
#[test]
fn quota_below_working_set_keeps_answers_exact() {
  let s = settings(200, 1, 0.0);
  let build = |quota: usize| {
    let mut g = AugGraph::with_stream(s.clone(), "");
    let mut seq = 0;
    for i in 0..12 {
      for (a, b) in [("H".to_string(), format!("P{i}")), (format!("P{i}"), "H".to_string())] {
        seq += 1;
        g.apply_seq_op(seq, &edge_op(&a, &b, 1.0));
      }
    }
    seq += 1;
    g.apply_seq_op(seq, &AugGraphOp::SetSnapshotQuota(quota));
    seq += 1;
    let h = id(&g, "H");
    g.apply_seq_op(seq, &AugGraphOp::EnsureCalculated(vec![h]));
    (g, seq)
  };
  let read = |g: &AugGraph| {
    read_scope(true, || g.read_mutual_scores(OpReadMutualScores { ego: "H".into() }))
  };
  let (mut small, mut seq) = build(1_000);
  let (mut large, mut seq_l) = build(usize::MAX);
  for _ in 0..2 {
    let (a, ra) = read(&small);
    let (b, _) = read(&large);
    assert_eq!(bytes(&Response::Scores(ResScores { scores: a })), bytes(&Response::Scores(ResScores { scores: b })));
    let epoch = small.epoch;
    seq += 1;
    small.apply_seq_op(
      seq,
      &AugGraphOp::AdmitSnapshots(AdmitBatch {
        epoch,
        base_seq: seq - 1,
        samples: Arc::new(ra.sampled.iter().map(|x| (**x).clone()).collect()),
      }),
    );
    let (_, rl) = read(&large);
    let epoch = large.epoch;
    seq_l += 1;
    large.apply_seq_op(
      seq_l,
      &AugGraphOp::AdmitSnapshots(AdmitBatch {
        epoch,
        base_seq: seq_l - 1,
        samples: Arc::new(rl.sampled.iter().map(|x| (**x).clone()).collect()),
      }),
    );
    assert!(small.snapshots.bytes() <= 1_000, "{}", small.snapshots.bytes());
  }
  assert_eq!(large.snapshots.len(), 12);
  assert!(small.snapshots.len() < 12);
}

/// Without an admission budget nothing is kept, and answers stay exact.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_admission_budget_keeps_answers_exact() {
  let reference = MultiGraphProcessor::new(settings(200, 0, 0.0));
  let starved = MultiGraphProcessor::new(Settings { admit_queue_mb: 0, ..settings(200, 2, 0.0) });
  load_fixture(&reference, "").await;
  load_fixture(&starved, "").await;
  for _ in 0..2 {
    let a = request(&reference, "", mutual("H")).await;
    let b = request(&starved, "", mutual("H")).await;
    assert_eq!(bytes(&a), bytes(&b));
    sync(&starved).await;
  }
  let st = starved.snapshot_stats("").unwrap();
  assert_eq!(st.snapshot_count, 0);
  assert!(st.admissions_skipped > 0);
}

// ---------------------------------------------------------------------------
// Seeds and settings
// ---------------------------------------------------------------------------

#[test]
fn fresh_seed_depends_on_seed_subgraph_and_ego_only() {
  let s = settings(100, 1, 0.0);
  let mut a = AugGraph::with_stream(s.clone(), "");
  let mut b = AugGraph::with_stream(s.clone(), "");
  a.apply_seq_op(1, &edge_op("U1", "U2", 1.0));
  b.apply_seq_op(1, &edge_op("U1", "U2", 1.0));
  b.apply_seq_op(2, &edge_op("U2", "U3", 1.0));
  b.apply_seq_op(3, &edge_op("U2", "U3", 0.0));
  assert_eq!(a.fresh_seed(0), b.fresh_seed(0));
  assert_ne!(a.fresh_seed(0), a.fresh_seed(1));
  let c = AugGraph::with_stream(s.clone(), "other");
  assert_ne!(a.fresh_seed(0), c.fresh_seed(0));
  let d = AugGraph::with_stream(Settings { seed: 1, ..s }, "");
  assert_ne!(a.fresh_seed(0), d.fresh_seed(0));
  assert_ne!(a.epoch, c.epoch, "every graph incarnation has its own epoch");
}

#[test]
fn snapshot_settings() {
  let s = Settings { num_walks: 500, on_demand_num_walks: 1_000, ..Settings::default() };
  assert_eq!(s.on_demand_walks(), 500, "capped at NUM_WALKS");
  assert!(!Settings { walks_cache_size: 0, ..Settings::default() }.snapshots_enabled());
  assert!(!Settings { walks_cache_size: 5, snapshots_mb: 0, ..Settings::default() }.snapshots_enabled());
  assert!(Settings { walks_cache_size: 5, ..Settings::default() }.snapshots_enabled());
  assert!(Settings::default().validate().is_ok());
  for bad in [
    Settings { snapshot_staleness: -1.0, ..Settings::default() },
    Settings { snapshot_staleness: f64::NAN, ..Settings::default() },
    Settings { snapshot_staleness: f64::INFINITY, ..Settings::default() },
    Settings { on_demand_num_walks: 0, ..Settings::default() },
    Settings { sampling_concurrency: 0, ..Settings::default() },
  ] {
    assert!(bad.validate().is_err());
  }
}
