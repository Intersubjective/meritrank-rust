use crate::data::*;
use crate::node_registry::*;
use crate::settings::*;
use crate::vsids::VSIDSManager;

use meritrank_core::{Graph, IntMap, MeritRank, NodeId};
use moka::sync::Cache;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

mod absorb;
mod calc;
mod edges;
mod graph_read;
mod neighbors;
mod scores;
mod snapshots;
pub use snapshots::{
  read_scope, FrameSnapshot, FrameSnapshots, GraphCounters, MutationLog, ReadReport, ReverseDiag,
  ReverseSource, MUTATION_LOG_CAPACITY, STALENESS_DELTA,
};
pub use snapshots::record_frames;
pub(crate) use snapshots::record_frame_access;

pub type ClusterGroupBounds = Vec<NodeScore>;

/// Cluster bounds are valid for one revision of the ego's estimate and one zero-opinion revision.
pub type ClusterKey = (NodeId, u64, u64);

pub struct AugGraph {
  pub mr:                    MeritRank,
  pub nodes:                 NodeRegistry,
  pub settings:              Settings,
  pub zero_opinion:          Vec<NodeScore>, // FIXME: change to map because of sparseness
  /// Derived from this copy's own state only: `Clone` builds a fresh one, so the two buffer
  /// copies never share it.
  pub cached_score_clusters: Cache<ClusterKey, ClusterGroupBounds>,
  /// Per ego: changes whenever the estimate behind its scores changes (a recalculation, a repair
  /// of its walks, a new or invalidated snapshot) — never on eviction (D14).
  pub revisions:             IntMap<NodeId, u64>,
  /// Bumped by every zero-opinion write.
  pub zero_revision:         u64,
  pub vsids:                 VSIDSManager,
  pub stamp:                 u64,
  /// Reverse-score snapshots (D14); replicated state.
  pub snapshots:             FrameSnapshots,
  /// Graph changes of the recent operations, to validate late admissions.
  pub mutation_log:          MutationLog,
  pub counters:              GraphCounters,
  /// Identity of the processor incarnation (a new one per subgraph creation, reset, bulk load).
  pub epoch:                 u64,
  /// Sequence number of the last operation applied to this copy.
  pub applied_seq:           u64,
  /// Key of the subgraph's random streams.
  pub stream:                u64,
}

fn cluster_cache(settings: &Settings) -> Cache<ClusterKey, ClusterGroupBounds> {
  Cache::builder()
    .max_capacity(settings.score_clusters_cache_size as u64)
    .time_to_live(Duration::from_secs(settings.score_clusters_timeout))
    .build()
}

impl Clone for AugGraph {
  fn clone(&self) -> Self {
    AugGraph {
      mr:                    self.mr.clone(),
      nodes:                 self.nodes.clone(),
      settings:              self.settings.clone(),
      zero_opinion:          self.zero_opinion.clone(),
      cached_score_clusters: cluster_cache(&self.settings),
      revisions:             self.revisions.clone(),
      zero_revision:         self.zero_revision,
      vsids:                 self.vsids.clone(),
      stamp:                 self.stamp,
      snapshots:             self.snapshots.clone(),
      mutation_log:          self.mutation_log.clone(),
      counters:              self.counters.clone(),
      epoch:                 self.epoch,
      applied_seq:           self.applied_seq,
      stream:                self.stream,
    }
  }
}

/// SplitMix64 finaliser.
pub fn mix64(mut z: u64) -> u64 {
  z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
  z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
  z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
  z ^ (z >> 31)
}

/// Stable 64-bit key of a subgraph name (FNV-1a), for deriving its random streams.
pub fn stream_key(name: &str) -> u64 {
  name.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
    (h ^ b as u64).wrapping_mul(0x0100_0000_01B3)
  })
}

/// Seed of the random stream an operation uses: both copies apply it with the same stream, so
/// they stay identical, and a rerun of the same sequence reproduces every walk.
pub fn op_seed(
  seed: u64,
  stream: u64,
  seq: u64,
) -> u64 {
  mix64(mix64(seed ^ mix64(stream)) ^ seq)
}

/// Domain of fresh-frame seeds, apart from operation seeds.
const FRESH_DOMAIN: u64 = 0xF5E5_D14F_5E5D_14F5;

