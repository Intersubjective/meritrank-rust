//! One implementation of a frame's arithmetic (D14), shared by every path that builds or
//! maintains a frame: `calculate` (resident frames), incremental repair (`set_edge`, walls) and
//! `sample_frame` (on-demand frames outside the walk storage).
//!
//! - `generate_walk_into` is the only generator of a fresh walk.
//! - `walk_contribution` is the only rule of what a walk credits and blames.
//! - `FrameCounters` is the only representation of a frame's counters. It is canonical: no
//!   zero-valued keys, blame as an integer histogram over the depth `k` (weight `γ^k`), so the
//!   counters maintained incrementally equal, bit for bit, the counters recounted from the walks.
//! - `score` is the only score formula.

use integer_hasher::{IntMap, IntSet};
use rand::Rng;

use crate::errors::{internal_fatal, MeritRankError};
use crate::graph::{Graph, NodeId, Weight};
use crate::random_walk::RandomWalk;
use crate::rank::BlameRadius;

/// Weight `γ^k` of a blame at depth `k`, by `k` repeated multiplications starting from 1.0 (the
/// same arithmetic the prefix walk uses).
pub fn depth_weight(
  decay: Weight,
  depth: u32,
) -> Weight {
  let mut w = 1.0;
  for _ in 0..depth {
    w *= decay;
  }
  w
}

/// What one walk contributes to its ego's frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contribution {
  pub absorbed: bool,
  /// Unabsorbed walk: every distinct node it visited (the ego included), sorted. Empty if
  /// absorbed.
  pub credited: Vec<NodeId>,
  /// Absorbed walk: every blamed node with its depth (steps from its visit nearest to the
  /// absorbing arrival), sorted by node. Nodes whose weight `γ^k` is 0 are not blamed. Empty if
  /// not absorbed.
  pub blamed:   Vec<(NodeId, u32)>,
}

/// The contribution of `walk` (R10–R13): credits for an unabsorbed walk; blame for an absorbed
/// one — Prefix: every distinct non-ego node of the walk at the depth of its visit nearest to the
/// wall; Voucher: the wall and the node before it (unless that is the ego), both at depth 0.
pub fn walk_contribution(
  walk: &RandomWalk,
  radius: BlameRadius,
  decay: Weight,
) -> Contribution {
  let nodes = walk.get_nodes();
  if !walk.absorbed {
    let mut credited = nodes.to_vec();
    credited.sort_unstable();
    credited.dedup();
    return Contribution { absorbed: false, credited, blamed: vec![] };
  }
  let mut blamed = vec![];
  if nodes.len() >= 2 {
    let ego = nodes[0];
    let last = nodes.len() - 1;
    match radius {
      BlameRadius::Voucher => {
        let wall = nodes[last];
        blamed.push((wall, 0));
        let voucher = nodes[last - 1];
        if voucher != ego && voucher != wall {
          blamed.push((voucher, 0));
        }
      },
      BlameRadius::Prefix => {
        let mut seen: IntSet<NodeId> = IntSet::default();
        let mut b = 1.0;
        for i in (1..=last).rev() {
          let node = nodes[i];
          // `seen` takes the node even when its weight is 0: only its nearest visit counts.
          if node != ego && seen.insert(node) && b > 0.0 {
            blamed.push((node, (last - i) as u32));
          }
          b *= decay;
        }
      },
    }
  }
  blamed.sort_unstable();
  Contribution { absorbed: true, credited: vec![], blamed }
}

/// Blame of one node in one frame: how many absorbed walks blamed it at each depth.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlameHist {
  /// `counts[k]` = walks that blamed the node at depth `k`; no trailing zeros.
  pub counts: Vec<u32>,
}

impl BlameHist {
  fn walks(&self) -> u32 {
    self.counts.iter().sum()
  }

  /// `Σ_k counts[k] · γ^k` in ascending `k`, the weights by repeated multiplication.
  fn sum(
    &self,
    decay: Weight,
  ) -> Weight {
    let mut sum = 0.0;
    let mut w = 1.0;
    for &c in &self.counts {
      if c > 0 {
        sum += c as Weight * w;
      }
      w *= decay;
    }
    sum
  }
}

/// A frame's counters in canonical form (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameCounters {
  credits: IntMap<NodeId, u32>,
  blame:   IntMap<NodeId, BlameHist>,
}

