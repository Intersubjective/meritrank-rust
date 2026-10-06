//! D14: one frame arithmetic for every path (JOURNAL.md D14).
//!
//! Bitwise checks (`f64::to_bits`, canonical counters) that the on-demand sampler, the resident
//! calculation and the incremental repair cannot drift apart:
//! - T1 `sample_frame` == `calculate_seeded` (same seed), and an `n`-walk sample is the first
//!   `n` walks of a larger frame;
//! - T2 counters maintained incrementally == counters recounted from the stored walks, after any
//!   sequence of writes;
//! - T3 a copy of a resident frame scores every node exactly as the frame does;
//! - T4 a repaired frame agrees in distribution with fresh samples on the final graph;
//! - T5 golden hash of a sample (a deliberate change of generation or accounting must update it);
//! - T6 sampling leaves the walk storage, the counters, the dirty set and the resident stream
//!   untouched;
//! - T7 independent oracles of a walk's contribution (Prefix, Voucher, decay 0 / 1, underflow);
//! - mutations: every effective change is reported, with its TV when tracking is on.

use meritrank_core::{
  depth_weight, edge_change_tv, walk_contribution, BlameRadius, Contribution, FrameCounters,
  FrameSample, Graph, MeritRank, NodeId, RandomWalk, Weight,
};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Params {
  alpha:     Weight,
  discredit: Weight,
  decay:     Weight,
  radius:    BlameRadius,
}

const PARAMS: &[Params] = &[
  Params { alpha: 0.85, discredit: 0.0, decay: 0.8, radius: BlameRadius::Prefix },
  Params { alpha: 0.85, discredit: 1.5, decay: 0.8, radius: BlameRadius::Prefix },
  Params { alpha: 0.5, discredit: 1.0, decay: 0.0, radius: BlameRadius::Prefix },
  Params { alpha: 0.95, discredit: 2.0, decay: 1.0, radius: BlameRadius::Prefix },
  Params { alpha: 0.85, discredit: 1.0, decay: 0.8, radius: BlameRadius::Voucher },
  Params { alpha: 0.85, discredit: 1.0, decay: 1e-200, radius: BlameRadius::Prefix },
];

fn new_rank(
  n: usize,
  walks: usize,
  p: Params,
) -> MeritRank {
  let mut mr = MeritRank::new(Graph::new(), walks);
  mr.alpha = p.alpha;
  mr.discredit = p.discredit;
  mr.blame_decay = p.decay;
  mr.blame_radius = p.radius;
  mr.reseed(7);
  for _ in 0..n {
    mr.get_new_nodeid();
  }
  mr
}

/// A random graph: dense short-range ring neighbourhoods, a few long-range edges, some dead ends
/// and walls of the first egos (strengths in (0, 1] and above).
fn random_graph(
  seed: u64,
  n: usize,
  walks: usize,
  p: Params,
) -> MeritRank {
  let mut rng = StdRng::seed_from_u64(seed);
  let mut mr = new_rank(n, walks, p);
  for src in 0..n {
    if src % 7 == 6 {
      continue; // dead end
    }
    for k in 1..=3 {
      let dst = (src + k) % n;
      mr.set_edge(src, dst, rng.random_range(0.1..3.0)).unwrap();
    }
    if rng.random::<f64>() < 0.3 {
      let dst = rng.random_range(0..n);
      if dst != src {
        mr.set_edge(src, dst, rng.random_range(0.1..1.0)).unwrap();
      }
    }
  }
  for ego in 0..3 {
    let wall = (ego + 2 + seed as usize) % n;
    if wall != ego {
      let strength = [0.3, 1.0, 2.5][ego % 3];
      mr.set_edge(ego, wall, -strength).unwrap();
    }
  }
  mr
}

fn bits(scores: &[(NodeId, Weight)]) -> Vec<(NodeId, u64)> {
  scores.iter().map(|(n, s)| (*n, s.to_bits())).collect()
}

fn frame_scores_bits(
  mr: &MeritRank,
  ego: NodeId,
  nodes: &[NodeId],
) -> Vec<(NodeId, u64)> {
  nodes
    .iter()
    .map(|&n| (n, mr.get_node_score(ego, n).unwrap().to_bits()))
    .collect()
}

