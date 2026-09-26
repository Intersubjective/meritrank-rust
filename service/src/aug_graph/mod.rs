use crate::data::*;
use crate::node_registry::*;
use crate::settings::*;
use crate::utils::log::*;
use crate::vsids::VSIDSManager;

use meritrank_core::{Graph, IntMap, MeritRank, NodeId};
use moka::sync::Cache;

use std::time::Duration;

mod absorb;
mod calc;
mod edges;
mod graph_read;
mod neighbors;
mod scores;

pub type ClusterGroupBounds = Vec<NodeScore>;

/// Cluster bounds are valid for one generation of the ego's frame and one zero-opinion revision.
pub type ClusterKey = (NodeId, NodeKind, u64, u64);

pub struct AugGraph {
  pub mr:                    MeritRank,
  pub nodes:                 NodeRegistry,
  pub settings:              Settings,
  pub zero_opinion:          Vec<NodeScore>, // FIXME: change to map because of sparseness
  /// Derived from this copy's own state only: `Clone` builds a fresh one, so the two buffer
  /// copies never share it.
  pub cached_score_clusters: Cache<ClusterKey, ClusterGroupBounds>,
  /// Bumped whenever an ego's walks change (`MeritRank::take_dirty_egos`).
  pub generations:           IntMap<NodeId, u64>,
  /// Bumped by every zero-opinion write.
  pub zero_revision:         u64,
  pub vsids:                 VSIDSManager,
  pub stamp:                 u64,
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
      generations:           self.generations.clone(),
      zero_revision:         self.zero_revision,
      vsids:                 self.vsids.clone(),
      stamp:                 self.stamp,
    }
  }
}

thread_local! {
  /// Egos whose frames the current read touched, when recording (`record_frames`).
  static FRAME_LOG: std::cell::RefCell<Option<std::collections::BTreeSet<NodeId>>> =
    const { std::cell::RefCell::new(None) };
}

/// Notes that the current read accesses `ego`'s frame (no-op unless recording).
pub(crate) fn record_frame_access(ego: NodeId) {
  FRAME_LOG.with(|log| {
    if let Some(set) = log.borrow_mut().as_mut() {
      set.insert(ego);
    }
  });
}

/// Runs a synchronous read and returns, with its result, the egos whose frames it accessed.
pub fn record_frames<T>(read: impl FnOnce() -> T) -> (T, Vec<NodeId>) {
  FRAME_LOG.with(|log| *log.borrow_mut() = Some(Default::default()));
  let result = read();
  let frames = FRAME_LOG
    .with(|log| log.borrow_mut().take())
    .unwrap_or_default()
    .into_iter()
    .collect();
  (result, frames)
}

#[derive(Debug)]
pub(crate) enum AugGraphError {
  SelfReference,
  IncorrectNodeKinds(NodeName, NodeName),
}

impl AugGraph {
  pub fn new(settings: Settings) -> AugGraph {
    let mut mr = MeritRank::new(Graph::new(), settings.num_walks);
    mr.alpha = settings.alpha;
    // Both buffer copies start from the same stream; the subgraph worker reseeds it before every
    // operation.
    mr.reseed(settings.seed);

    AugGraph {
      mr,
      nodes: NodeRegistry::new(),
      settings: settings.clone(),
      zero_opinion: Vec::new(),
      cached_score_clusters: cluster_cache(&settings),
      generations: IntMap::default(),
      zero_revision: 0,
      vsids: VSIDSManager::new(),
      stamp: 0,
    }
  }

  pub(crate) fn cluster_key(
    &self,
    ego: NodeId,
    kind: NodeKind,
  ) -> ClusterKey {
    let generation = self.generations.get(&ego).copied().unwrap_or(0);
    (ego, kind, generation, self.zero_revision)
  }

  /// Bumps the generation of every ego whose walks changed since the last call.
  pub(crate) fn bump_generations(&mut self) {
    for ego in self.mr.take_dirty_egos() {
      *self.generations.entry(ego).or_default() += 1;
    }
  }

  /// Returns true if ego is a User node (valid for score/calculation).
  /// Logs error and returns false if not; callers should return empty/fail.
  pub(crate) fn ensure_ego_is_user(&self, ego_name: &str, ego_info: &NodeInfo) -> bool {
    if ego_info.kind == NodeKind::User {
      return true;
    }
    log_error!("Non-user node used as ego (rejected): {}", ego_name);
    false
  }

  pub(crate) fn get_object_owner(
    &self,
    node: NodeId,
  ) -> Option<NodeId> {
    match self.nodes.id_to_info.get(node) {
      Some(info) => match info.owner {
        Some(id) => Some(id),
        None => {
          if info.kind == NodeKind::Opinion {
            self
              .mr
              .graph
              .get_node_data(node)
              .and_then(|data| data.inbound_edges.iter().next().map(|(&k, _)| k))
          } else {
            Some(node)
          }
        },
      },
      None => Some(node),
    }
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

    let user_id =
      registry.register(&mut mr, "Alice".to_string(), NodeKind::User);
    assert_eq!(user_id, 0);

    let comment_id = registry.register_with_owner(
      &mut mr,
      "Comment1".to_string(),
      NodeKind::Comment,
      user_id,
    );
    assert_eq!(comment_id, 1);

    // Test get_by_id
    let info = registry.get_by_id(0).unwrap();
    assert_eq!(info.name, "Alice");
    assert_eq!(info.kind, NodeKind::User);
    assert_eq!(info.owner, None);

    // Test get_by_name
    let info = registry.get_by_name("Comment1").unwrap();
    assert_eq!(info.id, 1);
    assert_eq!(info.kind, NodeKind::Comment);
    assert_eq!(info.owner, Some(user_id));

    // Test registering an existing name
    let existing_id =
      registry.register(&mut mr, "Alice".to_string(), NodeKind::User);
    assert_eq!(existing_id, 0);

    // Test non-existent entries
    assert_eq!(registry.get_by_id(2), None);
    assert_eq!(registry.get_by_name("Bob"), None);

    // Test nodes_by_kind (index by kind)
    assert_eq!(registry.nodes_by_kind(NodeKind::User), &[0]);
    assert_eq!(registry.nodes_by_kind(NodeKind::Comment), &[1]);
    assert!(registry.nodes_by_kind(NodeKind::Beacon).is_empty());
  }
}
