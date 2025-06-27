pub const NUM_SCORE_QUANTILES: usize = 100;

pub type NodeCluster = usize;

use meritrank_core::{NodeId, Weight};
use crate::aug_graph::AugGraph;
use crate::aug_graph::nodes::{nodes_by_kind, NodeKind, ALL_NODE_KINDS};
use crate::log::*;
use crate::utils::quantiles::bounds_are_empty;

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
          ego, kind, time_secs, node_count, k, &node_ids,
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

    let k = self.settings.zero_opinion_factor;
    let node_count = self.node_count;
    let node_ids = self.nodes_by_kind(kind);

    self._update_node_score_clustering(ego, kind, node_count, k, &node_ids)
  }

  pub fn _update_node_score_clustering(
    &self,
    ego: NodeId,
    kind: NodeKind,
    node_count: usize,
    zero_opinion_factor: f64,
    node_ids: &[NodeId],
  ) {
    log_trace!(
      "{} {:?} {} {} {}",
      ego,
      kind,
      node_count,
      self.settings.num_walks,
      zero_opinion_factor
    );

    if ego >= node_count {
      log_error!("Node does not exist: {}", ego);
      return;
    }

    let bounds = self.calculate_score_clusters_bounds(
      ego,
      kind,
      zero_opinion_factor,
      node_ids,
    );

    let new_cluster = ScoreCluster {
      bounds,
    };

    self.cached_score_clusters.insert((ego, kind), new_cluster);
  }

  fn calculate_score_clusters_bounds(
    &self,
    ego: NodeId,
    kind: NodeKind,
    zero_opinion_factor: f64,
    node_ids: &[NodeId],
  ) -> Vec<Weight> {
    log_trace!(
      "{} {:?} {} {}",
      ego,
      kind,
      self.settings.num_walks,
      zero_opinion_factor
    );

    let scores: Vec<Weight> = node_ids
      .iter()
      .map(|dst| self.fetch_raw_score(ego, *dst, zero_opinion_factor))
      .filter(|score| *score >= EPSILON)
      .collect();

    if scores.is_empty() {
      return vec![0.0; NUM_SCORE_QUANTILES - 1];
    }

    calculate_quantiles_bounds(scores, NUM_SCORE_QUANTILES)
  }
}