fn walk(
  nodes: &[NodeId],
  absorbed: bool,
) -> RandomWalk {
  let mut w = RandomWalk::from_nodes(nodes.to_vec());
  w.absorbed = absorbed;
  w
}

// ---------------------------------------------------------------------------
// T7: independent oracles of a walk's contribution
// ---------------------------------------------------------------------------

#[test]
fn t7_depth_weight_is_repeated_multiplication() {
  for decay in [0.0, 0.3, 0.8, 1.0, 1e-200] {
    let mut w = 1.0f64;
    for k in 0..50u32 {
      assert_eq!(depth_weight(decay, k).to_bits(), w.to_bits(), "decay {decay} k {k}");
      w *= decay;
    }
  }
}

#[test]
fn t7_unabsorbed_walk_credits_its_distinct_nodes() {
  let c = walk_contribution(&walk(&[4, 1, 2, 1, 9], false), BlameRadius::Prefix, 0.8);
  assert_eq!(
    c,
    Contribution { absorbed: false, credited: vec![1, 2, 4, 9], blamed: vec![] }
  );
}

#[test]
fn t7_prefix_blames_each_node_at_its_nearest_depth() {
  // ego 0; the wall 3 is the absorbing arrival (depth 0); 1 is nearest at depth 1 (its earlier
  // visit at depth 3 does not count); 2 at depth 2; the ego is never blamed.
  let c = walk_contribution(&walk(&[0, 1, 2, 1, 3], true), BlameRadius::Prefix, 0.5);
  assert_eq!(
    c,
    Contribution { absorbed: true, credited: vec![], blamed: vec![(1, 1), (2, 2), (3, 0)] }
  );
  // A return to the ego inside the prefix is still not blamed.
  let c = walk_contribution(&walk(&[0, 5, 0, 6, 3], true), BlameRadius::Prefix, 0.5);
  assert_eq!(c.blamed, vec![(3, 0), (5, 3), (6, 1)]);
}

#[test]
fn t7_prefix_decay_zero_blames_the_wall_only_and_one_blames_all() {
  let w = walk(&[0, 1, 2, 3], true);
  assert_eq!(walk_contribution(&w, BlameRadius::Prefix, 0.0).blamed, vec![(3, 0)]);
  assert_eq!(
    walk_contribution(&w, BlameRadius::Prefix, 1.0).blamed,
    vec![(1, 2), (2, 1), (3, 0)]
  );
}

#[test]
fn t7_prefix_underflow_drops_zero_weights() {
  // 1e-200² underflows to 0: only depths 0 and 1 have positive weight.
  let c = walk_contribution(&walk(&[0, 1, 2, 3], true), BlameRadius::Prefix, 1e-200);
  assert_eq!(c.blamed, vec![(2, 1), (3, 0)]);
}

#[test]
fn t7_voucher_blames_the_wall_and_the_node_before() {
  let c = walk_contribution(&walk(&[0, 1, 2, 3], true), BlameRadius::Voucher, 0.8);
  assert_eq!(c.blamed, vec![(2, 0), (3, 0)]);
  // The node before the wall is the ego: only the wall.
  let c = walk_contribution(&walk(&[0, 3], true), BlameRadius::Voucher, 0.8);
  assert_eq!(c.blamed, vec![(3, 0)]);
}

#[test]
fn t7_blame_sum_ascending_depths() {
  let mut c = FrameCounters::new();
  // Walks blaming node 5 at depths 0, 5, 3: Σ = 1 + γ^3 + γ^5, summed in ascending depth.
  for blamed in [vec![(5, 0)], vec![(5, 5)], vec![(5, 3)]] {
    c.apply(&Contribution { absorbed: true, credited: vec![], blamed }, true).unwrap();
  }
  let g = 0.8f64;
  let expected = 1.0 * depth_weight(g, 0) + 1.0 * depth_weight(g, 3) + 1.0 * depth_weight(g, 5);
  assert_eq!(c.blame_sum(5, g).to_bits(), expected.to_bits());
  assert_eq!(c.blame_walks(5), 3);
  assert_eq!(c.blame_hist(5).unwrap().counts, vec![1, 0, 0, 1, 0, 1]);
}

