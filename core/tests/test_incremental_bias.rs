//! Statistical check of the optimized incremental walk invalidation
//! (`OPTIMIZE_INVALIDATION`): walks repaired incrementally after edge changes
//! must be distributed exactly like walks generated from scratch on the final
//! graph. Topologies: a linear chain (one ego) and a star with a hub and
//! several egos. Compared per ego and node: visit frequency (`pos_hits / W`), which is a
//! Binomial(W, p) proportion, against (a) a fresh recalculation on the final
//! graph and (b) the analytic value where the topology makes it easy.
//!
//! Walks use the thread RNG, so tolerances are z-scores, not fixed epsilons.

use meritrank_core::{Graph, MeritRank, NodeId, Weight};

const W: usize = 200_000;
const Z_FAIL: f64 = 5.0;
const ALPHA: f64 = 0.85;

type Edge = (NodeId, NodeId, Weight);

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

fn chain(n: usize) -> Vec<Edge> {
  (1..n).map(|i| (i - 1, i, 1.0)).collect()
}

fn apply_ops(
  edges: &[Edge],
  ops: &[Edge],
) -> Vec<Edge> {
  let mut out: Vec<Edge> = edges.to_vec();
  for &(s, d, w) in ops {
    out.retain(|&(s2, d2, _)| !(s2 == s && d2 == d));
    if w != 0.0 {
      out.push((s, d, w));
    }
  }
  out
}

fn freq(
  mr: &MeritRank,
  ego: NodeId,
  node: NodeId,
) -> f64 {
  mr.get_personal_hits()[&ego].get_count(&node) as f64 / W as f64
}

/// z-score of the difference between two Binomial(W, ·) proportions.
fn z_two(
  p1: f64,
  p2: f64,
) -> f64 {
  let p = (p1 + p2) / 2.0;
  let se = (2.0 * p * (1.0 - p) / W as f64).sqrt();
  if se == 0.0 {
    if p1 == p2 { 0.0 } else { f64::INFINITY }
  } else {
    (p1 - p2) / se
  }
}

/// z-score of an observed proportion against an exact probability.
fn z_one(
  p_obs: f64,
  p_exact: f64,
) -> f64 {
  let se = (p_exact * (1.0 - p_exact) / W as f64).sqrt();
  if se == 0.0 {
    if p_obs == p_exact { 0.0 } else { f64::INFINITY }
  } else {
    (p_obs - p_exact) / se
  }
}

#[derive(Clone, Copy)]
enum Op {
  /// Set edge weight; 0 deletes.
  Edge(NodeId, NodeId, Weight),
  /// Calculate an ego in the middle of the sequence.
  Calc(NodeId),
}

fn edge_ops(ops: &[Op]) -> Vec<Edge> {
  ops
    .iter()
    .filter_map(|op| match *op {
      Op::Edge(s, d, w) => Some((s, d, w)),
      Op::Calc(_) => None,
    })
    .collect()
}

/// Runs the scenario and returns the largest |z| seen (incremental vs fresh,
/// and incremental vs analytic where given). `egos` are calculated before the
/// ops; `Op::Calc` calculates further egos mid-sequence. `analytic` holds
/// `(ego, node, probability)`.
fn run(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
  analytic: &[(NodeId, NodeId, f64)],
) -> f64 {
  let mut inc = build(n, initial);
  for &ego in egos {
    inc.calculate(ego).unwrap();
  }
  let mut all_egos = egos.to_vec();
  for op in ops {
    match *op {
      Op::Edge(s, d, w) => inc.set_edge(s, d, w).unwrap(),
      Op::Calc(ego) => {
        inc.calculate(ego).unwrap();
        all_egos.push(ego);
      },
    }
  }

  let mut fresh = build(n, &apply_ops(initial, &edge_ops(ops)));
  for &ego in &all_egos {
    fresh.calculate(ego).unwrap();
  }

  println!("\n== {name} ==");
  let mut max_z: f64 = 0.0;
  for &ego in &all_egos {
    println!("ego {ego}");
    println!("node   incremental   fresh      z(inc-fresh)  analytic   z(inc-analytic)");
    for node in 0..n {
      let pi = freq(&inc, ego, node);
      let pf = freq(&fresh, ego, node);
      let zf = z_two(pi, pf);
      max_z = max_z.max(zf.abs());
      let a = analytic
        .iter()
        .find(|(e, k, _)| *e == ego && *k == node)
        .map(|(_, _, p)| *p);
      match a {
        Some(pa) => {
          let za = z_one(pi, pa);
          max_z = max_z.max(za.abs());
          println!(
            "{node:>4}   {pi:>10.5}   {pf:>9.5}   {zf:>+10.2}   {pa:>9.5}   {za:>+10.2}"
          );
        },
        None => println!("{node:>4}   {pi:>10.5}   {pf:>9.5}   {zf:>+10.2}"),
      }
    }
  }
  println!("max |z| = {max_z:.2}");
  max_z
}

