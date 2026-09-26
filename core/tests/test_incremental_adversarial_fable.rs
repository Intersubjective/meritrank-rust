//! Adversarial statistical tests for the optimized incremental walk repair
//! (`OPTIMIZE_INVALIDATION`) with NEGATIVE edges in play.
//!
//! For every scenario an instance is built, its egos are calculated, a
//! sequence of edits is applied through `MeritRank::set_edge` (plus optional
//! mid-sequence `calculate` / `clear_ego`), and the resulting per-(ego, node)
//! positive-hit and negative-hit frequencies (`count / W`, each a
//! Binomial(W, p) proportion) are compared with a fresh instance built on the
//! final graph using a pooled two-sample z-test. |z| >= 5 is a failure.
//!
//! Written by an adversarial agent (Fable); the defects it found (stale
//! `negative_segment_start` after recalculation, cancelling cached sums, panic
//! on a tiny weight to an absent edge) are fixed, so these are regression tests
//! now. Slow (minutes): run with `--features expensive_tests`.
#![cfg(feature = "expensive_tests")]

use meritrank_core::{Graph, MeritRank, NodeId, Weight};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::sync::Mutex;

/// Walks per ego. Debug builds run the internal consistency assertions after
/// every edit (O(egos * peers * visits)), so they use a smaller W.
const W: usize = if cfg!(debug_assertions) { 20_000 } else { 200_000 };
const Z_FAIL: f64 = 5.0;
const ALPHA: f64 = 0.85;

/// Each scenario holds two MeritRank instances with W walks per ego; run the
/// scenarios one at a time to keep peak memory bounded under the parallel
/// test runner.
static SERIAL: Mutex<()> = Mutex::new(());

type Edge = (NodeId, NodeId, Weight);

#[derive(Clone, Copy, Debug)]
enum Op {
  /// Set edge weight through the public API; 0 deletes.
  Edge(NodeId, NodeId, Weight),
  /// (Re)calculate an ego mid-sequence.
  Calc(NodeId),
  /// Evict an ego's walks (cache eviction path).
  ClearEgo(NodeId),
}

fn build(
  n: usize,
  edges: &[Edge],
) -> MeritRank {
  let mut mr = MeritRank::new(Graph::new(), W);
  mr.alpha = ALPHA;
  for _ in 0..n {
    mr.get_new_nodeid();
  }
  for &(s, d, w) in edges {
    mr.set_edge(s, d, w).unwrap();
  }
  mr
}

fn apply_ops(
  edges: &[Edge],
  ops: &[Edge],
) -> Vec<Edge> {
  let mut out: Vec<Edge> = edges.to_vec();
  for &(s, d, w) in ops {
    out.retain(|&(s2, d2, _)| !(s2 == s && d2 == d));
    if w.abs() > 1e-6 {
      out.push((s, d, w));
    }
  }
  out
}

fn edge_ops(ops: &[Op]) -> Vec<Edge> {
  ops
    .iter()
    .filter_map(|op| match *op {
      Op::Edge(s, d, w) => Some((s, d, w)),
      _ => None,
    })
    .collect()
}

fn pos_freq(
  mr: &MeritRank,
  ego: NodeId,
  node: NodeId,
) -> f64 {
  mr.get_personal_hits()
    .get(&ego)
    .map_or(0.0, |c| c.get_count(&node) as f64)
    / W as f64
}

fn neg_freq(
  mr: &MeritRank,
  ego: NodeId,
  node: NodeId,
) -> f64 {
  mr.get_negative_hits()
    .get(&ego)
    .map_or(0.0, |c| c.get_count(&node) as f64)
    / W as f64
}

/// z-score of the difference between two Binomial(W, .) proportions.
fn z_two(
  p1: f64,
  p2: f64,
) -> f64 {
  let p = (p1 + p2) / 2.0;
  let se = (2.0 * p * (1.0 - p) / W as f64).sqrt();
  if se == 0.0 {
    if p1 == p2 {
      0.0
    } else {
      f64::INFINITY
    }
  } else {
    (p1 - p2) / se
  }
}

struct Outcome {
  max_z: f64,
  worst: String,
}