#[test]
fn t7_score_formula() {
  assert_eq!(
    meritrank_core::score(30, 2.5, 1.5, 100).to_bits(),
    ((30.0 - 1.5 * 2.5) / 100.0f64).to_bits()
  );
}

// ---------------------------------------------------------------------------
// Canonical counters
// ---------------------------------------------------------------------------

#[test]
fn counters_add_then_remove_is_empty() {
  let mut c = FrameCounters::new();
  let a = Contribution { absorbed: false, credited: vec![1, 2, 3], blamed: vec![] };
  let b = Contribution { absorbed: true, credited: vec![], blamed: vec![(2, 0), (4, 3)] };
  c.apply(&a, true).unwrap();
  c.apply(&b, true).unwrap();
  c.apply(&a, false).unwrap();
  c.apply(&b, false).unwrap();
  assert!(c.is_empty());
  assert_eq!(c, FrameCounters::new());
  assert!(c.nodes().is_empty());
}

#[test]
fn counters_remove_unknown_is_an_error_and_changes_nothing() {
  let mut c = FrameCounters::new();
  c.apply(&Contribution { absorbed: false, credited: vec![1], blamed: vec![] }, true).unwrap();
  let before = c.clone();
  let r = c.apply(&Contribution { absorbed: false, credited: vec![1, 2], blamed: vec![] }, false);
  assert!(r.is_err());
  assert_eq!(c, before);
}

#[test]
fn t8_counters_overflow_is_an_error() {
  let mut c = FrameCounters::with_credits_for_test(1, u32::MAX);
  let before = c.clone();
  let r = c.apply(&Contribution { absorbed: false, credited: vec![0, 1], blamed: vec![] }, true);
  assert!(r.is_err());
  assert_eq!(c, before, "a failed apply must not change the counters");
}

// ---------------------------------------------------------------------------
// T1: the sampler is the calculation
// ---------------------------------------------------------------------------

#[test]
fn t1_sample_equals_calculation_bitwise() {
  const W: usize = 300;
  for (pi, &p) in PARAMS.iter().enumerate() {
    for seed in 0..6u64 {
      let base = random_graph(seed, 40, W, p);
      for ego in [0usize, 1, 5, 13] {
        let walk_seed = 1_000 * seed + ego as u64;
        let sample = base.sample_frame(ego, W, walk_seed).unwrap();
        let mut mr = base.clone();
        mr.calculate_seeded(ego, walk_seed).unwrap();
        let ctx = format!("params {pi} seed {seed} ego {ego}");

        assert_eq!(sample.n, W, "{ctx}");
        assert_eq!(&sample.counters, mr.frame_counters(ego).unwrap(), "{ctx}: counters");
        assert_eq!(Some(&sample), mr.frame_sample(ego).as_ref(), "{ctx}: frame copy");

        let footprint: Vec<NodeId> = mr
          .ego_walks(ego)
          .unwrap()
          .iter()
          .flat_map(|w| w.get_nodes().iter().copied())
          .collect::<std::collections::BTreeSet<_>>()
          .into_iter()
          .collect();
        let sampled: Vec<NodeId> = sample.visits.iter().map(|(n, _)| *n).collect();
        assert_eq!(sampled, footprint, "{ctx}: footprint");

        let nodes = sample.counters.nodes();
        assert_eq!(
          bits(&sample.scores(p.discredit, p.decay)),
          frame_scores_bits(&mr, ego, &nodes),
          "{ctx}: scores"
        );
      }
    }
  }
}

#[test]
fn t1_smaller_sample_is_the_first_walks_of_a_frame() {
  let p = PARAMS[1];
  const W: usize = 500;
  for seed in 0..4u64 {
    let mut mr = random_graph(seed, 30, W, p);
    mr.calculate_seeded(2, 99 + seed).unwrap();
    let walks = mr.ego_walks(2).unwrap();
    for n in [1usize, 37, 250] {
      let sample = mr.sample_frame(2, n, 99 + seed).unwrap();
      let prefix =
        FrameCounters::from_walks(walks[..n].iter().copied(), p.radius, p.decay).unwrap();
      assert_eq!(sample.counters, prefix, "seed {seed} n {n}");
      let arrivals: u64 = walks[..n].iter().map(|w| w.len() as u64).sum();
      assert_eq!(sample.visits.iter().map(|(_, v)| *v).sum::<u64>(), arrivals);
    }
  }
}