/// Epochs of graph incarnations: unique in the process.
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(crate) enum AugGraphError {
  SelfReference,
  EmptyName,
}

impl AugGraph {
  pub fn new(settings: Settings) -> AugGraph {
    Self::with_stream(settings, "")
  }

  /// A graph for the subgraph `name`: its random streams are keyed by the name, and it gets a new
  /// processor epoch.
  pub fn with_stream(
    settings: Settings,
    name: &str,
  ) -> AugGraph {
    let stream = stream_key(name);
    let epoch = NEXT_EPOCH.fetch_add(1, Ordering::Relaxed);
    Self::build(settings, stream, epoch)
  }

  pub(crate) fn build_reset(
    settings: Settings,
    stream: u64,
    epoch: u64,
  ) -> AugGraph {
    Self::build(settings, stream, epoch)
  }

  fn build(
    settings: Settings,
    stream: u64,
    epoch: u64,
  ) -> AugGraph {
    let mut mr = MeritRank::new(Graph::new(), settings.num_walks);
    mr.alpha = settings.alpha;
    mr.discredit = settings.discredit_lambda;
    mr.blame_decay = settings.blame_decay;
    mr.blame_radius = settings.blame_radius;
    // Both buffer copies start from the same stream; the subgraph worker reseeds it before every
    // operation.
    mr.reseed(settings.seed);

    let mut snapshots = FrameSnapshots::default();
    snapshots.quota = settings.snapshot_bytes_per_copy(1);

    AugGraph {
      mr,
      nodes: NodeRegistry::new(),
      settings: settings.clone(),
      zero_opinion: Vec::new(),
      cached_score_clusters: cluster_cache(&settings),
      revisions: IntMap::default(),
      zero_revision: 0,
      vsids: VSIDSManager::new(),
      stamp: 0,
      snapshots,
      mutation_log: MutationLog::default(),
      counters: GraphCounters::default(),
      epoch,
      applied_seq: 0,
      stream,
    }
  }

  /// Applies operation `seq` as the subgraph worker does: reseeds the per-operation stream, then
  /// applies it and records `seq` as applied. Replaying the same sequence reproduces the state.
  pub fn apply_seq_op(
    &mut self,
    seq: u64,
    op: &AugGraphOp,
  ) {
    self.mr.reseed(op_seed(self.settings.seed, self.stream, seq));
    self.applied_seq = seq;
    self.apply_op(op);
  }

  /// Seed of `ego`'s fresh frames (calculations and samples): a function of the settings' seed,
  /// the subgraph and the ego only.
  pub fn fresh_seed(
    &self,
    ego: NodeId,
  ) -> u64 {
    mix64(mix64(self.settings.seed ^ mix64(self.stream)) ^ mix64(ego as u64 ^ FRESH_DOMAIN))
  }

  /// The ego's revision: changes whenever the estimate behind its scores changes.
  pub fn revision(
    &self,
    ego: NodeId,
  ) -> u64 {
    self.revisions.get(&ego).copied().unwrap_or(0)
  }

  pub(crate) fn bump_revision(
    &mut self,
    ego: NodeId,
  ) {
    *self.revisions.entry(ego).or_default() += 1;
  }

  pub(crate) fn cluster_key(
    &self,
    ego: NodeId,
  ) -> ClusterKey {
    (ego, self.revision(ego), self.zero_revision)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::node_registry::NodeRegistry;
  use meritrank_core::Graph;

  #[test]
  fn node_registry() {
    let mut mr = MeritRank::new(Graph::new(), 10000);

    let mut registry = NodeRegistry::new();

    let user_id = registry.register(&mut mr, "Alice".to_string());
    assert_eq!(user_id, 0);

    // One node class: any name registers the same way.
    let comment_id = registry.register(&mut mr, "Comment1".to_string());
    assert_eq!(comment_id, 1);

    let info = registry.get_by_id(0).unwrap();
    assert_eq!(info.name, "Alice");

    let info = registry.get_by_name("Comment1").unwrap();
    assert_eq!(info.id, 1);

    let existing_id = registry.register(&mut mr, "Alice".to_string());
    assert_eq!(existing_id, 0);

    assert_eq!(registry.get_by_id(2), None);
    assert_eq!(registry.get_by_name("Bob"), None);
    assert_eq!(registry.len(), 2);
  }
}
