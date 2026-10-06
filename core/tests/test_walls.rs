//! Negative edges as absorbing walls (NEGATIVE_EDGES_FEATURE.md): analytic values, axioms, and
//! incremental maintenance (R16 and positive edges around walls) against generation from
//! scratch on the final graph.
//!
//! Statistics: per ego and node, credits / W and blame / W are means of per-walk values in
//! [0, 1], so `p(1 − p)/W` bounds their variance; two-sample and one-sample z-scores use it and
//! fail at |z| >= 5.

use meritrank_core::{BlameRadius, Graph, MeritRank, NodeId, Weight};

const W: usize = 40_000;
const Z_FAIL: f64 = 5.0;
const A: f64 = 0.85;

type Edge = (NodeId, NodeId, Weight);

fn build(
  n: usize,
  edges: &[Edge],
  seed: u64,
) -> MeritRank {
  let mut mr = MeritRank::new(Graph::new(), W);
  mr.alpha = A;
  mr.reseed(seed);
  for _ in 0..n {
    mr.get_new_nodeid();
  }
  for &(s, d, w) in edges {
    mr.set_edge(s, d, w).unwrap();
  }
  mr
}

fn credits(
  mr: &MeritRank,
  ego: NodeId,
  node: NodeId,
) -> f64 {
  mr.credits_of(ego, node) as f64 / W as f64
}

fn blame(
  mr: &MeritRank,
  ego: NodeId,
  node: NodeId,
) -> f64 {
  mr.blame_of(ego, node)
    / W as f64
}

fn z_two(
  p1: f64,
  p2: f64,
) -> f64 {
  let p = ((p1 + p2) / 2.0).clamp(0.0, 1.0);
  let se = (2.0 * p * (1.0 - p) / W as f64).sqrt();
  if se == 0.0 {
    if (p1 - p2).abs() < 1e-12 { 0.0 } else { f64::INFINITY }
  } else {
    (p1 - p2) / se
  }
}

fn z_one(
  p: f64,
  exact: f64,
) -> f64 {
  let se = (exact.clamp(0.0, 1.0) * (1.0 - exact.clamp(0.0, 1.0)) / W as f64).sqrt();
  if se == 0.0 {
    if (p - exact).abs() < 1e-12 { 0.0 } else { f64::INFINITY }
  } else {
    (p - exact) / se
  }
}

fn assert_near(
  what: &str,
  got: f64,
  exact: f64,
) {
  let z = z_one(got, exact);
  assert!(z.abs() < Z_FAIL, "{what}: {got} vs exact {exact} (z = {z:.1})");
}

fn apply(
  edges: &[Edge],
  ops: &[Edge],
) -> Vec<Edge> {
  let mut out = edges.to_vec();
  for &(s, d, w) in ops {
    out.retain(|&(s2, d2, _)| !(s2 == s && d2 == d));
    if w != 0.0 {
      out.push((s, d, w));
    }
  }
  out
}

/// Incremental (initial graph, egos calculated, then `ops`) vs fresh (final graph): every ego's
/// credits and blame per node. Returns the largest |z|.
fn compare(
  name: &str,
  n: usize,
  initial: &[Edge],
  egos: &[NodeId],
  ops: &[Edge],
  configure: impl Fn(&mut MeritRank),
) -> f64 {
  let mut inc = build(n, initial, 1);
  configure(&mut inc);
  for &e in egos {
    inc.calculate(e).unwrap();
  }
  for &(s, d, w) in ops {
    inc.set_edge(s, d, w).unwrap();
  }
  let mut fresh = build(n, &apply(initial, ops), 2);
  configure(&mut fresh);
  for &e in egos {
    fresh.calculate(e).unwrap();
  }
  let mut worst: f64 = 0.0;
  let mut report = String::new();
  for &e in egos {
    for node in 0..n {
      let zc = z_two(credits(&inc, e, node), credits(&fresh, e, node));
      let zb = z_two(blame(&inc, e, node), blame(&fresh, e, node));
      if zc.abs().max(zb.abs()) > worst {
        worst = zc.abs().max(zb.abs());
        report = format!(
          "ego {e} node {node}: credits {:.4}/{:.4} (z {zc:.1}), blame {:.4}/{:.4} (z {zb:.1})",
          credits(&inc, e, node),
          credits(&fresh, e, node),
          blame(&inc, e, node),
          blame(&fresh, e, node)
        );
      }
    }
  }
  println!("{name}: max |z| = {worst:.2} ({report})");
  assert!(worst < Z_FAIL, "{name}: {report}");
  worst
}