/// A frame calculated into a reused block (after other egos were calculated and evicted) is the
/// same as the sample: nothing of the block's previous content leaks.
#[test]
fn t1_reused_block_equals_sample() {
  let p = PARAMS[1];
  let mut mr = random_graph(9, 30, 250, p);
  mr.calculate_seeded(5, 1).unwrap();
  mr.calculate_seeded(6, 2).unwrap();
  mr.clear_ego(5).unwrap();
  mr.calculate_seeded(7, 3).unwrap(); // takes 5's block
  mr.calculate_seeded(6, 4).unwrap(); // recalculated in place
  for (ego, seed) in [(7usize, 3u64), (6, 4)] {
    let sample = mr.sample_frame(ego, 250, seed).unwrap();
    assert_eq!(Some(&sample), mr.frame_sample(ego).as_ref(), "ego {ego}");
  }
}

#[test]
fn t1_seeded_calculation_ignores_the_resident_stream() {
  let p = PARAMS[0];
  let mut a = random_graph(3, 25, 200, p);
  let mut b = a.clone();
  b.reseed(123_456);
  a.calculate_seeded(4, 5).unwrap();
  b.calculate_seeded(4, 5).unwrap();
  assert_eq!(a.frame_counters(4), b.frame_counters(4));
}

// ---------------------------------------------------------------------------
// T2 / T3: incremental counters == recount; frame copies score as the frame
// ---------------------------------------------------------------------------

#[test]
fn t2_incremental_counters_equal_recount_after_any_writes() {
  const W: usize = 120;
  for (pi, &p) in PARAMS.iter().enumerate() {
    for seed in 0..4u64 {
      let n = 24;
      let mut mr = random_graph(seed, n, W, p);
      let mut rng = StdRng::seed_from_u64(500 + seed);
      for ego in [0usize, 1, 2, 9] {
        mr.calculate_seeded(ego, seed * 31 + ego as u64).unwrap();
      }
      for step in 0..60 {
        let src = rng.random_range(0..n);
        let dst = (src + rng.random_range(1..n)) % n;
        let w = match rng.random_range(0..7) {
          0 => 0.0,                                  // delete (or no-op)
          1 => -rng.random_range(0.05..1.5),         // wall up / down / sign transition
          2 => rng.random_range(1e-7..1e-5),          // near-epsilon weight
          _ => rng.random_range(0.1..4.0),           // add / reweight
        };
        mr.set_edge(src, dst, w).unwrap();
        if step % 13 == 5 {
          // Delete every out-edge of a node (it becomes a dead end).
          let outs: Vec<NodeId> =
            mr.graph.get_node_data(src).unwrap().get_outgoing_edges().map(|(d, _)| d).collect();
          for d in outs {
            mr.set_edge(src, d, 0.0).unwrap();
          }
        }
        if step % 17 == 3 {
          mr.clear_ego(1).unwrap();
          mr.calculate_seeded(1, step as u64).unwrap();
        }
        for ego in mr.calculated_egos() {
          let ctx = format!("params {pi} seed {seed} step {step} ego {ego}");
          let kept = mr.frame_counters(ego).unwrap().clone();
          let recount = mr.recount_frame(ego).unwrap();
          assert_eq!(kept, recount, "{ctx}: counters");
          let nodes = kept.nodes();
          let a: Vec<u64> = nodes.iter().map(|&x| kept.score(x, p.discredit, p.decay, W).to_bits()).collect();
          let b: Vec<u64> =
            nodes.iter().map(|&x| recount.score(x, p.discredit, p.decay, W).to_bits()).collect();
          assert_eq!(a, b, "{ctx}: scores");
        }
      }
    }
  }
}

