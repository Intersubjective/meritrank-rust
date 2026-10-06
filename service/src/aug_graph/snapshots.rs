//! Reverse-score snapshots (D14, JOURNAL.md).
//!
//! A snapshot keeps the raw scores and the footprint of a frame that is not resident: the copy of
//! an evicted frame, or an on-demand sample admitted after a read. It serves reverse scores
//! without the frame. The store is part of the replicated state of a subgraph: it changes only in
//! `apply_op`, so both buffer copies hold the same snapshots.
//!
//! Validity. Strict mode (staleness c = 0): a snapshot is dropped by any change of a positive
//! out-edge of a node in its footprint, and by any wall change of its owner — then it equals what
//! a fresh calculation would give. With c > 0 (a heuristic, no guarantee) each such change adds
//! `(1+λ)·α·tv·visits(S)/n` to its drift, and it is dropped once the drift exceeds
//! `c·(1+λ)·sqrt(ln(2/δ)/(2n))`; changes outside the footprint are not seen.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Arc;

use meritrank_core::{FrameSample, Mutations, NodeId};

use super::AugGraph;
use crate::data::AdmitBatch;

/// Operations whose mutations a subgraph remembers to validate late admissions; a sample taken
/// before the oldest remembered operation is rejected.
pub const MUTATION_LOG_CAPACITY: usize = 8192;

/// δ of the staleness threshold `c·(1+λ)·sqrt(ln(2/δ)/(2n))`.
pub const STALENESS_DELTA: f64 = 0.05;

/// `c·(1+λ)·sqrt(ln(2/δ)/(2n))`; 0 in strict mode.
fn staleness_threshold(
  c: f64,
  lambda: f64,
  n: usize,
) -> f64 {
  if c <= 0.0 || n == 0 {
    return 0.0;
  }
  c * (1.0 + lambda) * ((2.0 / STALENESS_DELTA).ln() / (2.0 * n as f64)).sqrt()
}

/// Fixed bytes counted per snapshot besides its vectors (map entry, FIFO entry, header).
const SNAPSHOT_OVERHEAD: usize = 128;

/// One snapshot: scores of every node of the footprint, with their arrival counts.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameSnapshot {
  /// Walks the frame had.
  pub n:            usize,
  /// Accumulated drift of the staleness heuristic (0 in strict mode).
  pub drift:        f64,
  /// Revision of the ego when the snapshot was taken.
  pub revision:     u64,
  /// Operation after which the snapshot's content was taken.
  pub captured_seq: u64,
  ids:              Vec<NodeId>,
  raw:              Vec<f64>,
  visits:           Vec<u64>,
  /// Cluster bounds over this snapshot's scores, for the zero-opinion revision they were
  /// computed at.
  bounds:           Option<(u64, Vec<f64>)>,
  /// Insertion token (FIFO bookkeeping).
  token:            u64,
}

impl FrameSnapshot {
  fn from_sample(
    sample: &FrameSample,
    discredit: f64,
    decay: f64,
    captured_seq: u64,
    revision: u64,
  ) -> Self {
    let mut ids = Vec::with_capacity(sample.visits.len());
    let mut raw = Vec::with_capacity(sample.visits.len());
    let mut visits = Vec::with_capacity(sample.visits.len());
    for &(node, v) in &sample.visits {
      ids.push(node);
      raw.push(sample.score(node, discredit, decay));
      visits.push(v);
    }
    FrameSnapshot {
      n: sample.n,
      drift: 0.0,
      revision,
      captured_seq,
      ids,
      raw,
      visits,
      bounds: None,
      token: 0,
    }
  }

  /// The stored cluster bounds, if computed at zero-opinion revision `zero_revision`.
  pub(crate) fn bounds_at(
    &self,
    zero_revision: u64,
  ) -> Option<&Vec<f64>> {
    self.bounds.as_ref().filter(|(z, _)| *z == zero_revision).map(|(_, b)| b)
  }

  fn index(
    &self,
    node: NodeId,
  ) -> Option<usize> {
    self.ids.binary_search(&node).ok()
  }

  /// Raw (walk) score of `node`, before zero opinion; 0 outside the footprint.
  pub fn raw(
    &self,
    node: NodeId,
  ) -> f64 {
    self.index(node).map_or(0.0, |i| self.raw[i])
  }

  pub fn in_footprint(
    &self,
    node: NodeId,
  ) -> bool {
    self.index(node).is_some()
  }