fn check_multi(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Op],
  analytic: &[(NodeId, NodeId, f64)],
) {
  let z = run(name, n, initial, egos, ops, analytic);
  assert!(z < Z_FAIL, "{name}: max |z| = {z:.2} >= {Z_FAIL}");
}

/// Single-ego (node 0) shorthand used by the chain tests.
fn check(
  name: &str,
  n: usize,
  initial: &[Edge],
  ops: &[Edge],
  analytic: &[(NodeId, f64)],
) {
  let ops: Vec<Op> = ops.iter().map(|&(s, d, w)| Op::Edge(s, d, w)).collect();
  let analytic: Vec<_> = analytic.iter().map(|&(k, p)| (0, k, p)).collect();
  check_multi(name, n, initial, &[0], &ops, &analytic);
}

fn a(k: i32) -> f64 {
  ALPHA.powi(k)
}

// Chain 0→1→…→5 plus a spare node 6 that the ops attach.
const N: usize = 7;

#[test]
fn chain_add_branch_mid() {
  // 2 is not a dead end: walks that stopped at 2 by α must stay stopped.
  let init = chain(6);
  check(
    "add 2→6 (mid-chain branch)",
    N,
    &init,
    &[(2, 6, 1.0)],
    &[
      (0, 1.0),
      (1, a(1)),
      (2, a(2)),
      (3, a(3) / 2.0),
      (4, a(4) / 2.0),
      (5, a(5) / 2.0),
      (6, a(3) / 2.0),
    ],
  );
}

#[test]
fn chain_add_edge_from_tail() {
  // 5 is a dead end: every walk ending at 5 gets a chance to continue.
  let init = chain(6);
  check(
    "add 5→6 (dead-end tail)",
    N,
    &init,
    &[(5, 6, 1.0)],
    &(0..N).map(|k| (k, a(k as i32))).collect::<Vec<_>>(),
  );
}

#[test]
fn chain_add_edge_from_ego() {
  let init = chain(6);
  check(
    "add 0→6 (from ego)",
    N,
    &init,
    &[(0, 6, 3.0)],
    &[
      (0, 1.0),
      (1, a(1) / 4.0),
      (2, a(2) / 4.0),
      (3, a(3) / 4.0),
      (6, a(1) * 3.0 / 4.0),
    ],
  );
}

#[test]
fn chain_delete_mid() {
  // Deleting 3→4 turns 3 into a dead end.
  let init = chain(6);
  check(
    "delete 3→4",
    N,
    &init,
    &[(3, 4, 0.0)],
    &[(0, 1.0), (1, a(1)), (2, a(2)), (3, a(3)), (4, 0.0), (5, 0.0)],
  );
}

#[test]
fn chain_delete_one_of_two() {
  // Forced-step path: walks that took 2→6 re-pick among 2's remaining edges.
  let mut init = chain(6);
  init.push((2, 6, 1.0));
  check(
    "branch 2→6, then delete 2→6",
    N,
    &init,
    &[(2, 6, 0.0)],
    &(0..6).map(|k| (k, a(k as i32))).chain([(6, 0.0)]).collect::<Vec<_>>(),
  );
}

#[test]
fn chain_change_weight() {
  // set_edge on an existing edge = delete + add.
  let mut init = chain(6);
  init.push((2, 6, 1.0));
  check(
    "branch 2→6, then 2→6 weight 1 → 3",
    N,
    &init,
    &[(2, 6, 3.0)],
    &[(3, a(3) / 4.0), (4, a(4) / 4.0), (6, a(3) * 3.0 / 4.0)],
  );
}