#[test]
fn t3_frame_copy_scores_as_the_frame() {
  for &p in PARAMS {
    let mut mr = random_graph(11, 30, 400, p);
    mr.calculate_seeded(0, 1).unwrap();
    mr.set_edge(3, 4, 0.0).unwrap();
    mr.set_edge(0, 7, -0.6).unwrap();
    mr.set_edge(5, 9, 2.0).unwrap();
    let copy = mr.frame_sample(0).unwrap();
    assert_eq!(copy.n, 400);
    for (node, _) in &copy.visits {
      assert_eq!(
        copy.score(*node, p.discredit, p.decay).to_bits(),
        mr.get_node_score(0, *node).unwrap().to_bits()
      );
    }
    let mut all = mr.get_all_scores(0, None).unwrap();
    all.sort_by_key(|(n, _)| *n);
    assert_eq!(bits(&copy.scores(p.discredit, p.decay)), bits(&all));
  }
}

#[test]
fn credits_of_matches_counters() {
  let p = PARAMS[0];
  let mut mr = random_graph(2, 20, 300, p);
  mr.calculate_seeded(0, 3).unwrap();
  let c = mr.frame_counters(0).unwrap().clone();
  for node in c.nodes() {
    assert_eq!(mr.credits_of(0, node), c.credits(node));
  }
  assert_eq!(mr.credits_of(0, 10_000), 0);
  assert_eq!(mr.credits_of(19, 1), 0);
}

// ---------------------------------------------------------------------------
// T4: repaired frames agree in distribution with fresh samples
// ---------------------------------------------------------------------------

/// After a series of writes, the incrementally repaired frame and fresh samples on the final
/// graph estimate the same scores: per-node credits are means of Bernoulli variables, so
/// |Δ| <= 5·sqrt(2·p(1−p)/W) except with negligible probability (checked over all nodes of a
/// fixed set of seeds).
#[test]
fn t4_repaired_frame_agrees_with_fresh_samples() {
  const W: usize = 20_000;
  let p = Params { alpha: 0.85, discredit: 1.0, decay: 0.8, radius: BlameRadius::Prefix };
  for seed in 0..3u64 {
    let mut mr = random_graph(seed, 30, W, p);
    mr.calculate_seeded(0, seed).unwrap();
    let mut rng = StdRng::seed_from_u64(seed + 77);
    for step in 0..25 {
      let src = if step % 5 == 0 { 0 } else { rng.random_range(0..30) };
      let dst = (src + rng.random_range(1..30)) % 30;
      // Every fifth write is a wall of the frame's own ego (raised, lowered, removed).
      let w = if step % 5 == 0 {
        -rng.random_range(0.0..1.5)
      } else if rng.random::<f64>() < 0.3 {
        0.0
      } else {
        rng.random_range(0.1..3.0)
      };
      mr.set_edge(src, dst, w).unwrap();
    }
    let fresh = mr.sample_frame(0, W, 10_000 + seed).unwrap();
    let kept = mr.frame_counters(0).unwrap();
    for node in 0..30 {
      let a = kept.credits(node) as f64 / W as f64;
      let b = fresh.counters.credits(node) as f64 / W as f64;
      let q = (a + b) / 2.0;
      let se = (2.0 * q * (1.0 - q) / W as f64).sqrt().max(1e-9);
      assert!((a - b).abs() <= 5.0 * se, "seed {seed} node {node}: {a} vs {b}");
      // Blame per walk lies in [0, 1]: the same bound holds for its mean.
      let (x, y) = (kept.blame_sum(node, p.decay) / W as f64, fresh.counters.blame_sum(node, p.decay) / W as f64);
      let q = ((x + y) / 2.0).clamp(0.0, 1.0);
      let se = (2.0 * q * (1.0 - q) / W as f64).sqrt().max(1e-9);
      assert!((x - y).abs() <= 5.0 * se, "seed {seed} node {node} blame: {x} vs {y}");
    }
  }
}

// ---------------------------------------------------------------------------
// T5: golden hash
// ---------------------------------------------------------------------------

fn fnv64(bytes: &[u8]) -> u64 {
  bytes.iter().fold(0xCBF2_9CE4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01B3))
}

