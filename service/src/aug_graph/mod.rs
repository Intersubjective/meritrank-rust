use crate::aug_graph::clustering::ClusterGroupBounds;
use crate::aug_graph::node_registry::NodeRegistry;
use crate::aug_graph::nodes::NodeKind;
use crate::aug_graph::settings::AugGraphSettings;
use bincode::{Decode, Encode};
use meritrank_core::{Graph, MeritRank, NodeId};
use moka::sync::Cache;
use std::time::Duration;
use crate::aug_graph::vsids::VSIDSManager;

mod clustering;
mod node_registry;
pub mod nodes;
pub mod read;
pub mod settings;
pub mod vsids;
mod write;

pub type NodeName = String;
pub type NodeScore = f64;


#[derive(Clone)]
pub struct AugGraph {
  mr:                    MeritRank,
  nodes:                 NodeRegistry,
  settings:              AugGraphSettings,
  zero_opinion:          Vec<NodeScore>, // TODO: change to map because of sparseness
  cached_scores:         Cache<(NodeId, NodeId), NodeScore>,
  cached_score_clusters: Cache<(NodeId, NodeKind), ClusterGroupBounds>,
  //poll_store:            PollStore,
  vsids:      VSIDSManager,
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
      vsids:      VSIDSManager::new(),
    }
  }
}