#[test]
fn chain_back_edge_cycle() {
  // Cycle 1→…→4→1: repeated visits, unique counting per walk.
  let init = chain(6);
  check(
    "add back edge 4→1 (cycle)",
    N,
    &init,
    &[(4, 1, 1.0)],
    &[],
  );
}

#[test]
fn chain_many_ops() {
  // A sequence of mixed ops on one chain, compared to the final graph only.
  let init = chain(6);
  check(
    "sequence: branch, cycle, reweight, delete, re-add",
    N,
    &init,
    &[
      (2, 6, 1.0),
      (6, 4, 2.0),
      (4, 1, 1.0),
      (2, 6, 0.5),
      (3, 4, 0.0),
      (5, 6, 1.0),
      (4, 1, 0.0),
      (3, 4, 2.0),
    ],
    &[],
  );
}

/// Negative control: the harness must detect a real difference. Incremental
/// graph ends with 2→6 at weight 1, the reference is built with weight 1.3.
#[test]
fn harness_detects_bias() {
  let ego = 0;
  let init = chain(6);
  let mut inc = build(N, &init);
  inc.calculate(ego).unwrap();
  inc.set_edge(2, 6, 1.0).unwrap();
  let mut fresh = build(N, &apply_ops(&init, &[(2, 6, 1.3)]));
  fresh.calculate(ego).unwrap();
  let z = z_two(freq(&inc, ego, 6), freq(&fresh, ego, 6));
  println!("negative control z = {z:.2}");
  assert!(z.abs() >= Z_FAIL, "harness failed to detect a 30% weight change");
}

// ---------------------------------------------------------------------------
// Star with a hub and several egos.
//
// Egos 0, 1, 2 → hub 3 → leaves 4..7; spare nodes 8, 9. Every hub edge change
// invalidates walks of all egos at once, and the cyclic variant revisits the
// hub within one walk (the optimizer scans every occurrence of the source).
// ---------------------------------------------------------------------------

const SN: usize = 10;
const HUB: NodeId = 3;
const EGOS: [NodeId; 3] = [0, 1, 2];

/// Acyclic star: each ego → hub, hub → leaves with the given weights.
fn star(leaves: &[(NodeId, Weight)]) -> Vec<Edge> {
  let mut e: Vec<Edge> = EGOS.iter().map(|&ego| (ego, HUB, 1.0)).collect();
  e.extend(leaves.iter().map(|&(l, w)| (HUB, l, w)));
  e
}

/// Analytic visit probabilities for the acyclic star.
fn star_analytic(leaves: &[(NodeId, Weight)]) -> Vec<(NodeId, NodeId, f64)> {
  let sum: Weight = leaves.iter().map(|(_, w)| w).sum();
  let mut out = vec![];
  for &ego in &EGOS {
    for &other in &EGOS {
      out.push((ego, other, if other == ego { 1.0 } else { 0.0 }));
    }
    out.push((ego, HUB, a(1)));
    for &(l, w) in leaves {
      out.push((ego, l, a(2) * w / sum));
    }
  }
  out
}

/// Cyclic star: leaves 4 and 5 return to the hub, leaf 7 leads to ego 1 (and
/// from there back to the hub), leaf 6 is a dead end; ego 0 also trusts leaf 4.
fn cyclic_star() -> Vec<Edge> {
  let mut e = star(&[(4, 1.0), (5, 2.0), (6, 3.0), (7, 4.0)]);
  e.extend([(0, 4, 1.0), (4, HUB, 1.0), (5, HUB, 1.0), (7, 1, 1.0)]);
  e
}

#[test]
fn star_add_leaf() {
  let before = [(4, 1.0), (5, 2.0), (6, 3.0)];
  let after = [(4, 1.0), (5, 2.0), (6, 3.0), (7, 4.0)];
  check_multi(
    "star: add hub→7",
    SN,
    &star(&before),
    &EGOS,
    &[Op::Edge(HUB, 7, 4.0)],
    &star_analytic(&after),
  );
}

#[test]
fn star_delete_leaf() {
  let before = [(4, 1.0), (5, 2.0), (6, 3.0), (7, 4.0)];
  let after = [(4, 1.0), (6, 3.0), (7, 4.0)];
  check_multi(
    "star: delete hub→5",
    SN,
    &star(&before),
    &EGOS,
    &[Op::Edge(HUB, 5, 0.0)],
    &star_analytic(&after),
  );
}