fn overflow() -> MeritRankError {
  MeritRankError::InternalFatalError(Some(internal_fatal::FRAME_COUNTER_OVERFLOW))
}

fn underflow() -> MeritRankError {
  MeritRankError::InternalFatalError(Some(internal_fatal::FRAME_COUNTER_UNDERFLOW))
}

impl FrameCounters {
  pub fn new() -> Self {
    Self::default()
  }

  /// Adds (`add = true`) or removes a walk's contribution. Errors on overflow or on removing what
  /// was never added; the counters are unchanged then.
  pub fn apply(
    &mut self,
    c: &Contribution,
    add: bool,
  ) -> Result<(), MeritRankError> {
    // Check everything first, so that a failure changes nothing.
    for node in &c.credited {
      let have = self.credits.get(node).copied().unwrap_or(0);
      if add && have == u32::MAX {
        return Err(overflow());
      }
      if !add && have == 0 {
        return Err(underflow());
      }
    }
    for &(node, depth) in &c.blamed {
      let k = depth as usize;
      let have = self.blame.get(&node).and_then(|h| h.counts.get(k)).copied().unwrap_or(0);
      if add && have == u32::MAX {
        return Err(overflow());
      }
      if !add && have == 0 {
        return Err(underflow());
      }
    }
    for &node in &c.credited {
      if add {
        *self.credits.entry(node).or_insert(0) += 1;
      } else if let Some(v) = self.credits.get_mut(&node) {
        *v -= 1;
        if *v == 0 {
          self.credits.remove(&node);
        }
      }
    }
    for &(node, depth) in &c.blamed {
      let k = depth as usize;
      if add {
        let h = self.blame.entry(node).or_default();
        if h.counts.len() <= k {
          h.counts.resize(k + 1, 0);
        }
        h.counts[k] += 1;
      } else if let Some(h) = self.blame.get_mut(&node) {
        h.counts[k] -= 1;
        while h.counts.last() == Some(&0) {
          h.counts.pop();
        }
        if h.counts.is_empty() {
          self.blame.remove(&node);
        }
      }
    }
    Ok(())
  }

  /// Counters recounted from scratch from the given walks (empty walks contribute nothing).
  pub fn from_walks<'a, I>(
    walks: I,
    radius: BlameRadius,
    decay: Weight,
  ) -> Result<Self, MeritRankError>
  where
    I: IntoIterator<Item = &'a RandomWalk>,
  {
    let mut c = Self::new();
    for walk in walks {
      if !walk.is_empty() {
        c.apply(&walk_contribution(walk, radius, decay), true)?;
      }
    }
    Ok(c)
  }

  /// Unabsorbed walks that visited `node`.
  pub fn credits(
    &self,
    node: NodeId,
  ) -> u32 {
    self.credits.get(&node).copied().unwrap_or(0)
  }

  /// Absorbed walks that blame `node`.
  pub fn blame_walks(
    &self,
    node: NodeId,
  ) -> u32 {
    self.blame.get(&node).map_or(0, BlameHist::walks)
  }

  /// `Σ_k counts[k] · γ^k`, summed in ascending `k`.
  pub fn blame_sum(
    &self,
    node: NodeId,
    decay: Weight,
  ) -> Weight {
    self.blame.get(&node).map_or(0.0, |h| h.sum(decay))
  }

  pub fn blame_hist(
    &self,
    node: NodeId,
  ) -> Option<&BlameHist> {
    self.blame.get(&node)
  }

  /// Every node with credits or blame, sorted.
  pub fn nodes(&self) -> Vec<NodeId> {
    let mut v: Vec<NodeId> = self.credits.keys().chain(self.blame.keys()).copied().collect();
    v.sort_unstable();
    v.dedup();
    v
  }

  pub fn is_empty(&self) -> bool {
    self.credits.is_empty() && self.blame.is_empty()
  }

  /// Score of `node` over `n` walks.
  pub fn score(
    &self,
    node: NodeId,
    discredit: Weight,
    decay: Weight,
    n: usize,
  ) -> Weight {
    score(self.credits(node), self.blame_sum(node, decay), discredit, n)
  }

  /// Test support: a counter set to a given value (overflow tests).
  #[doc(hidden)]
  pub fn with_credits_for_test(
    node: NodeId,
    credits: u32,
  ) -> Self {
    let mut c = Self::default();
    c.credits.insert(node, credits);
    c
  }
}