fn discredit(lambda: f64) -> impl Fn(&mut MeritRank) {
  move |mr: &mut MeritRank| {
    mr.discredit = lambda;
    mr.blame_decay = 0.8;
  }
}

// ---------------------------------------------------------------------------
// Analytic values
// ---------------------------------------------------------------------------

/// Chain 0 → 1 → 2 → 3 with a wall 0 ⊣ 2. Hard: everything at and behind the wall scores 0
/// (A2), node 1 is credited only by walks that stop at 1, the ego by the unabsorbed walks.
/// Soft (d = 0.5): nodes behind the wall get exactly (1 − d) of their baseline.
#[test]
fn chain_hard_and_soft_wall() {
  let chain = [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0)];
  let mut mr = build(4, &chain, 7);
  mr.set_edge(0, 2, -1.0).unwrap();
  mr.calculate(0).unwrap();
  assert_near("hard: ego", credits(&mr, 0, 0), 1.0 - A * A);
  assert_near("hard: node 1", credits(&mr, 0, 1), A * (1.0 - A));
  assert_eq!(credits(&mr, 0, 2), 0.0);
  assert_eq!(credits(&mr, 0, 3), 0.0);
  assert_eq!(mr.get_node_score(0, 3).unwrap(), 0.0);

  let d = 0.5;
  let mut mr = build(4, &chain, 8);
  mr.set_edge(0, 2, -d).unwrap();
  mr.calculate(0).unwrap();
  assert_near("soft: node 3", credits(&mr, 0, 3), (1.0 - d) * A.powi(3));
  // The wall itself is credited when the walk passes it and is not absorbed later.
  assert_near("soft: node 2", credits(&mr, 0, 2), (1.0 - d) * A * A);
  assert_near("soft: ego", credits(&mr, 0, 0), 1.0 - d * A * A);
}

/// Closed form with discredit (0 → 1 → 2, hard wall 0 ⊣ 2): the voucher 1 gets
/// `a(1 − a) − λ·b·a²` with b = γ (whole prefix) or b = 1 (direct voucher); the wall −λ·a²; the
/// ego is never blamed (R13) and scores 1 − a².
#[test]
fn closed_form_with_discredit() {
  let (lambda, gamma) = (0.6, 0.8);
  for (radius, b) in [(BlameRadius::Prefix, gamma), (BlameRadius::Voucher, 1.0)] {
    let mut mr = build(3, &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, -1.0)], 9);
    mr.discredit = lambda;
    mr.blame_decay = gamma;
    mr.blame_radius = radius;
    mr.calculate(0).unwrap();
    assert_near("blame of the wall", blame(&mr, 0, 2), A * A);
    assert_near("blame of the voucher", blame(&mr, 0, 1), b * A * A);
    assert_eq!(blame(&mr, 0, 0), 0.0, "the ego is never blamed");
    let s1 = mr.get_node_score(0, 1).unwrap();
    let exact = A * (1.0 - A) - lambda * b * A * A;
    let se = ((1.0 + lambda) / (W as f64).sqrt()).max(1e-9);
    assert!((s1 - exact).abs() < 5.0 * se, "{radius:?}: score(1) {s1} vs {exact}");
    let s0 = mr.get_node_score(0, 0).unwrap();
    assert!((s0 - (1.0 - A * A)).abs() < 5.0 * se, "{radius:?}: ego {s0}");
  }
}

/// Blame takes the visit nearest to the wall (R10). With alpha = 1 every walk loops 1 ⇄ 2 until it
/// leaves for the hard wall 3, so every walk is absorbed with the same nearest distances:
/// 3 → b = 1, 2 → γ, 1 → γ², however often 1 and 2 were visited.
#[test]
fn blame_uses_the_visit_nearest_to_the_wall() {
  let gamma = 0.5;
  let mut mr = build(
    4,
    &[(0, 1, 1.0), (1, 2, 1.0), (2, 1, 1.0), (2, 3, 1.0), (0, 3, -1.0)],
    10,
  );
  mr.alpha = 1.0;
  mr.blame_decay = gamma;
  mr.calculate(0).unwrap();
  assert_eq!(credits(&mr, 0, 0), 0.0, "every walk is absorbed");
  assert!((blame(&mr, 0, 3) - 1.0).abs() < 1e-9);
  assert!((blame(&mr, 0, 2) - gamma).abs() < 1e-9);
  assert!((blame(&mr, 0, 1) - gamma * gamma).abs() < 1e-9);
}

