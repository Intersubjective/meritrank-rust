use crate::aug_graph::node_registry::NodeRegistry;
use crate::aug_graph::nodes::{NodeKind, ScoreClustersByKind};
use crate::aug_graph::settings::AugGraphSettings;
use bincode::{Decode, Encode};
use left_right::Absorb;
use meritrank_core::{MeritRank, NodeId, Weight};
use moka::sync::Cache;
use std::time::Duration;

mod clustering;
mod node_registry;
mod nodes;
mod write;
mod read;
mod settings;

#[derive(Debug, Encode, Decode, Eq, PartialEq)]
pub enum AugGraphOpcode {
  WriteEdge,
}

#[derive(Clone)]
pub struct AugGraph {
  mr:                    MeritRank,
  nodes:                 NodeRegistry,
  settings:              AugGraphSettings,
  zero_opinion:          Vec<Weight>, // TODO: change to map because of sparseness
  cached_scores:         Cache<(NodeId, NodeId), Weight>,
  //cached_walks:          LruCache<NodeId, ()>,
  cached_score_clusters: Cache<(NodeId, NodeKind), ScoreClustersByKind>,

  //poll_store:            PollStore,
}

impl AugGraph {
  pub fn new() -> AugGraph {
    let cached_score_clusters: Cache<NodeId, ScoreClustersByKind> =
      Cache::builder()
      .max_capacity(1_000_000)  // Adjust as needed
      .time_to_live(Duration::from_secs(3600))  // Optional: Set TTL
      .build();
    todo!();
  }
}