/// Runs the scenario once. Returns the largest |z| over all (ego, node,
/// pos/neg) cells and a description of the worst cell.
fn run(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
) -> Outcome {
  let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

  let mut inc = build(n, initial);
  let mut live: Vec<NodeId> = Vec::new();
  for &ego in egos {
    inc.calculate(ego).unwrap();
    live.push(ego);
  }
  for op in ops {
    match *op {
      Op::Edge(s, d, w) => inc.set_edge(s, d, w).unwrap(),
      Op::Calc(ego) => {
        inc.calculate(ego).unwrap();
        if !live.contains(&ego) {
          live.push(ego);
        }
      },
      Op::ClearEgo(ego) => {
        inc.clear_ego(ego).unwrap();
        live.retain(|&e| e != ego);
      },
    }
  }

  let mut fresh = build(n, &apply_ops(initial, &edge_ops(ops)));
  for &ego in &live {
    fresh.calculate(ego).unwrap();
  }

  println!("\n== {name} ==");
  let mut max_z: f64 = 0.0;
  let mut worst = String::from("(none)");
  for &ego in &live {
    println!("ego {ego}");
    println!(
      "node    pos_inc   pos_fresh   z_pos      neg_inc   neg_fresh   z_neg"
    );
    for node in 0..n {
      let (pi, pf) = (pos_freq(&inc, ego, node), pos_freq(&fresh, ego, node));
      let (ni, nf) = (neg_freq(&inc, ego, node), neg_freq(&fresh, ego, node));
      if pi == 0.0 && pf == 0.0 && ni == 0.0 && nf == 0.0 {
        continue;
      }
      let zp = z_two(pi, pf);
      let zn = z_two(ni, nf);
      println!(
        "{node:>4}   {pi:>8.5}   {pf:>9.5}   {zp:>+7.2}    {ni:>8.5}   {nf:>9.5}   {zn:>+7.2}"
      );
      if zp.abs() > max_z {
        max_z = zp.abs();
        worst = format!("ego {ego} node {node} pos: inc {pi:.5} fresh {pf:.5} z {zp:+.2}");
      }
      if zn.abs() > max_z {
        max_z = zn.abs();
        worst = format!("ego {ego} node {node} neg: inc {ni:.5} fresh {nf:.5} z {zn:+.2}");
      }
    }
  }
  println!("max |z| = {max_z:.2}  ({worst})");
  Outcome { max_z, worst }
}

fn check(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
) {
  let o = run(name, n, initial, egos, ops);
  assert!(o.max_z < Z_FAIL, "{name}: max |z| = {:.2} >= {Z_FAIL}: {}", o.max_z, o.worst);
}

/// Runs the scenario `trials` times and fails if any trial exceeds the
/// threshold; prints all trial maxima so a systematic bias is visible as a
/// repeated, same-sign excursion rather than a one-off.
fn check_repeated(
  name: &str,
  trials: usize,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
) {
  let mut results = Vec::new();
  for t in 0..trials {
    let o = run(&format!("{name} [trial {t}]"), n, initial, egos, ops);
    results.push((o.max_z, o.worst));
  }
  let summary: Vec<String> = results
    .iter()
    .map(|(z, w)| format!("{z:.1} ({w})"))
    .collect();
  println!("{name}: trial maxima: {}", summary.join(" | "));
  let worst = results
    .iter()
    .map(|(z, _)| *z)
    .fold(0.0_f64, f64::max);
  assert!(
    worst < Z_FAIL,
    "{name}: max |z| over {trials} trials = {worst:.2} >= {Z_FAIL}; trials: {}",
    summary.join(" | ")
  );
}

// ---------------------------------------------------------------------------
// Mixed-sign graph.
//
//   0 -(+2)-> 1     0 -(-1)-> 2
//   1 -(+1)-> 3     1 -(-1)-> 4
//   2 -(+1)-> 3     2 -(+1)-> 5
//   3 -(+1)-> 6     3 -(+1)-> 0     (back into ego 0)
//   4 -(+1)-> 5     4 -(-1)-> 1
//   5 -(+1)-> 6     5 -(-2)-> 2
//   6 -(+1)-> 1     6 -(+1)-> 5
//   7 spare
//
// From ego 0: node 2 is only ever entered through a negative edge (so it sits
// at `negative_segment_start`), node 4 likewise; 0, 1, 3, 5, 6 are visited in
// both regimes (3 -> 0 brings the walk back to the ego in negative mode).
// ---------------------------------------------------------------------------

const MN: usize = 8;