/// Every entry is a trial (D16): through a cycle B → D → B the walks entering B are absorbed with
/// `d_eff = d / (1 − (1 − d)·r)`, r = a² the return probability.
#[test]
fn cycle_through_a_soft_wall_retries() {
  let d = 0.4;
  // 0 → 3 → 1 (the wall), 1 ⇄ 2.
  let mut mr = build(4, &[(0, 3, 1.0), (3, 1, 1.0), (1, 2, 1.0), (2, 1, 1.0)], 13);
  mr.set_edge(0, 1, -d).unwrap();
  mr.calculate(0).unwrap();
  // Walks reach 1 with probability a²; each return (1 → 2 → 1) takes a² too.
  let r = A * A;
  let d_eff = d / (1.0 - (1.0 - d) * r);
  let absorbed = 1.0 - credits(&mr, 0, 0);
  assert_near("absorbed fraction", absorbed, A * A * d_eff);
  // A single trial would give a²·d instead.
  assert!((absorbed - A * A * d).abs() > 0.05, "looks like one trial per walk: {absorbed}");
}

/// Without walls every score is a visit probability: the ego scores exactly 1 (R11).
#[test]
fn ego_scores_one_without_walls() {
  let mut mr = build(3, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)], 14);
  mr.calculate(0).unwrap();
  assert_eq!(mr.get_node_score(0, 0).unwrap(), 1.0);
}

// ---------------------------------------------------------------------------
// Axioms
// ---------------------------------------------------------------------------

/// A1 (hard wall): the out-edges of B change no score but B's own — whether the graph is built
/// that way or they are added incrementally, including a cycle back through the ego's side.
#[test]
fn a1_hard_wall_out_edges_do_not_matter() {
  // 0 → 1 → B(2) ; 0 → 3 → 4; B's candidate out-edges: to 3, 4, 1, 0.
  let base = [(0, 1, 1.0), (1, 2, 1.0), (0, 3, 1.0), (3, 4, 1.0), (0, 2, -1.0)];
  let extra = [(2, 3, 5.0), (2, 4, 1.0), (2, 1, 2.0), (2, 0, 1.0)];
  let mut plain = build(5, &base, 20);
  plain.calculate(0).unwrap();
  let mut more = build(5, &apply(&base, &extra), 21);
  more.calculate(0).unwrap();
  let mut inc = build(5, &base, 22);
  inc.calculate(0).unwrap();
  for &(s, d, w) in &extra {
    inc.set_edge(s, d, w).unwrap();
  }
  for node in [0, 1, 3, 4] {
    for (label, other) in [("fresh", &more), ("incremental", &inc)] {
      let z = z_two(credits(&plain, 0, node), credits(other, 0, node));
      assert!(z.abs() < Z_FAIL, "A1 {label}: node {node} z = {z:.1}");
    }
  }
}

/// A3: a node with support independent of the wall (and nothing absorbed behind it) keeps its
/// baseline score.
#[test]
fn a3_no_collateral_damage() {
  let base = [(0, 1, 1.0), (1, 2, 1.0), (0, 3, 1.0), (3, 4, 1.0)];
  let mut no_wall = build(5, &base, 30);
  no_wall.calculate(0).unwrap();
  let mut wall = build(5, &apply(&base, &[(0, 2, -1.0)]), 31);
  wall.calculate(0).unwrap();
  for node in [3, 4] {
    let z = z_two(credits(&no_wall, 0, node), credits(&wall, 0, node));
    assert!(z.abs() < Z_FAIL, "A3: node {node} z = {z:.1}");
    assert!(wall.get_node_score(0, node).unwrap() > 0.0);
  }
}