/// `score_A(X) = (credits_X − λ·blame_X) / n` (R11) — the only score formula.
pub fn score(
  credits: u32,
  blame_sum: Weight,
  discredit: Weight,
  n: usize,
) -> Weight {
  (credits as Weight - discredit * blame_sum) / n as Weight
}

/// Generates one fresh walk of `ego` into `walk` (cleared first): the ego, then steps until the
/// walk stops, hits a dead end or is absorbed. The only generator of fresh walks.
pub fn generate_walk_into<R: Rng + ?Sized>(
  graph: &Graph,
  ego: NodeId,
  alpha: Weight,
  rng: &mut R,
  walk: &mut RandomWalk,
) -> Result<(), MeritRankError> {
  walk.clear();
  walk.push(ego)?;
  graph.continue_walk(walk, alpha, rng)
}

/// A frame of `n` walks taken outside the walk storage (on-demand), or a copy of a resident one.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameSample {
  pub ego:      NodeId,
  pub n:        usize,
  pub counters: FrameCounters,
  /// Footprint: every node any walk arrived at (the ego included), sorted, with the number of
  /// arrivals, repeats and absorbed walks included.
  pub visits:   Vec<(NodeId, u64)>,
}

impl FrameSample {
  /// A sample from `walks` (their counters and arrivals).
  pub fn from_walks<'a, I>(
    ego: NodeId,
    walks: I,
    radius: BlameRadius,
    decay: Weight,
  ) -> Result<Self, MeritRankError>
  where
    I: IntoIterator<Item = &'a RandomWalk>,
  {
    let mut acc = SampleAccumulator::new(ego);
    for walk in walks {
      if !walk.is_empty() {
        acc.add(walk, radius, decay)?;
      }
    }
    Ok(acc.finish())
  }

  /// Arrivals at `node` (0 if outside the footprint).
  pub fn visits_of(
    &self,
    node: NodeId,
  ) -> u64 {
    self.visits.binary_search_by_key(&node, |(n, _)| *n).map_or(0, |i| self.visits[i].1)
  }

  pub fn in_footprint(
    &self,
    node: NodeId,
  ) -> bool {
    self.visits.binary_search_by_key(&node, |(n, _)| *n).is_ok()
  }

  pub fn score(
    &self,
    node: NodeId,
    discredit: Weight,
    decay: Weight,
  ) -> Weight {
    self.counters.score(node, discredit, decay, self.n)
  }

  /// Every node with credits or blame and its score, sorted by node.
  pub fn scores(
    &self,
    discredit: Weight,
    decay: Weight,
  ) -> Vec<(NodeId, Weight)> {
    self
      .counters
      .nodes()
      .into_iter()
      .map(|node| (node, self.score(node, discredit, decay)))
      .collect()
  }

  /// A stable logical serialization (no map order, no capacities), for golden hashes.
  pub fn stable_bytes(&self) -> Vec<u8> {
    let mut out = vec![];
    let mut put = |x: u64| out.extend_from_slice(&x.to_le_bytes());
    put(self.ego as u64);
    put(self.n as u64);
    let mut nodes = self.counters.nodes();
    nodes.extend(self.visits.iter().map(|(n, _)| *n));
    nodes.sort_unstable();
    nodes.dedup();
    put(nodes.len() as u64);
    for node in nodes {
      put(node as u64);
      put(self.counters.credits(node) as u64);
      let counts = self.counters.blame_hist(node).map_or(&[][..], |h| &h.counts[..]);
      put(counts.len() as u64);
      for &c in counts {
        put(c as u64);
      }
      put(self.visits_of(node));
    }
    out
  }
}

/// Builds a `FrameSample` walk by walk.
pub(crate) struct SampleAccumulator {
  ego:      NodeId,
  n:        usize,
  counters: FrameCounters,
  visits:   IntMap<NodeId, u64>,
}

impl SampleAccumulator {
  pub(crate) fn new(ego: NodeId) -> Self {
    SampleAccumulator { ego, n: 0, counters: FrameCounters::new(), visits: IntMap::default() }
  }

  pub(crate) fn add(
    &mut self,
    walk: &RandomWalk,
    radius: BlameRadius,
    decay: Weight,
  ) -> Result<(), MeritRankError> {
    self.counters.apply(&walk_contribution(walk, radius, decay), true)?;
    for &node in walk.get_nodes() {
      let v = self.visits.entry(node).or_insert(0);
      *v = v.checked_add(1).ok_or(MeritRankError::InternalFatalError(Some(
        internal_fatal::FRAME_VISITS_OVERFLOW,
      )))?;
    }
    self.n += 1;
    Ok(())
  }