  pub fn visits_of(
    &self,
    node: NodeId,
  ) -> u64 {
    self.index(node).map_or(0, |i| self.visits[i])
  }

  pub fn footprint_len(&self) -> usize {
    self.ids.len()
  }

  /// Retained bytes (element storage of its vectors plus a fixed overhead).
  pub fn bytes(&self) -> usize {
    self.ids.capacity() * std::mem::size_of::<NodeId>()
      + self.raw.capacity() * std::mem::size_of::<f64>()
      + self.visits.capacity() * std::mem::size_of::<u64>()
      + self.bounds.as_ref().map_or(0, |(_, b)| b.capacity() * std::mem::size_of::<f64>())
      + SNAPSHOT_OVERHEAD
  }
}

/// The snapshot store of one buffer copy: bounded by a byte quota, first-in first-out.
#[derive(Clone, Debug, Default)]
pub struct FrameSnapshots {
  pub(crate) by_ego: BTreeMap<NodeId, FrameSnapshot>,
  pub(crate) fifo:   VecDeque<(NodeId, u64)>,
  pub(crate) bytes:  usize,
  pub(crate) quota:  usize,
  next_token:        u64,
}

impl FrameSnapshots {
  pub fn len(&self) -> usize {
    self.by_ego.len()
  }

  pub fn is_empty(&self) -> bool {
    self.by_ego.is_empty()
  }

  pub fn bytes(&self) -> usize {
    self.bytes
  }

  pub fn quota(&self) -> usize {
    self.quota
  }

  pub fn get(
    &self,
    ego: NodeId,
  ) -> Option<&FrameSnapshot> {
    self.by_ego.get(&ego)
  }

  pub fn contains(
    &self,
    ego: NodeId,
  ) -> bool {
    self.by_ego.contains_key(&ego)
  }

  /// Egos with a snapshot, sorted.
  pub fn egos(&self) -> Vec<NodeId> {
    self.by_ego.keys().copied().collect()
  }

  /// Stores a snapshot (replacing the ego's previous one), then evicts the oldest while over the
  /// quota. A snapshot larger than the quota is not stored. Returns whether it was stored.
  fn insert(
    &mut self,
    ego: NodeId,
    mut snap: FrameSnapshot,
  ) -> bool {
    self.remove(ego);
    if snap.bytes() > self.quota {
      return false;
    }
    self.next_token += 1;
    snap.token = self.next_token;
    self.bytes += snap.bytes();
    self.fifo.push_back((ego, snap.token));
    self.by_ego.insert(ego, snap);
    self.shrink_to_quota();
    true
  }

  fn remove(
    &mut self,
    ego: NodeId,
  ) -> bool {
    match self.by_ego.remove(&ego) {
      Some(s) => {
        self.bytes -= s.bytes();
        true
      },
      None => false,
    }
  }

  fn shrink_to_quota(&mut self) {
    while self.bytes > self.quota {
      let Some((ego, token)) = self.fifo.pop_front() else { break };
      if self.by_ego.get(&ego).map_or(false, |s| s.token == token) {
        self.remove(ego);
      }
    }
    // Drop FIFO entries of snapshots removed meanwhile, so the queue stays bounded.
    if self.fifo.len() > 2 * self.by_ego.len() + 16 {
      let by_ego = &self.by_ego;
      self.fifo.retain(|(e, t)| by_ego.get(e).map_or(false, |s| s.token == *t));
    }
  }

  fn set_quota(
    &mut self,
    quota: usize,
  ) {
    self.quota = quota;
    self.shrink_to_quota();
  }

  fn clear(&mut self) {
    self.by_ego.clear();
    self.fifo.clear();
    self.bytes = 0;
  }
}

/// Graph changes of the last `MUTATION_LOG_CAPACITY` mutating operations.
#[derive(Clone, Debug, Default)]
pub struct MutationLog {
  entries: VecDeque<(u64, Vec<NodeId>, Vec<NodeId>)>,
  /// Mutations of operations at or before `floor` may be missing: a sample whose base is before
  /// it cannot be validated.
  floor:   u64,
}

impl MutationLog {
  fn push(
    &mut self,
    seq: u64,
    m: &Mutations,
  ) {
    if m.is_empty() {
      return;
    }
    self.entries.push_back((seq, m.sources.iter().map(|s| s.src).collect(), m.wall_owners.clone()));
    while self.entries.len() > MUTATION_LOG_CAPACITY {
      if let Some((s, _, _)) = self.entries.pop_front() {
        self.floor = self.floor.max(s);
      }
    }
  }