/// A6: walls never raise a score above the wall-free graph (one-sided), with discredit.
#[test]
fn a6_walls_never_raise_scores() {
  let base = [
    (0, 1, 1.0),
    (1, 2, 1.0),
    (2, 3, 1.0),
    (0, 4, 1.0),
    (4, 3, 1.0),
    (3, 0, 1.0),
    (4, 5, 1.0),
  ];
  let mut no_wall = build(6, &base, 40);
  no_wall.discredit = 0.5;
  no_wall.calculate(0).unwrap();
  let mut walls = build(6, &apply(&base, &[(0, 2, -0.7), (0, 5, -1.0)]), 41);
  walls.discredit = 0.5;
  walls.calculate(0).unwrap();
  for node in 0..6 {
    let (a, b) = (
      no_wall.get_node_score(0, node).unwrap_or(0.0),
      walls.get_node_score(0, node).unwrap_or(0.0),
    );
    let se = (2.0 / W as f64).sqrt();
    assert!(b <= a + 5.0 * se, "A6: node {node} rose from {a} to {b}");
  }
}

/// A9: a wall change of ego 0 leaves every other frame bit-identical and marks only 0 dirty.
#[test]
fn a9_wall_change_touches_only_the_owner() {
  let edges = [(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0), (1, 3, 1.0), (3, 2, 1.0)];
  let mut mr = build(4, &edges, 50);
  for e in [0, 1, 3] {
    mr.calculate(e).unwrap();
  }
  mr.take_dirty_egos();
  let before: Vec<_> = [1, 3].iter().map(|&e| mr.get_all_scores(e, None).unwrap()).collect();
  mr.set_edge(0, 2, -0.6).unwrap();
  mr.set_edge(0, 2, -1.0).unwrap();
  mr.set_edge(0, 3, -0.3).unwrap();
  mr.set_edge(0, 2, 0.0).unwrap();
  assert_eq!(mr.take_dirty_egos(), vec![0]);
  let after: Vec<_> = [1, 3].iter().map(|&e| mr.get_all_scores(e, None).unwrap()).collect();
  assert_eq!(before, after, "other frames changed");
}

// ---------------------------------------------------------------------------
// Incremental maintenance vs fresh generation
// ---------------------------------------------------------------------------

/// Graph with a cycle through the future wall 3 and a second wall candidate 5 behind it.
fn web() -> Vec<Edge> {
  vec![
    (0, 1, 1.0),
    (0, 2, 2.0),
    (1, 3, 1.0),
    (2, 3, 1.0),
    (2, 4, 1.0),
    (3, 4, 1.0),
    (3, 1, 1.0),
    (4, 5, 1.0),
    (5, 3, 1.0),
    (4, 0, 0.5),
  ]
}

#[test]
fn inc_add_walls() {
  for lambda in [0.0, 0.6] {
    compare("add hard wall", 6, &web(), &[0], &[(0, 3, -1.0)], discredit(lambda));
    compare("add soft wall", 6, &web(), &[0], &[(0, 3, -0.4)], discredit(lambda));
    compare("add two walls", 6, &web(), &[0], &[(0, 3, -0.5), (0, 5, -0.7)], discredit(lambda));
  }
}

#[test]
fn inc_change_wall_strength() {
  let mut init = web();
  init.push((0, 3, -0.3));
  for lambda in [0.0, 0.6] {
    compare("strengthen 0.3 → 0.8", 6, &init, &[0], &[(0, 3, -0.8)], discredit(lambda));
    compare("strengthen 0.3 → 1", 6, &init, &[0], &[(0, 3, -1.0)], discredit(lambda));
    compare("weaken 0.3 → 0.1", 6, &init, &[0], &[(0, 3, -0.1)], discredit(lambda));
    compare("remove", 6, &init, &[0], &[(0, 3, 0.0)], discredit(lambda));
    compare("|w| above 1", 6, &init, &[0], &[(0, 3, -2.0), (0, 3, -3.0)], discredit(lambda));
    compare(
      "up, down, up",
      6,
      &init,
      &[0],
      &[(0, 3, -0.9), (0, 3, -0.2), (0, 3, -0.6)],
      discredit(lambda),
    );
  }
  let mut hard = web();
  hard.push((0, 3, -1.0));
  compare("hard → soft", 6, &hard, &[0], &[(0, 3, -0.5)], discredit(0.6));
  compare("hard → removed", 6, &hard, &[0], &[(0, 3, 0.0)], discredit(0.6));
}

