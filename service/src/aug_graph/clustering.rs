pub const NUM_SCORE_QUANTILES: usize = 100;

pub type NodeCluster = usize;

use crate::aug_graph::nodes::{NodeKind, ALL_NODE_KINDS};
use crate::aug_graph::read::NodeScore;
use crate::aug_graph::AugGraph;
use crate::log::*;
use crate::utils::quantiles::bounds_are_empty;
use meritrank_core::{NodeId, Weight};

type ClusterGroupBounds = Vec<NodeScore>;

pub type ScoreCluster = i32;

impl AugGraph {
  fn init_node_score_clustering(
    &mut self,
    ego: NodeId,
  ) {
    log_trace!("{}", ego);

    let node_count = self.node_count;
    let time_secs = self.time_begin.elapsed().as_secs();
    let node_infos = self.node_infos.clone();

    self
      .cached_score_clusters
      .resize(node_count, Default::default());

    for kind in ALL_NODE_KINDS {
      if bounds_are_empty(&self.cached_score_clusters[ego][kind].bounds) {
        let node_ids = nodes_by_kind(kind, &node_infos);

        let k = self.settings.zero_opinion_factor;
        self._update_node_score_clustering(
          ego, kind, time_secs, node_count, &node_ids,
        );
      }
    }
  }

  pub fn update_node_score_clustering(
    &self,
    ego: NodeId,
    kind: NodeKind,
  ) {
    log_trace!("{} {:?}", ego, kind);
    let node_count = self.node_count;
    let node_ids = self.nodes_by_kind(kind);

    log_trace!(
      "{} {:?} {} {}",
      ego,
      kind,
      node_count,
      self.settings.num_walks
    );

    if ego >= node_count {
      log_error!("Node does not exist: {}", ego);
      return;
    }

    let bounds = self.calculate_score_clusters_bounds(ego, kind, node_ids);

    let new_cluster = ScoreCluster {
      bounds,
    };

    self.cached_score_clusters.insert((ego, kind), new_cluster);
  }

  fn calculate_score_clusters_bounds(
    &self,
    ego: NodeId,
    kind: NodeKind,
    node_ids: &[NodeId],
  ) -> Vec<Weight> {
    log_trace!("{} {:?} {}", ego, kind, self.settings.num_walks);

    let scores: Vec<Weight> = node_ids
      .iter()
      .map(|dst| self.fetch_raw_score(ego, *dst))
      .filter(|score| *score >= f64::EPSILON)
      .collect();

    if scores.is_empty() {
      return vec![0.0; NUM_SCORE_QUANTILES - 1];
    }

    calculate_quantiles_bounds(scores, NUM_SCORE_QUANTILES)
  }

  pub fn apply_score_clustering(
    &self,
    ego_id: NodeId,
    score: NodeScore,
    kind: NodeKind,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{:?} {} {}", context, ego_id, score);

    if score < f64::EPSILON {
      //  Clusterize only positive scores.
      return (score, 0);
    }

    self.init_node_score_clustering(context, ego_id);

    let elapsed_secs = self.time_begin.elapsed().as_secs();

    let clusters = &self.cached_score_clusters;

    let updated_sec = clusters[ego_id][kind].updated_sec;

    if elapsed_secs >= updated_sec + self.settings.score_clusters_timeout {
      log_verbose!("Recalculate clustering for node {}",ego_id);
      self.update_node_score_clustering(ego_id, kind);
    }

    let clusters = &self.cached_score_clusters;

    let bounds = &clusters[ego_id][kind].bounds;

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
