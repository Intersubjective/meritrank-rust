use crate::data::*;
use crate::node_registry::*;
use crate::settings::*;
use crate::utils::log::*;
use crate::vsids::VSIDSManager;

use meritrank_core::{Graph, MeritRank, NodeId};
use moka::sync::Cache;

use std::time::Duration;

mod absorb;
mod calc;
mod edges;
mod graph_read;
mod neighbors;
mod scores;

pub type ClusterGroupBounds = Vec<NodeScore>;

#[derive(Clone)]
pub struct AugGraph {
  pub mr:                    MeritRank,
  pub nodes:                 NodeRegistry,
  pub settings:              Settings,
  pub zero_opinion:          Vec<NodeScore>, // FIXME: change to map because of sparseness
  pub cached_scores:         Cache<(NodeId, NodeId), NodeScore>,
  pub cached_score_clusters: Cache<(NodeId, NodeKind), ClusterGroupBounds>,
  pub vsids:                 VSIDSManager,
  pub stamp:                 u64,
}

#[derive(Debug)]
pub(crate) enum AugGraphError {
  SelfReference,
  IncorrectNodeKinds(NodeName, NodeName),
}

impl AugGraph {
  pub fn new(settings: Settings) -> AugGraph {
    let cached_scores: Cache<(NodeId, NodeId), NodeScore> = Cache::builder()
      .max_capacity(settings.scores_cache_size as u64)
      .time_to_live(Duration::from_secs(settings.scores_cache_timeout))
      .build();

    let cached_score_clusters: Cache<(NodeId, NodeKind), ClusterGroupBounds> =
      Cache::builder()
        .max_capacity(settings.score_clusters_cache_size as u64)
        .time_to_live(Duration::from_secs(settings.score_clusters_timeout))
        .build();

    AugGraph {
      mr: MeritRank::new(Graph::new()),
      nodes: NodeRegistry::new(),
      settings: settings.clone(),
      zero_opinion: Vec::new(),
      cached_scores,
      cached_score_clusters,
      vsids: VSIDSManager::new(),
      stamp: 0,
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
    let mut mr = MeritRank::new(Graph::new());

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