fn mixed() -> Vec<Edge> {
  vec![
    (0, 1, 2.0),
    (0, 2, -1.0),
    (1, 3, 1.0),
    (1, 4, -1.0),
    (2, 3, 1.0),
    (2, 5, 1.0),
    (3, 6, 1.0),
    (3, 0, 1.0),
    (4, 5, 1.0),
    (4, 1, -1.0),
    (5, 6, 1.0),
    (5, 2, -2.0),
    (6, 1, 1.0),
    (6, 5, 1.0),
  ]
}

fn e(
  s: NodeId,
  d: NodeId,
  w: Weight,
) -> Op {
  Op::Edge(s, d, w)
}

/// Negative control on the negative-hit channel: incremental instance ends
/// with 0->2 at -1, the reference is built with -1.4. The harness must see it.
#[test]
fn harness_detects_bias_in_negative_hits() {
  let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
  let init = mixed();
  let mut inc = build(MN, &init);
  inc.calculate(0).unwrap();
  inc.set_edge(0, 2, -1.0).unwrap(); // same weight: delete + add path
  let mut fresh = build(MN, &apply_ops(&init, &[(0, 2, -1.4)]));
  fresh.calculate(0).unwrap();
  let zn = z_two(neg_freq(&inc, 0, 2), neg_freq(&fresh, 0, 2));
  let zp = z_two(pos_freq(&inc, 0, 1), pos_freq(&fresh, 0, 1));
  println!("negative control: z_neg(2) = {zn:.2}, z_pos(1) = {zp:.2}");
  assert!(zn.abs() >= Z_FAIL && zp.abs() >= Z_FAIL, "harness failed to detect a 40% weight change on a negative edge");
}

/// Edge additions at the three positions around a segment boundary: from the
/// node that takes the negative edge (5 before `-2 -> 2`), from the node AT
/// the boundary (2), and from a node right after it (3, also visited in the
/// positive regime). Positive and negative new edges.
#[test]
fn boundary_additions_before_at_after_neg_start() {
  let init = mixed();
  check("add +1 from 5 (node before neg_start)", MN, &init, &[0], &[e(5, 7, 1.0)]);
  check("add -1 from 5 (node before neg_start)", MN, &init, &[0], &[e(5, 7, -1.0)]);
  check("add +1 from 2 (node at neg_start)", MN, &init, &[0], &[e(2, 7, 1.0)]);
  check("add -1 from 2 (node at neg_start; no-op for neg regime)", MN, &init, &[0], &[e(2, 7, -1.0)]);
  check("add +3 from 3 (both regimes)", MN, &init, &[0], &[e(3, 7, 3.0)]);
  check("add -3 from 3 (both regimes)", MN, &init, &[0], &[e(3, 7, -3.0)]);
  check("add negative edge from ego", MN, &init, &[0], &[e(0, 7, -2.0)]);
}

/// Deletions that force a step in a particular regime: the negative edge that
/// opens a segment (0->2, 5->2, 1->4), a positive edge taken in the negative
/// regime (2->3), and the last positive edge of a node that then dead-ends in
/// the negative regime only (4->5).
#[test]
fn deletions_force_step_in_right_regime() {
  let init = mixed();
  check("delete negative edge from ego 0->2", MN, &init, &[0], &[e(0, 2, 0.0)]);
  check("delete negative edge 5->2 (cycle)", MN, &init, &[0], &[e(5, 2, 0.0)]);
  check("delete negative edge 1->4", MN, &init, &[0], &[e(1, 4, 0.0)]);
  check("delete positive edge 2->3 taken in neg regime", MN, &init, &[0], &[e(2, 3, 0.0)]);
  check("delete 4->5: dead end in neg regime only", MN, &init, &[0], &[e(4, 5, 0.0)]);
  check(
    "dead end in neg regime, then positive edge re-added",
    MN,
    &init,
    &[0],
    &[e(4, 5, 0.0), e(4, 7, 1.0)],
  );
  check(
    "strip all positive edges from 2, then restore one",
    MN,
    &init,
    &[0],
    &[e(2, 3, 0.0), e(2, 5, 0.0), e(2, 7, 2.0)],
  );
}