/// The logical content of a fixed sample. A change of walk generation, accounting or the seed
/// policy changes it: update GOLDEN deliberately and record why in JOURNAL.md.
#[test]
fn t5_golden_sample() {
  const GOLDEN: u64 = 0x0bfa_1199_1414_1729; // JOURNAL.md D14
  let p = PARAMS[1];
  let mr = random_graph(42, 50, 1_000, p);
  let sample = mr.sample_frame(3, 1_000, 0xD14).unwrap();
  let h = fnv64(&sample.stable_bytes());
  assert_eq!(h, GOLDEN, "golden sample changed: {h:#x}");
}

// ---------------------------------------------------------------------------
// T6: sampling is storage-neutral
// ---------------------------------------------------------------------------

#[test]
fn t6_sampling_is_storage_neutral() {
  let p = PARAMS[1];
  for warm in [false, true] {
    let mut mr = random_graph(5, 30, 200, p);
    mr.calculate_seeded(0, 1).unwrap();
    mr.calculate_seeded(4, 2).unwrap();
    if warm {
      let _ = mr.sample_frame(9, 50, 3).unwrap(); // builds lazy distributions
    }
    let _ = mr.take_dirty_egos();
    let _ = mr.take_mutations();
    let before = mr.clone();

    let _ = mr.sample_frame(9, 500, 4).unwrap();
    let _ = mr.sample_frame(0, 500, 5).unwrap(); // a resident ego too

    assert_eq!(mr.allocated_walks(), before.allocated_walks());
    for ego in before.calculated_egos() {
      let a: Vec<Vec<NodeId>> =
        mr.ego_walks(ego).unwrap().iter().map(|w| w.get_nodes().to_vec()).collect();
      let b: Vec<Vec<NodeId>> =
        before.ego_walks(ego).unwrap().iter().map(|w| w.get_nodes().to_vec()).collect();
      assert_eq!(a, b, "stored walks of {ego}");
    }
    assert_eq!(mr.visits_capacity(), before.visits_capacity());
    assert_eq!(mr.calculated_egos(), before.calculated_egos());
    for ego in mr.calculated_egos() {
      assert_eq!(mr.frame_counters(ego), before.frame_counters(ego));
    }
    assert!(mr.take_dirty_egos().is_empty());
    assert!(mr.take_mutations().is_empty());
    // The resident stream did not move: the same unseeded calculation follows on both — also
    // after a seeded calculation, which uses its own stream.
    let mut a = mr.clone();
    let mut b = before.clone();
    a.calculate_seeded(20, 99).unwrap();
    a.calculate(12).unwrap();
    b.calculate(12).unwrap();
    assert_eq!(a.frame_counters(12), b.frame_counters(12));
  }
}

// ---------------------------------------------------------------------------
// Mutations
// ---------------------------------------------------------------------------

fn sources(m: &meritrank_core::Mutations) -> Vec<NodeId> {
  m.sources.iter().map(|s| s.src).collect()
}

#[test]
fn mutations_report_effective_changes_without_frames() {
  let mut mr = new_rank(6, 10, PARAMS[0]);
  mr.set_edge(0, 1, 1.0).unwrap();
  mr.set_edge(2, 3, 1.0).unwrap();
  let m = mr.take_mutations();
  assert_eq!(sources(&m), vec![0, 2], "recorded without any calculated ego");
  assert!(m.wall_owners.is_empty());
  assert!(mr.take_mutations().is_empty(), "taken once");

  mr.set_edge(0, 1, 1.0).unwrap(); // same weight: no change
  mr.set_edge(4, 5, 0.0).unwrap(); // deleting an absent edge: no change
  assert!(mr.take_mutations().is_empty());

  mr.set_edge(0, 1, 2.0).unwrap(); // reweight
  mr.set_edge(0, 2, 1.0).unwrap(); // second change of the same source: one entry
  assert_eq!(sources(&mr.take_mutations()), vec![0]);

  mr.set_edge(3, 4, -0.5).unwrap(); // wall
  let m = mr.take_mutations();
  assert!(m.sources.is_empty());
  assert_eq!(m.wall_owners, vec![3]);
  mr.set_edge(3, 4, -0.5).unwrap(); // same strength
  mr.set_edge(3, 4, -1.5).unwrap(); // strength capped at 1 both ways? 0.5 → 1.0: effective
  assert_eq!(mr.take_mutations().wall_owners, vec![3]);
  mr.set_edge(3, 4, -2.0).unwrap(); // 1.0 → 1.0: not effective
  assert!(mr.take_mutations().is_empty());

  mr.set_edge(0, 1, -1.0).unwrap(); // sign transition: a positive deletion and a wall
  let m = mr.take_mutations();
  assert_eq!(sources(&m), vec![0]);
  assert_eq!(m.wall_owners, vec![0]);
}