#[test]
fn star_reweight_leaf() {
  let before = [(4, 1.0), (5, 2.0), (6, 3.0), (7, 4.0)];
  let after = [(4, 6.0), (5, 2.0), (6, 3.0), (7, 4.0)];
  check_multi(
    "star: hub→4 weight 1 → 6",
    SN,
    &star(&before),
    &EGOS,
    &[Op::Edge(HUB, 4, 6.0)],
    &star_analytic(&after),
  );
}

#[test]
fn cyclic_star_add_leaf() {
  check_multi(
    "cyclic star: add hub→8",
    SN,
    &cyclic_star(),
    &EGOS,
    &[Op::Edge(HUB, 8, 2.0)],
    &[],
  );
}

#[test]
fn cyclic_star_delete_leaf() {
  // 5 is on a hub cycle: forced re-pick at a revisited hub.
  check_multi(
    "cyclic star: delete hub→5",
    SN,
    &cyclic_star(),
    &EGOS,
    &[Op::Edge(HUB, 5, 0.0)],
    &[],
  );
}

#[test]
fn cyclic_star_reweight_leaf() {
  check_multi(
    "cyclic star: hub→4 weight 1 → 5",
    SN,
    &cyclic_star(),
    &EGOS,
    &[Op::Edge(HUB, 4, 5.0)],
    &[],
  );
}

#[test]
fn cyclic_star_toggle_back_edges() {
  check_multi(
    "cyclic star: add 6→hub, delete 4→hub",
    SN,
    &cyclic_star(),
    &EGOS,
    &[Op::Edge(6, HUB, 1.0), Op::Edge(4, HUB, 0.0)],
    &[],
  );
}

#[test]
fn cyclic_star_hub_dead_end_and_back() {
  // Strip every hub out-edge (hub becomes a dead end), then restore two.
  check_multi(
    "cyclic star: hub → dead end → partially restored",
    SN,
    &cyclic_star(),
    &EGOS,
    &[
      Op::Edge(HUB, 4, 0.0),
      Op::Edge(HUB, 5, 0.0),
      Op::Edge(HUB, 6, 0.0),
      Op::Edge(HUB, 7, 0.0),
      Op::Edge(HUB, 7, 2.0),
      Op::Edge(HUB, 5, 1.0),
    ],
    &[],
  );
}

#[test]
fn cyclic_star_ego_edges() {
  // Changes at an ego: its own walks start there, others pass through it (7→1).
  check_multi(
    "cyclic star: ego 1 edges change",
    SN,
    &cyclic_star(),
    &EGOS,
    &[
      Op::Edge(1, HUB, 3.0),
      Op::Edge(1, 8, 1.0),
      Op::Edge(8, 2, 1.0),
    ],
    &[],
  );
}

#[test]
fn cyclic_star_mid_sequence_calc() {
  // Ego 2 is first calculated on a partially updated graph, then repaired.
  check_multi(
    "cyclic star: ego 2 calculated mid-sequence",
    SN,
    &cyclic_star(),
    &[0, 1],
    &[
      Op::Edge(HUB, 8, 2.0),
      Op::Edge(HUB, 5, 0.0),
      Op::Calc(2),
      Op::Edge(8, HUB, 1.0),
      Op::Edge(HUB, 4, 4.0),
      Op::Edge(HUB, 6, 0.0),
    ],
    &[],
  );
}

#[test]
fn cyclic_star_many_ops() {
  check_multi(
    "cyclic star: long mixed sequence",
    SN,
    &cyclic_star(),
    &EGOS,
    &[
      Op::Edge(HUB, 8, 2.0),
      Op::Edge(8, 9, 1.0),
      Op::Edge(9, HUB, 1.0),
      Op::Edge(HUB, 5, 0.0),
      Op::Edge(2, 8, 2.0),
      Op::Edge(HUB, 4, 3.0),
      Op::Edge(7, 1, 0.0),
      Op::Edge(7, 2, 1.0),
      Op::Edge(HUB, 5, 0.5),
      Op::Edge(9, HUB, 0.0),
      Op::Edge(6, HUB, 2.0),
    ],
    &[],
  );
}