/// Sign flips through set_edge (delete + add), same weight re-set, and
/// re-weighting across sign.
#[test]
fn sign_flips_and_same_weight() {
  let init = mixed();
  check("flip 0->2 neg -> pos", MN, &init, &[0], &[e(0, 2, 1.0)]);
  check("flip 1->3 pos -> neg", MN, &init, &[0], &[e(1, 3, -1.0)]);
  check("flip 5->2 neg -> pos", MN, &init, &[0], &[e(5, 2, 2.0)]);
  check("flip 3->0 (edge into ego) pos -> neg", MN, &init, &[0], &[e(3, 0, -1.0)]);
  check(
    "flip back and forth 0->2: -1, +1, -1, +2, -2",
    MN,
    &init,
    &[0],
    &[e(0, 2, 1.0), e(0, 2, -1.0), e(0, 2, 2.0), e(0, 2, -2.0)],
  );
  check(
    "same weight set twice on pos and neg edges",
    MN,
    &init,
    &[0],
    &[e(0, 1, 2.0), e(0, 2, -1.0), e(0, 1, 2.0), e(5, 2, -2.0)],
  );
}

/// Edges into the ego, a node visited in both segments of the same walk, and
/// removing the cycle that brings walks back to the ego.
#[test]
fn edges_into_ego_and_both_segment_visits() {
  let init = mixed();
  check("add 6->0 positive into ego", MN, &init, &[0], &[e(6, 0, 1.0)]);
  check("add 5->0 negative into ego", MN, &init, &[0], &[e(5, 0, -1.0)]);
  check("delete 3->0 (cycle to ego)", MN, &init, &[0], &[e(3, 0, 0.0)]);
  check(
    "ego gains a positive edge to a neg-only node",
    MN,
    &init,
    &[0],
    &[e(0, 4, 1.0)],
  );
  check(
    "node 2 becomes reachable positively then loses the negative entry",
    MN,
    &init,
    &[0],
    &[e(6, 2, 1.0), e(0, 2, 0.0), e(5, 2, 0.0)],
  );
}

/// Several egos sharing the same nodes; some egos start on nodes that other
/// egos only reach in the negative regime.
#[test]
fn multi_ego_shared_signed_nodes() {
  let init = mixed();
  let egos = [0, 2, 4, 6];
  check("multi-ego: add 2->7 +1", MN, &init, &egos, &[e(2, 7, 1.0)]);
  check("multi-ego: delete 0->2", MN, &init, &egos, &[e(0, 2, 0.0)]);
  check("multi-ego: flip 4->1 neg -> pos", MN, &init, &egos, &[e(4, 1, 1.0)]);
  check(
    "multi-ego: mixed sequence",
    MN,
    &init,
    &egos,
    &[
      e(2, 7, 1.0),
      e(7, 0, -1.0),
      e(0, 2, 0.0),
      e(4, 1, 1.0),
      e(6, 5, -1.0),
      e(3, 0, 0.0),
      e(7, 4, 2.0),
      e(0, 2, -3.0),
      e(2, 7, 0.0),
    ],
  );
}

/// A NEW ego calculated mid-sequence (on a partially updated graph) and then
/// repaired further. Its walk objects are fresh, so this is expected to pass.
#[test]
fn new_ego_calculated_mid_sequence() {
  let init = mixed();
  check(
    "new ego 2 mid-sequence",
    MN,
    &init,
    &[0],
    &[e(2, 7, 1.0), e(0, 2, 0.0), Op::Calc(2), e(7, 3, -1.0), e(5, 2, 0.0), e(0, 2, -1.0)],
  );
}

/// Long mixed sequence, single ego.
#[test]
fn long_mixed_sequence() {
  let init = mixed();
  check(
    "long mixed sequence",
    MN,
    &init,
    &[0],
    &[
      e(2, 7, 1.0),
      e(7, 1, -1.0),
      e(0, 2, 0.0),
      e(1, 3, -1.0),
      e(4, 5, 0.0),
      e(4, 7, 1.0),
      e(0, 2, -2.0),
      e(6, 0, 1.0),
      e(5, 2, 0.0),
      e(3, 6, 0.0),
      e(3, 6, -1.0),
      e(1, 3, 2.0),
      e(2, 3, 0.0),
      e(2, 5, 0.0),
      e(2, 3, 0.5),
      e(7, 1, 1.0),
      e(0, 7, -1.0),
      e(0, 1, 0.0),
      e(0, 1, 1.0),
    ],
  );
}