  /// Everything up to `seq` is forgotten (bulk load, reset).
  fn reset(
    &mut self,
    seq: u64,
  ) {
    self.entries.clear();
    self.floor = seq;
  }

  /// Whether a sample taken after operation `base` was invalidated since: a change of a source in
  /// its footprint, a wall change of its ego, or mutations the log no longer holds.
  fn invalidates(
    &self,
    base: u64,
    sample: &FrameSample,
  ) -> bool {
    if base < self.floor {
      return true;
    }
    self.entries.iter().rev().take_while(|(s, _, _)| *s > base).any(|(_, sources, walls)| {
      walls.contains(&sample.ego) || sources.iter().any(|&s| sample.in_footprint(s))
    })
  }
}

/// Deterministic counters of a buffer copy: both copies apply the same operations, so they agree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphCounters {
  pub frames_calculated:     u64,
  pub snapshots_captured:    u64,
  pub snapshots_admitted:    u64,
  pub snapshots_rejected:    u64,
  pub snapshots_invalidated: u64,
}

/// Where a reverse score of a peer comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReverseSource {
  /// The peer's resident frame.
  Frame,
  /// A snapshot of the peer.
  Snapshot,
  /// Neither: a read samples the peer (or, outside a read scope, the score is 0).
  Missing,
}

/// Diagnostics of a peer's reverse-score source (no wire change; tests, tracing, benchmarks).
#[derive(Clone, Debug, PartialEq)]
pub struct ReverseDiag {
  pub source:       ReverseSource,
  /// Walks behind the value (0 when missing).
  pub n:            usize,
  pub drift:        f64,
  /// Drift beyond which the snapshot stops serving (0 in strict mode).
  pub threshold:    f64,
  pub revision:     u64,
  pub captured_seq: Option<u64>,
  /// Operations applied since the snapshot was taken.
  pub age_ops:      Option<u64>,
}

// ---------------------------------------------------------------------------
// Read scope
// ---------------------------------------------------------------------------

/// What a read did, collected by `read_scope`.
#[derive(Clone, Debug, Default)]
pub struct ReadReport {
  /// Resident frames the read touched (sorted).
  pub frames:        Vec<NodeId>,
  /// Frames the read sampled, to offer for admission.
  pub sampled:       Vec<Arc<FrameSample>>,
  /// Distinct egos whose frames served the read.
  pub from_frame:    u64,
  /// Distinct egos whose snapshots served the read.
  pub from_snapshot: u64,
  pub sample_walks:  u64,
  /// Cluster bounds the read computed for its samples (aligned with `sampled`).
  pub bounds:        Vec<Option<Vec<f64>>>,
  /// Egos the read needed but found neither resident nor in a snapshot (answered 0 without
  /// sampling; a sampling read never has any).
  pub missing:       Vec<NodeId>,
}

#[derive(Default)]
struct Scope {
  sampling:  bool,
  frames:    BTreeSet<NodeId>,
  missing:   BTreeSet<NodeId>,
  snapshots: BTreeSet<NodeId>,
  samples:   BTreeMap<NodeId, Arc<FrameSample>>,
  order:     Vec<NodeId>,
  bounds:    HashMap<NodeId, Vec<f64>>,
}

thread_local! {
  static SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
}

/// Runs a synchronous read and reports, with its result, what it touched. With `sampling`, a
/// reverse score of a peer that has neither a resident frame nor a valid snapshot is taken from a
/// fresh sample of the peer (memoized for the read); without it, such a score is 0 (and the peer's
/// frame is recorded, as `record_frames` does).
pub fn read_scope<T>(
  sampling: bool,
  read: impl FnOnce() -> T,
) -> (T, ReadReport) {
  read_scope_with(sampling, vec![], read)
}

/// A sample taken before the final read of a request (in chunks, under short guards), with the
/// cluster bounds computed from it.
#[derive(Clone, Debug)]
pub struct Presample {
  pub sample:        Arc<FrameSample>,
  pub bounds:        Option<Vec<f64>>,
  /// Operation after which it was taken, and the zero-opinion revision of its bounds.
  pub seq:           u64,
  pub zero_revision: u64,
}