#[test]
fn mutations_report_tv_when_tracking() {
  let mut mr = new_rank(5, 10, PARAMS[0]);
  mr.set_edge(0, 1, 1.0).unwrap();
  let m = mr.take_mutations();
  assert_eq!(m.sources[0].tv, None, "no TV without tracking");

  mr.set_tv_tracking(true);
  mr.set_edge(0, 2, 1.0).unwrap(); // {1: 1} → {1: .5, 2: .5}
  let m = mr.take_mutations();
  assert_eq!(m.sources.len(), 1);
  assert!((m.sources[0].tv.unwrap() - 0.5).abs() < 1e-12);

  mr.set_edge(3, 4, 2.0).unwrap(); // a dead end gains an edge
  assert_eq!(mr.take_mutations().sources[0].tv, Some(1.0));
  mr.set_edge(3, 4, 0.0).unwrap(); // and loses it
  assert_eq!(mr.take_mutations().sources[0].tv, Some(1.0));

  // Two changes of one source in one operation: the net TV, from {.5, .5} to {1/8, 1/8, 2/8, 4/8}.
  mr.set_edge(0, 3, 2.0).unwrap();
  mr.set_edge(0, 4, 4.0).unwrap();
  let tv = mr.take_mutations().sources[0].tv.unwrap();
  assert!((tv - 0.75).abs() < 1e-12, "{tv}");

  // A proportional rescale of every out-edge (as VSIDS does) moves no probability: net TV 0.
  mr.set_edge(0, 1, 0.5).unwrap();
  mr.set_edge(0, 2, 0.5).unwrap();
  mr.set_edge(0, 3, 1.0).unwrap();
  mr.set_edge(0, 4, 2.0).unwrap();
  let m = mr.take_mutations();
  assert_eq!(m.sources.len(), 1);
  assert!(m.sources[0].tv.unwrap() < 1e-12, "{:?}", m);

  // An epsilon weight is a deletion; a wall replacing a positive edge is a positive deletion.
  mr.set_edge(0, 4, 1e-9).unwrap();
  let tv = mr.take_mutations().sources[0].tv.unwrap();
  // {1/8, 1/8, 2/8, 4/8} → {1/4, 1/4, 2/4}: TV = (1/8 + 1/8 + 2/8 + 4/8) / 2 = 1/2.
  assert!((tv - 0.5).abs() < 1e-12, "{tv}");
}

#[test]
fn edge_change_tv_formula() {
  assert!((edge_change_tv(1.0, 0.0, 1.0) - 0.5).abs() < 1e-12);
  assert_eq!(edge_change_tv(0.0, 0.0, 3.0), 1.0);
  assert_eq!(edge_change_tv(2.0, 2.0, 0.0), 1.0);
  // Reweight 1 → 3 among {1, 1}: before {.5, .5}, after {.75, .25}: TV .25.
  assert!((edge_change_tv(2.0, 1.0, 3.0) - 0.25).abs() < 1e-12);
  assert_eq!(edge_change_tv(4.0, 1.0, 1.0), 0.0);
}

/// Clearing or calculating frames is not a graph change.
#[test]
fn mutations_ignore_frame_lifecycle() {
  let mut mr = random_graph(1, 15, 50, PARAMS[0]);
  let _ = mr.take_mutations();
  mr.calculate_seeded(0, 1).unwrap();
  mr.clear_ego(0).unwrap();
  mr.calculate(3).unwrap();
  assert!(mr.take_mutations().is_empty());
}

#[allow(dead_code)]
fn _unused(_: &FrameSample) {}