// ---------------------------------------------------------------------------
// FINDING 1: re-calculating an ego whose walks had negative segments.
//
// `RandomWalk::clear` (core/src/random_walk.rs:77) clears `nodes` but leaves
// `negative_segment_start` set. `WalkStorage::clear_block_for_ego`
// (walk_storage.rs:133) reuses the walk objects, and `MeritRank::calculate`
// (rank.rs) then calls `continue_walk`, which reads
// `negative_segment_start.is_some()` as "positive-only mode". So every walk
// that previously entered a negative segment is regenerated sampling only
// positive edges from the ego, and the nodes at positions >= the stale
// `negative_segment_start` are booked as negative hits.
// ---------------------------------------------------------------------------

/// Minimal repro: 0 -(-1)-> 1 -(+1)-> 2. Fresh: neg(1) = a. After a second
/// calculate, the ~a fraction of walks that had a negative segment can only
/// sample positive edges from 0 (none), so they collapse to [0].
#[test]
fn recalc_minimal_negative_chain() {
  check_repeated(
    "calculate(0) twice on 0 -(-1)-> 1 -> 2",
    3,
    3,
    &[(0, 1, -1.0), (1, 2, 1.0)],
    &[0],
    &[Op::Calc(0)],
  );
}

/// Same defect on the mixed graph, and through the cache-eviction path
/// (`clear_ego` followed by `calculate`).
#[test]
fn recalc_and_clear_ego_on_mixed_graph() {
  let init = mixed();
  check_repeated("mixed: calculate(0) twice", 2, MN, &init, &[0], &[Op::Calc(0)]);
  check_repeated(
    "mixed: clear_ego(0) then calculate(0)",
    2,
    MN,
    &init,
    &[0],
    &[Op::ClearEgo(0), Op::Calc(0)],
  );
}

/// Recalc between edits: the stale segment marker also corrupts the regime
/// used by later incremental repairs.
#[test]
fn recalc_between_signed_edits() {
  let init = mixed();
  check_repeated(
    "mixed: edit, recalc, edit",
    2,
    MN,
    &init,
    &[0, 2],
    &[e(2, 7, 1.0), Op::Calc(0), e(7, 3, -1.0), e(0, 2, 0.0)],
  );
}

/// Control for finding 1: recalculating an ego on an all-positive graph is
/// fine (no walk ever had a negative segment).
#[test]
fn recalc_on_all_positive_graph_is_fine() {
  let init: Vec<Edge> = mixed().into_iter().map(|(s, d, w)| (s, d, w.abs())).collect();
  check("all-positive: calculate(0) twice", MN, &init, &[0], &[Op::Calc(0), e(2, 7, 1.0)]);
}

// ---------------------------------------------------------------------------
// FINDING 2: catastrophic cancellation in the cached edge sums.
//
// `NodeData::pos_sum` / `neg_sum` are maintained with `+=` on insert and
// `-=` (clamped at 0) on removal (core/src/graph.rs, `set_edge` /
// `remove_edge`), while `set_edge_` computes the invalidation probability as
// |w| / (abs_sum + |w|). With a weight ratio above 2^53 the small edge is
// absorbed into the sum; removing the large edge leaves `pos_sum == 0`
// although the small edge still exists, so the next addition invalidates
// every walk at the node with probability 1 instead of w / (1 + w).
// ---------------------------------------------------------------------------
#[test]
fn cached_sum_cancellation_with_large_weight_ratio() {
  check_repeated(
    "0->1 (1e16), 0->2 (1); delete 0->1; add 0->3 (1)",
    3,
    4,
    &[(0, 1, 1e16), (0, 2, 1.0)],
    &[0],
    &[e(0, 1, 0.0), e(0, 3, 1.0)],
  );
}