/// `read_scope` with samples taken beforehand: the caller vouches that they are valid for the
/// state read (see `AugGraph::presample_valid`).
pub fn read_scope_with<T>(
  sampling: bool,
  presamples: Vec<(Arc<FrameSample>, Option<Vec<f64>>)>,
  read: impl FnOnce() -> T,
) -> (T, ReadReport) {
  /// Restores the outer scope even if the read panics (threads are reused by the blocking pool).
  struct Restore(Option<Option<Scope>>);
  impl Drop for Restore {
    fn drop(&mut self) {
      if let Some(outer) = self.0.take() {
        SCOPE.with(|s| *s.borrow_mut() = outer);
      }
    }
  }
  let mut scope = Scope { sampling, ..Default::default() };
  for (sample, bounds) in presamples {
    let ego = sample.ego;
    if let Some(b) = bounds {
      scope.bounds.insert(ego, b);
    }
    scope.samples.insert(ego, sample);
    scope.order.push(ego);
  }
  let outer = SCOPE.with(|s| s.replace(Some(scope)));
  let mut restore = Restore(Some(outer));
  let result = read();
  let outer = restore.0.take().unwrap_or(None);
  let scope = SCOPE.with(|s| s.replace(outer)).unwrap_or_default();
  let sampled: Vec<Arc<FrameSample>> =
    scope.order.iter().filter_map(|e| scope.samples.get(e).cloned()).collect();
  let bounds = scope.order.iter().map(|e| scope.bounds.get(e).cloned()).collect();
  let report = ReadReport {
    bounds,
    from_frame:    scope.frames.len() as u64,
    from_snapshot: scope.snapshots.len() as u64,
    sample_walks:  sampled.iter().map(|s| s.n as u64).sum(),
    frames:        scope.frames.into_iter().collect(),
    missing:       scope.missing.into_iter().collect(),
    sampled,
  };
  (result, report)
}

/// Runs a synchronous read and returns, with its result, the egos whose frames it accessed.
pub fn record_frames<T>(read: impl FnOnce() -> T) -> (T, Vec<NodeId>) {
  let (r, report) = read_scope(false, read);
  (r, report.frames)
}

/// Notes that the current read accesses `ego`'s frame (no-op outside a read scope).
pub(crate) fn record_frame_access(ego: NodeId) {
  SCOPE.with(|s| {
    if let Some(scope) = s.borrow_mut().as_mut() {
      scope.frames.insert(ego);
    }
  });
}

fn note_snapshot(ego: NodeId) {
  SCOPE.with(|s| {
    if let Some(scope) = s.borrow_mut().as_mut() {
      scope.snapshots.insert(ego);
    }
  });
}

fn note_missing(ego: NodeId) {
  SCOPE.with(|s| {
    if let Some(scope) = s.borrow_mut().as_mut() {
      scope.missing.insert(ego);
    }
  });
}

fn scope_sampling() -> bool {
  SCOPE.with(|s| s.borrow().as_ref().map_or(false, |x| x.sampling))
}

fn scope_sample_get(ego: NodeId) -> Option<Arc<FrameSample>> {
  SCOPE.with(|s| s.borrow().as_ref().and_then(|x| x.samples.get(&ego).cloned()))
}

/// The read-local cluster bounds of a sampled ego (never cached across reads).
pub(crate) fn scope_bounds(
  ego: NodeId,
  compute: impl FnOnce() -> Vec<f64>,
) -> Option<Vec<f64>> {
  let sampled = SCOPE.with(|s| s.borrow().as_ref().map_or(false, |x| x.samples.contains_key(&ego)));
  if !sampled {
    return None;
  }
  if let Some(b) = SCOPE.with(|s| s.borrow().as_ref().and_then(|x| x.bounds.get(&ego).cloned())) {
    return Some(b);
  }
  let b = compute();
  SCOPE.with(|s| {
    if let Some(scope) = s.borrow_mut().as_mut() {
      scope.bounds.insert(ego, b.clone());
    }
  });
  Some(b)
}

// ---------------------------------------------------------------------------
// AugGraph: reading
// ---------------------------------------------------------------------------

impl AugGraph {
  /// A fresh sample of `ego` with its cluster bounds, taken now (a chunk of a request's samples).
  pub fn presample(
    &self,
    ego: NodeId,
  ) -> Option<Presample> {
    if self.mr.is_calculated(ego) || self.snapshots.contains(ego) {
      return None;
    }
    let sample = Arc::new(self.fresh_sample(ego)?);
    let (bounds, _) = read_scope_with(true, vec![(Arc::clone(&sample), None)], || {
      self.calculate_score_clusters_bounds_pub(ego)
    });
    Some(Presample { sample, bounds: Some(bounds), seq: self.applied_seq, zero_revision: self.zero_revision })
  }