#[test]
fn inc_sign_transitions() {
  // 0 → 2 is trust in `web`; replace it by a wall and back.
  compare("trust → wall", 6, &web(), &[0, 1], &[(0, 2, -0.7)], discredit(0.6));
  let mut walled = web();
  walled.retain(|&(s, d, _)| !(s == 0 && d == 2));
  walled.push((0, 2, -0.7));
  compare("wall → trust", 6, &walled, &[0, 1], &[(0, 2, 2.0)], discredit(0.6));
}

#[test]
fn inc_positive_edges_around_walls() {
  let mut init = web();
  init.push((0, 3, -0.6));
  init.push((0, 5, -1.0));
  compare("add edge at the wall", 6, &init, &[0], &[(3, 2, 2.0)], discredit(0.6));
  compare("delete edge at the wall", 6, &init, &[0], &[(3, 4, 0.0)], discredit(0.6));
  compare("add edge into the wall", 6, &init, &[0], &[(1, 5, 1.0)], discredit(0.6));
  compare("delete edge into the wall", 6, &init, &[0], &[(1, 3, 0.0)], discredit(0.6));
  compare("reweight behind the wall", 6, &init, &[0], &[(4, 5, 4.0)], discredit(0.6));
  compare(
    "mixed",
    6,
    &init,
    &[0],
    &[(3, 2, 1.0), (0, 3, -0.2), (5, 0, 1.0), (0, 5, 0.0), (2, 4, 0.0)],
    discredit(0.6),
  );
}

#[test]
fn inc_multiple_egos_with_walls() {
  let mut init = web();
  init.push((0, 3, -0.5));
  compare(
    "egos 0, 1, 2 with their own walls",
    6,
    &init,
    &[0, 1, 2],
    &[(1, 4, -0.8), (2, 5, -1.0), (0, 3, -0.9), (3, 4, 0.0), (1, 4, 0.0)],
    discredit(0.6),
  );
}

#[test]
fn inc_voucher_radius() {
  let voucher = |mr: &mut MeritRank| {
    mr.discredit = 0.6;
    mr.blame_radius = BlameRadius::Voucher;
  };
  compare("voucher radius", 6, &web(), &[0], &[(0, 3, -0.6), (0, 5, -1.0), (3, 2, 1.0)], voucher);
}

/// Tiny legitimate blame survives: with γ = 1e-15 the voucher's blame per walk is 1e-15, far below
/// any "rounding residue" threshold; weakening the wall so some walks survive must not wipe the
/// blame of the walks that stay absorbed (entries go away only with their last contributor).
#[test]
fn tiny_blame_is_not_dropped() {
  let mut mr = build(3, &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, -1.0)], 60);
  mr.blame_decay = 1e-15;
  mr.discredit = 1e17;
  mr.calculate(0).unwrap();
  let before = mr.blame_of(0, 1);
  assert!(before > 0.0);
  mr.set_edge(0, 2, -0.5).unwrap();
  let after = mr.blame_of(0, 1);
  assert!(after > 0.0, "tiny blame dropped");
  // About half of the absorbed walks stay absorbed.
  let ratio = after / before;
  assert!((ratio - 0.5).abs() < 0.05, "ratio {ratio}");
  assert!(mr.get_node_score(0, 1).unwrap() < 0.0);
  mr.verify().unwrap();
}

/// Evicting egos releases their visit-index memory: calculating and evicting many isolated egos
/// one after another keeps the index at the size of one frame.
#[test]
fn eviction_releases_visit_index_memory() {
  let mut mr = MeritRank::new(Graph::new(), 2_000);
  for _ in 0..60 {
    mr.get_new_nodeid();
  }
  mr.calculate(0).unwrap();
  let one = mr.visits_capacity();
  mr.clear_ego(0).unwrap();
  for ego in 1..60 {
    mr.calculate(ego).unwrap();
    mr.clear_ego(ego).unwrap();
  }
  mr.calculate(0).unwrap();
  assert!(
    mr.visits_capacity() <= 2 * one,
    "visit index grew from {} to {}",
    one,
    mr.visits_capacity()
  );
}