// ---------------------------------------------------------------------------
// FINDING 3: EPSILON handling of a tiny weight on a MISSING edge.
//
// `set_edge_` treats |w| <= EPSILON as deletion and calls
// `Graph::remove_edge`, which panics ("Edge not found") when the edge does
// not exist. `set_edge(s, d, 0.0)` on a missing edge is a silent no-op (the
// `old_weight == new_weight` early return), so 0 and 1e-7 behave differently.
// ---------------------------------------------------------------------------
#[test]
fn epsilon_tiny_weight_on_missing_edge_must_not_panic() {
  let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
  let mut mr = build(3, &[(0, 1, 1.0)]);
  mr.calculate(0).unwrap();
  // Existing edge with a tiny weight: treated as deletion, fine.
  mr.set_edge(0, 1, 1e-7).unwrap();
  assert_eq!(mr.graph.edge_weight(0, 1).unwrap(), None);
  // Missing edge with a tiny weight.
  let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    mr.set_edge(1, 2, 1e-7)
  }));
  match outcome {
    Ok(Ok(())) => {},
    Ok(Err(err)) => println!("set_edge(1, 2, 1e-7) returned a clean error: {err:?}"),
    Err(_) => panic!(
      "set_edge(1, 2, 1e-7) on a missing edge panicked (Graph::remove_edge 'Edge not found'); \
       set_edge(1, 2, 0.0) on the same missing edge is a no-op"
    ),
  }
}

// ---------------------------------------------------------------------------
// Seeded fuzzer: small dense graphs with mixed signs and random edit
// sequences. Reports the worst seed (and its scenario) so it can be minimised.
// ---------------------------------------------------------------------------

fn rand_weight(rng: &mut StdRng) -> Weight {
  let mag = [0.5, 1.0, 2.0, 4.0][rng.random_range(0..4)];
  if rng.random::<f64>() < 0.35 {
    -mag
  } else {
    mag
  }
}

fn fuzz_case(
  seed: u64,
  n: usize,
  n_ops: usize,
  with_recalc: bool,
) -> (Vec<Edge>, Vec<Op>) {
  let mut rng = StdRng::seed_from_u64(seed);
  let mut edges: Vec<Edge> = Vec::new();
  for s in 0..n {
    for d in 0..n {
      if s != d && rng.random::<f64>() < 0.45 {
        edges.push((s, d, rand_weight(&mut rng)));
      }
    }
  }
  let mut cur = edges.clone();
  let mut ops = Vec::new();
  for _ in 0..n_ops {
    if with_recalc && rng.random::<f64>() < 0.15 {
      ops.push(Op::Calc(rng.random_range(0..n)));
      continue;
    }
    let s = rng.random_range(0..n);
    let mut d = rng.random_range(0..n - 1);
    if d >= s {
      d += 1;
    }
    let exists = cur.iter().any(|&(a, b, _)| a == s && b == d);
    let w = if exists && rng.random::<f64>() < 0.4 {
      0.0
    } else {
      rand_weight(&mut rng)
    };
    ops.push(Op::Edge(s, d, w));
    cur = apply_ops(&cur, &[(s, d, w)]);
  }
  (edges, ops)
}

fn fuzz(
  name: &str,
  seeds: std::ops::Range<u64>,
  n: usize,
  n_ops: usize,
  egos: &[NodeId],
  with_recalc: bool,
) {
  let mut worst: Option<(u64, f64, String, Vec<Edge>, Vec<Op>)> = None;
  for seed in seeds {
    let (edges, ops) = fuzz_case(seed, n, n_ops, with_recalc);
    let o = run(&format!("{name} seed {seed}"), n, &edges, egos, &ops);
    println!("{name} seed {seed}: max |z| = {:.2} ({})", o.max_z, o.worst);
    if worst.as_ref().map_or(true, |w| o.max_z > w.1) {
      worst = Some((seed, o.max_z, o.worst, edges, ops));
    }
  }
  let (seed, z, desc, edges, ops) = worst.unwrap();
  println!("\n{name}: WORST seed {seed}: max |z| = {z:.2} ({desc})");
  println!("  initial edges: {edges:?}");
  println!("  ops: {ops:?}");
  assert!(z < Z_FAIL, "{name}: seed {seed} max |z| = {z:.2} >= {Z_FAIL} ({desc}); edges {edges:?}; ops {ops:?}");
}

/// Edit-only fuzzing (no recalculation of existing egos).
#[test]
fn fuzz_signed_edits_only() {
  let seeds = if cfg!(debug_assertions) { 0..4 } else { 0..10 };
  fuzz("fuzz edits-only", seeds, 6, 25, &[0, 1, 2], false);
}

/// Fuzzing with mid-sequence `calculate` of random nodes (may hit an existing
/// ego and therefore finding 1).
#[test]
fn fuzz_signed_edits_with_recalc() {
  let seeds = if cfg!(debug_assertions) { 100..104 } else { 100..110 };
  fuzz("fuzz with recalc", seeds, 6, 25, &[0, 1, 2], true);
}