  /// Whether a presample still equals a fresh sample now: same incarnation, no change in its
  /// footprint or of its owner's walls since it was taken, and still neither resident nor in a
  /// snapshot. Its bounds are kept only for the same zero-opinion revision.
  pub fn presample_valid(
    &self,
    epoch: u64,
    p: &Presample,
  ) -> Option<(Arc<FrameSample>, Option<Vec<f64>>)> {
    let ego = p.sample.ego;
    if epoch != self.epoch
      || self.mr.is_calculated(ego)
      || self.snapshots.contains(ego)
      || self.mutation_log.invalidates(p.seq, &p.sample)
    {
      return None;
    }
    let bounds = (p.zero_revision == self.zero_revision).then(|| p.bounds.clone()).flatten();
    Some((Arc::clone(&p.sample), bounds))
  }

  /// A fresh on-demand sample of `ego` (`Settings::on_demand_walks` walks).
  pub fn fresh_sample(
    &self,
    ego: NodeId,
  ) -> Option<FrameSample> {
    if self.nodes.get_by_id(ego).is_none() {
      return None;
    }
    self.mr.sample_frame(ego, self.settings.on_demand_walks(), self.fresh_seed(ego)).ok()
  }

  /// Drift beyond which a snapshot of `n` walks stops serving (0 in strict mode).
  pub fn snapshot_threshold(
    &self,
    n: usize,
  ) -> f64 {
    staleness_threshold(self.settings.snapshot_staleness, self.settings.discredit_lambda, n)
  }

  /// Where a reverse score of `peer` would come from now.
  pub fn reverse_diag(
    &self,
    peer: NodeId,
  ) -> ReverseDiag {
    let revision = self.revision(peer);
    if self.mr.is_calculated(peer) {
      return ReverseDiag {
        source: ReverseSource::Frame,
        n: self.mr.walks_per_ego(),
        drift: 0.0,
        threshold: 0.0,
        revision,
        captured_seq: None,
        age_ops: None,
      };
    }
    match self.snapshots.get(peer) {
      Some(s) => ReverseDiag {
        source: ReverseSource::Snapshot,
        n: s.n,
        drift: s.drift,
        threshold: self.snapshot_threshold(s.n),
        revision,
        captured_seq: Some(s.captured_seq),
        age_ops: Some(self.applied_seq.saturating_sub(s.captured_seq)),
      },
      None => ReverseDiag {
        source: ReverseSource::Missing,
        n: 0,
        drift: 0.0,
        threshold: 0.0,
        revision,
        captured_seq: None,
        age_ops: None,
      },
    }
  }

  /// The walk score of `dst` in `ego`'s frame (before zero opinion), from the resident frame, a
  /// snapshot, or — inside a sampling read scope — a fresh sample. `None` when none is available.
  pub(crate) fn walk_score(
    &self,
    ego: NodeId,
    dst: NodeId,
  ) -> Option<f64> {
    if self.mr.is_calculated(ego) {
      record_frame_access(ego);
      return self.mr.get_node_score(ego, dst).ok();
    }
    if let Some(s) = self.snapshots.get(ego) {
      note_snapshot(ego);
      return Some(s.raw(dst));
    }
    if scope_sampling() {
      let sample = match scope_sample_get(ego) {
        Some(x) => x,
        None => {
          let fresh = Arc::new(self.fresh_sample(ego)?);
          SCOPE.with(|s| {
            if let Some(scope) = s.borrow_mut().as_mut() {
              scope.samples.insert(ego, Arc::clone(&fresh));
              scope.order.push(ego);
            }
          });
          fresh
        },
      };
      return Some(sample.score(dst, self.settings.discredit_lambda, self.settings.blame_decay));
    }
    record_frame_access(ego);
    note_missing(ego);
    None
  }
}

impl AugGraph {
  /// Nodes of the walks behind the ego's scores (resident frame, snapshot, or the current read's
  /// sample); `None` when the ego has none.
  pub(crate) fn walk_nodes(
    &self,
    ego: NodeId,
  ) -> Option<Vec<NodeId>> {
    if let Some(c) = self.mr.frame_counters(ego) {
      return Some(c.nodes());
    }
    if let Some(s) = self.snapshots.get(ego) {
      return Some(s.ids.clone());
    }
    scope_sample_get(ego).map(|s| s.visits.iter().map(|(n, _)| *n).collect())
  }
}