  pub(crate) fn finish(self) -> FrameSample {
    let mut visits: Vec<(NodeId, u64)> = self.visits.into_iter().collect();
    visits.sort_unstable();
    FrameSample { ego: self.ego, n: self.n, counters: self.counters, visits }
  }
}

/// An effective change of a node's positive out-edges in one operation.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceChange {
  pub src: NodeId,
  /// Total variation between the node's next-step distributions before and after (conditional
  /// on continuing; 1 when a dead end gains or loses its last edge), summed over the changes of
  /// the operation and capped at 1. `None` unless tracking is on.
  pub tv:  Option<Weight>,
}

/// What the graph changes since the last `take_mutations`, recorded whether or not any frame is
/// resident.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mutations {
  /// Sorted by node, one entry per node.
  pub sources:     Vec<SourceChange>,
  /// Owners of effective wall changes, sorted.
  pub wall_owners: Vec<NodeId>,
}

impl Mutations {
  pub fn is_empty(&self) -> bool {
    self.sources.is_empty() && self.wall_owners.is_empty()
  }
}

/// TV between a node's next-step distributions when one out-edge changes `w → w_new`, with
/// `sum` the positive weight sum before the change (see `SourceChange::tv`).
pub fn edge_change_tv(
  sum: Weight,
  w: Weight,
  w_new: Weight,
) -> Weight {
  let sum_new = sum - w + w_new;
  let before = sum > 0.0;
  let after = sum_new > 0.0;
  match (before, after) {
    (false, false) => 0.0,
    (true, false) | (false, true) => 1.0,
    (true, true) => {
      let others = (sum - w).max(0.0);
      let tv = 0.5 * ((1.0 / sum_new - 1.0 / sum).abs() * others + (w_new / sum_new - w / sum).abs());
      tv.min(1.0)
    },
  }
}

/// Total variation between two next-step distributions given by positive weights (a node with no
/// weight is a dead end: TV 1 against any non-dead end, 0 against another dead end).
pub fn distribution_tv(
  before: &[(NodeId, Weight)],
  after: &[(NodeId, Weight)],
) -> Weight {
  let sb: Weight = before.iter().map(|(_, w)| *w).sum();
  let sa: Weight = after.iter().map(|(_, w)| *w).sum();
  match (sb > 0.0, sa > 0.0) {
    (false, false) => 0.0,
    (true, false) | (false, true) => 1.0,
    (true, true) => {
      let mut p: IntMap<NodeId, (Weight, Weight)> = IntMap::default();
      for &(n, w) in before {
        p.entry(n).or_default().0 += w / sb;
      }
      for &(n, w) in after {
        p.entry(n).or_default().1 += w / sa;
      }
      let mut diffs: Vec<(NodeId, Weight)> = p.into_iter().map(|(n, (b, a))| (n, (a - b).abs())).collect();
      diffs.sort_unstable_by_key(|(n, _)| *n); // a fixed summation order
      (0.5 * diffs.iter().map(|(_, d)| d).sum::<Weight>()).min(1.0)
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn blame_overflow_is_an_error() {
    let mut c = FrameCounters::new();
    c.blame.insert(3, BlameHist { counts: vec![0, u32::MAX] });
    let before = c.clone();
    let r = c.apply(
      &Contribution { absorbed: true, credited: vec![], blamed: vec![(1, 0), (3, 1)] },
      true,
    );
    assert!(r.is_err());
    assert_eq!(c, before);
  }

  #[test]
  fn hist_trailing_zeros_are_trimmed() {
    let mut c = FrameCounters::new();
    let deep = Contribution { absorbed: true, credited: vec![], blamed: vec![(5, 4)] };
    let shallow = Contribution { absorbed: true, credited: vec![], blamed: vec![(5, 1)] };
    c.apply(&shallow, true).unwrap();
    c.apply(&deep, true).unwrap();
    c.apply(&deep, false).unwrap();
    assert_eq!(c.blame_hist(5).unwrap().counts, vec![0, 1]);
    let mut d = FrameCounters::new();
    d.apply(&shallow, true).unwrap();
    assert_eq!(c, d);
  }
}
