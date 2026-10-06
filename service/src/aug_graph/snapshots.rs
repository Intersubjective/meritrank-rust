//! Reverse-score snapshots (D14, JOURNAL.md).
//!
//! A snapshot keeps the raw scores and the footprint of a frame that is not resident: the copy of
//! an evicted frame, or an on-demand sample admitted after a read. It serves reverse scores
//! without the frame. The store is part of the replicated state of a subgraph: it changes only in
//! `apply_op`, so both buffer copies hold the same snapshots.

use std::sync::Arc;

use meritrank_core::{FrameSample, NodeId};

use super::AugGraph;

/// Operations whose mutations a subgraph remembers to validate late admissions; a sample taken
/// before the oldest remembered operation is rejected.
pub const MUTATION_LOG_CAPACITY: usize = 8192;

/// δ of the staleness threshold `c·(1+λ)·sqrt(ln(2/δ)/(2n))`.
pub const STALENESS_DELTA: f64 = 0.05;

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
}

impl FrameSnapshot {
  /// Raw (walk) score of `node`, before zero opinion; 0 outside the footprint.
  pub fn raw(
    &self,
    node: NodeId,
  ) -> f64 {
    let _ = node;
    todo!("D14: FrameSnapshot::raw")
  }

  pub fn in_footprint(
    &self,
    node: NodeId,
  ) -> bool {
    let _ = node;
    todo!("D14: FrameSnapshot::in_footprint")
  }

  pub fn visits_of(
    &self,
    node: NodeId,
  ) -> u64 {
    let _ = node;
    todo!("D14: FrameSnapshot::visits_of")
  }

  pub fn footprint_len(&self) -> usize {
    self.ids.len()
  }

  /// Retained bytes (element storage of its vectors plus a fixed overhead).
  pub fn bytes(&self) -> usize {
    todo!("D14: FrameSnapshot::bytes")
  }
}

/// The snapshot store of one buffer copy.
#[derive(Clone, Debug, Default)]
pub struct FrameSnapshots {
  pub(crate) by_ego: std::collections::BTreeMap<NodeId, FrameSnapshot>,
  pub(crate) fifo:   std::collections::VecDeque<NodeId>,
  pub(crate) bytes:  usize,
  pub(crate) quota:  usize,
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

/// What a read did, collected by `read_scope`.
#[derive(Clone, Debug, Default)]
pub struct ReadReport {
  /// Resident frames the read touched (sorted).
  pub frames:        Vec<NodeId>,
  /// Frames the read sampled, to offer for admission.
  pub sampled:       Vec<Arc<FrameSample>>,
  pub from_frame:    u64,
  pub from_snapshot: u64,
  pub sample_walks:  u64,
}

/// Runs a synchronous read and reports, with its result, what it touched. With `sampling`, a
/// reverse score of a peer that has neither a resident frame nor a valid snapshot is taken from a
/// fresh sample of the peer (memoized for the read); without it, such a score is 0 (and the peer's
/// frame is recorded, as `record_frames` does).
pub fn read_scope<T>(
  sampling: bool,
  read: impl FnOnce() -> T,
) -> (T, ReadReport) {
  let _ = (sampling, read);
  todo!("D14: read_scope")
}

impl AugGraph {
  /// Seed of `ego`'s fresh frames (calculations and samples): a function of the settings' seed,
  /// the subgraph and the ego only.
  pub fn fresh_seed(
    &self,
    ego: NodeId,
  ) -> u64 {
    let _ = ego;
    todo!("D14: fresh_seed")
  }

  /// A fresh on-demand sample of `ego` (`Settings::on_demand_walks` walks).
  pub fn fresh_sample(
    &self,
    ego: NodeId,
  ) -> Option<FrameSample> {
    let _ = ego;
    todo!("D14: fresh_sample")
  }

  /// The ego's revision: changes whenever the estimate behind its scores changes.
  pub fn revision(
    &self,
    ego: NodeId,
  ) -> u64 {
    let _ = ego;
    todo!("D14: revision")
  }

  /// Drift beyond which a snapshot of `n` walks stops serving (0 in strict mode).
  pub fn snapshot_threshold(
    &self,
    n: usize,
  ) -> f64 {
    let _ = n;
    todo!("D14: snapshot_threshold")
  }

  /// Where a reverse score of `peer` would come from now.
  pub fn reverse_diag(
    &self,
    peer: NodeId,
  ) -> ReverseDiag {
    let _ = peer;
    todo!("D14: reverse_diag")
  }
}