// ---------------------------------------------------------------------------
// AugGraph: replicated changes (called from `apply_op`)
// ---------------------------------------------------------------------------

impl AugGraph {
  /// Keeps an evicted frame as a snapshot (same content: the revision does not change).
  pub(crate) fn capture_frame(
    &mut self,
    ego: NodeId,
  ) {
    if self.snapshots.quota == 0 || !self.mr.is_calculated(ego) {
      return;
    }
    let Some(sample) = self.mr.frame_sample(ego) else { return };
    let snap = FrameSnapshot::from_sample(
      &sample,
      self.settings.discredit_lambda,
      self.settings.blame_decay,
      self.applied_seq,
      self.revision(ego),
    );
    if self.snapshots.insert(ego, snap) {
      self.counters.snapshots_captured += 1;
    }
  }

  /// The ego became resident: its frame is authoritative.
  pub(crate) fn drop_snapshot(
    &mut self,
    ego: NodeId,
  ) {
    self.snapshots.remove(ego);
  }

  /// Logs the operation's graph changes and applies them to the snapshots.
  pub(crate) fn absorb_mutations(
    &mut self,
    m: &Mutations,
  ) {
    self.mutation_log.push(self.applied_seq, m);
    if self.snapshots.is_empty() || m.is_empty() {
      return;
    }
    let strict = self.settings.snapshot_staleness <= 0.0;
    let factor = (1.0 + self.settings.discredit_lambda) * self.settings.alpha;
    let mut dropped: BTreeSet<NodeId> = m
      .wall_owners
      .iter()
      .copied()
      .filter(|o| self.snapshots.contains(*o))
      .collect();
    let (c, lambda) = (self.settings.snapshot_staleness, self.settings.discredit_lambda);
    let mut thresholds: BTreeMap<usize, f64> = BTreeMap::new();
    for change in &m.sources {
      for (ego, snap) in self.snapshots.by_ego.iter_mut() {
        let Some(i) = snap.index(change.src) else { continue };
        if strict {
          dropped.insert(*ego);
          continue;
        }
        let tv = change.tv.unwrap_or(1.0);
        snap.drift += factor * tv * snap.visits[i] as f64 / snap.n as f64;
        let limit = *thresholds.entry(snap.n).or_insert_with(|| staleness_threshold(c, lambda, snap.n));
        if snap.drift > limit {
          dropped.insert(*ego);
        }
      }
    }
    for ego in dropped {
      if self.snapshots.remove(ego) {
        self.counters.snapshots_invalidated += 1;
        self.bump_revision(ego);
      }
    }
  }

  /// Validates and stores samples taken by a read (see `MutationLog::invalidates`).
  pub(crate) fn admit(
    &mut self,
    batch: &AdmitBatch,
  ) {
    for (i, sample) in batch.samples.iter().enumerate() {
      let ego = sample.ego;
      let valid = batch.epoch == self.epoch
        && self.nodes.get_by_id(ego).is_some()
        && !self.mr.is_calculated(ego)
        && !self.snapshots.contains(ego)
        && !self.mutation_log.invalidates(batch.base_seq, sample);
      if !valid {
        self.counters.snapshots_rejected += 1;
        continue;
      }
      self.bump_revision(ego);
      let mut snap = FrameSnapshot::from_sample(
        sample,
        self.settings.discredit_lambda,
        self.settings.blame_decay,
        batch.base_seq,
        self.revision(ego),
      );
      if batch.zero_revision == self.zero_revision {
        if let Some(Some(b)) = batch.bounds.get(i) {
          snap.bounds = Some((self.zero_revision, b.clone()));
        }
      }
      if self.snapshots.insert(ego, snap) {
        self.counters.snapshots_admitted += 1;
      } else {
        self.counters.snapshots_rejected += 1;
      }
    }
  }

  pub(crate) fn set_snapshot_quota(
    &mut self,
    quota: usize,
  ) {
    self.snapshots.set_quota(quota);
  }

  /// Forgets every snapshot and every logged mutation up to now (bulk load).
  pub(crate) fn clear_snapshots(&mut self) {
    self.snapshots.clear();
    self.mutation_log.reset(self.applied_seq);
  }
}
