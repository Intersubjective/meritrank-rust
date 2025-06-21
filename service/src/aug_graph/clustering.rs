pub type NodeCluster = usize;

use crate::aug_graph::nodes::NodeKind;
use crate::aug_graph::{AugGraph, NodeScore};
use crate::log::*;
use crate::utils::quantiles::{bounds_are_empty, calculate_quantiles_bounds};
use meritrank_core::{NodeId, Weight};

pub type ClusterGroupBounds = Vec<NodeScore>;

impl AugGraph {
  pub fn update_node_score_clustering(
    &self,
    ego: NodeId,
    kind: NodeKind,
  ) -> ClusterGroupBounds {
    log_trace!("{} {:?}", ego, kind);
    let node_ids = self.nodes.nodes_by_kind(kind);
    let bounds = self.calculate_score_clusters_bounds(ego, kind, &*node_ids);
    self
      .cached_score_clusters
      .insert((ego, kind), bounds.clone());
    bounds
  }

  fn calculate_score_clusters_bounds(
    &self,
    ego: NodeId,
    kind: NodeKind,
    node_ids: &[NodeId],
  ) -> Vec<NodeScore> {
    log_trace!("{} {:?}", ego, kind);

    let scores: Vec<NodeScore> = node_ids
      .iter()
      .map(|dst| self.fetch_raw_score(ego, *dst))
      .filter(|score| *score >= f64::EPSILON)
      .collect();

    if scores.is_empty() {
      return vec![0.0; self.settings.num_score_quantiles - 1];
    }

    calculate_quantiles_bounds(scores, self.settings.num_score_quantiles)
  }

  pub fn apply_score_clustering(
    &self,
    ego_id: NodeId,
    score: NodeScore,
    kind: NodeKind,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{} {}", ego_id, score);

    if score < f64::EPSILON {
      //  Clusterize only positive scores.
      return (score, 0);
    }

    let bounds = &self
      .cached_score_clusters
      .get(&(ego_id, kind))
      .unwrap_or_else(|| self.update_node_score_clustering(ego_id, kind));

    if bounds_are_empty(bounds) {
      return (score, 1); // Return 1 instead of 0 for empty bounds
    }
    let mut cluster = 1; // Start with cluster 1

    for bound in bounds {
      if score <= *bound {
        break;
      }
      cluster += 1;
    }
    (score, cluster)
  }
}
