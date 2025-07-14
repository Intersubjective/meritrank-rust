use crate::clustering::ClusterGroupBounds;
use crate::node_registry::NodeRegistry;
use crate::nodes::NodeKind;
use crate::settings::AugGraphSettings;
use crate::vsids::VSIDSManager;
use meritrank_core::{Graph, MeritRank, NodeId};
use moka::sync::Cache;
use std::time::Duration;

pub type NodeName = String;
pub type NodeScore = f64;

#[derive(Clone)]
pub struct AugGraph {
  pub mr:                    MeritRank,
  pub nodes:                 NodeRegistry,
  pub settings:              AugGraphSettings,
  pub zero_opinion:          Vec<NodeScore>, // FIXME: change to map because of sparseness
  pub cached_scores:         Cache<(NodeId, NodeId), NodeScore>,
  pub cached_score_clusters: Cache<(NodeId, NodeKind), ClusterGroupBounds>,
  // pub poll_store:            PollStore,
  pub vsids:                 VSIDSManager,
}

impl AugGraph {
  pub fn new(settings: AugGraphSettings) -> AugGraph {
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
    }
  }
}
