use crate::data::*;
use crate::helpers::*;
use crate::node_registry::*;
use crate::utils::{log::*, quantiles::*};

use meritrank_core::{NodeId, Weight};

use super::{snapshots::scope_bounds, AugGraph};

impl AugGraph {
  /// Recomputes and caches the ego's cluster bounds.
  pub fn update_node_score_clustering(
    &self,
    ego: NodeId,
  ) -> super::ClusterGroupBounds {
    log_trace!("{}", ego);
    let bounds = self.calculate_score_clusters_bounds(ego);
    self.cached_score_clusters.insert(self.cluster_key(ego), bounds.clone());
    bounds
  }

  /// Quantile bounds of the ego's positive scores over every node (one node class, D14).
  fn calculate_score_clusters_bounds(
    &self,
    ego: NodeId,
  ) -> Vec<NodeScore> {
    log_trace!("{}", ego);

    let scores: Vec<NodeScore> = self
      .score_candidates(ego)
      .into_iter()
      .map(|dst| self.fetch_raw_score(ego, dst))
      .filter(|score| *score >= f64::EPSILON)
      .collect();

    if scores.is_empty() {
      return vec![0.0; self.settings.num_score_quantiles - 1];
    }

    calculate_quantiles_bounds(scores, self.settings.num_score_quantiles)
  }

  /// The nodes that can score non-zero in the ego's frame: those of its walks (frame, snapshot or
  /// read-local sample) and those with a zero opinion. Every other node scores exactly 0, so the
  /// bounds over these equal the bounds over every node (the quantiles sort their input) — at
  /// O(footprint) instead of O(nodes).
  fn score_candidates(
    &self,
    ego: NodeId,
  ) -> Vec<NodeId> {
    let mut nodes: Vec<NodeId> = match self.walk_nodes(ego) {
      Some(v) => v,
      None => return (0..self.nodes.len()).collect(),
    };
    if self.settings.zero_opinion_factor != 0.0 {
      nodes.extend(
        self
          .zero_opinion
          .iter()
          .enumerate()
          .filter(|(_, z)| **z != 0.0)
          .map(|(id, _)| id),
      );
    }
    nodes.retain(|n| *n < self.nodes.len());
    nodes.sort_unstable();
    nodes.dedup();
    nodes
  }

  /// The ego's cluster bounds: from the cache (keyed by the ego's revision and the zero-opinion
  /// revision), or — for an ego sampled by the current read — computed for this read only.
  fn cluster_bounds(
    &self,
    ego: NodeId,
  ) -> Vec<Weight> {
    if !self.mr.is_calculated(ego) {
      if let Some(b) = self.snapshots.get(ego).and_then(|s| s.bounds_at(self.zero_revision)) {
        return b.clone();
      }
    }
    if let Some(b) = self.cached_score_clusters.get(&self.cluster_key(ego)) {
      if self.mr.is_calculated(ego) || self.snapshots.contains(ego) {
        return b;
      }
    }
    if !self.mr.is_calculated(ego) && !self.snapshots.contains(ego) {
      // A read-local sample: its bounds are not cached across reads (another read may sample
      // the ego on a changed graph under the same revision).
      if let Some(b) = scope_bounds(ego, || self.calculate_score_clusters_bounds(ego)) {
        return b;
      }
    }
    self.update_node_score_clustering(ego)
  }

  pub fn apply_score_clustering(
    &self,
    ego_id: NodeId,
    score: NodeScore,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{} {}", ego_id, score);

    if score < f64::EPSILON {
      //  Clusterize only positive scores.
      return (score, 0);
    }

    let bounds = self.cluster_bounds(ego_id);

    if bounds_are_empty(&bounds) {
      return (score, 1); // Return 1 instead of 0 for empty bounds
    }
    let mut cluster = 1; // Start with cluster 1

    for bound in &bounds {
      if score <= *bound {
        break;
      }
      cluster += 1;
    }
    (score, cluster)
  }

  pub fn read_scores(
    &self,
    data: OpReadScores,
  ) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let ego = data.ego;
    let filter_options = data.score_options;

    if let Some(ego_info) = self.nodes.get_by_name(&ego) {
      let scores = self.fetch_all_scores(ego_info);
      self.apply_filters_and_pagination(scores, ego_info, &filter_options)
    } else {
      // Ego not in this context's graph (no edges involving this user were written here).
      log_warning!("Ego not found in context (no scores): {:?}", ego);
      vec![]
    }
  }

  pub fn read_node_score(
    &self,
    data: OpReadNodeScore,
  ) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let ego = data.ego;
    let dst = data.target;

    let ego_info = match self.nodes.get_by_name(&ego) {
      Some(x) => x,
      None => {
        log_error!("Node not found: {:?}", ego);
        return vec![];
      },
    };

    let dst_id = match self.nodes.get_by_name(&dst) {
      Some(x) => x.id,
      None => {
        log_error!("Node not found: {:?}", dst);
        return vec![];
      },
    };

    let (score, cluster) =
      self.apply_score_clustering(ego_info.id, self.fetch_raw_score(ego_info.id, dst_id));
    let (reverse_score, reverse_cluster) = self.fetch_score_clustered(dst_id, ego_info.id);

    vec![ScoreResult {
      ego: ego.into(),
      target: dst.into(),
      score,
      reverse_score,
      cluster,
      reverse_cluster,
    }]
  }

  pub(crate) fn fetch_score(
    &self,
    ego: NodeId,
    dst: NodeId,
  ) -> (NodeScore, NodeCluster) {
    self.apply_score_clustering(ego, self.fetch_raw_score(ego, dst))
  }

  pub(crate) fn apply_filters_and_pagination(
    &self,
    scores: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    filter_options: &FilterOptions,
  ) -> Vec<ScoreResult> {
    let filtered_sorted_scores = filter_and_sort_scores(scores, filter_options);

    self.paginate_and_format_items(
      filtered_sorted_scores,
      ego_info,
      filter_options.index,
      filter_options.count,
    )
  }

  fn paginate_and_format_items(
    &self,
    items: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    index: u32,
    count: u32,
  ) -> Vec<ScoreResult> {
    // Client-supplied: clamp instead of slicing out of range or overflowing.
    let start = (index as usize).min(items.len());
    let end = (index as usize).saturating_add(count as usize).min(items.len());

    items[start..end]
      .iter()
      .map(|(target_info, score, cluster)| {
        let (reverse_score, reverse_cluster) =
          self.fetch_score_clustered(target_info.id, ego_info.id);
        ScoreResult {
          ego: ego_info.name.clone(),
          target: target_info.name.clone(),
          score: *score,
          reverse_score,
          cluster: *cluster,
          reverse_cluster,
        }
      })
      .collect()
  }

  /// Score of `dst_id` in `ego_id`'s frame, read from the frame's counters, with its cluster.
  pub fn fetch_score_clustered(
    &self,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{} {}", dst_id, ego_id);

    let score = self.fetch_raw_score(ego_id, dst_id);

    if self.nodes.get_by_id(dst_id).is_some() {
      self.apply_score_clustering(ego_id, score)
    } else {
      (score, 0)
    }
  }

  pub(crate) fn fetch_all_scores(
    &self,
    ego_info: &NodeInfo,
  ) -> Vec<(NodeInfo, NodeScore, NodeCluster)> {
    log_trace!("{}", ego_info.id);
    self
      .fetch_all_raw_scores(ego_info.id, self.settings.zero_opinion_factor)
      .iter()
      .filter_map(|(dst_id, score)| {
        self.nodes.get_by_id(*dst_id).map(|node_info| {
          let cluster = self.apply_score_clustering(ego_info.id, *score).1;
          (node_info.clone(), *score, cluster)
        })
      })
      .collect()
  }

  pub fn with_zero_opinion(
    &self,
    dst_id: NodeId,
    score: NodeScore,
  ) -> NodeScore {
    log_trace!("{} {}", dst_id, score);

    let zero_score = match self.zero_opinion.get(dst_id) {
      Some(x) => *x,
      _ => 0.0,
    };
    let k = self.settings.zero_opinion_factor;
    score * (1.0 - k) + k * zero_score
  }

  fn with_zero_opinions(
    &self,
    scores: Vec<(NodeId, NodeScore)>,
  ) -> Vec<(NodeId, NodeScore)> {
    let k = self.settings.zero_opinion_factor;

    let mut res: Vec<(NodeId, NodeScore)> = vec![];
    res.resize(self.zero_opinion.len(), (0, 0.0));

    for (id, zero_score) in self.zero_opinion.iter().enumerate() {
      res[id] = (id, zero_score * k);
    }

    for (id, score) in scores.iter() {
      if *id >= res.len() {
        let n = res.len();
        res.resize(id + 1, (0, 0.0));
        for id in n..res.len() {
          res[id].0 = id;
        }
      }
      res[*id].1 += (1.0 - k) * score;
    }

    res
      .into_iter()
      .filter(|(_id, score)| *score != 0.0)
      .collect::<Vec<_>>()
  }

  pub fn fetch_raw_score(
    &self,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> NodeScore {
    log_trace!("{} {} {}", ego_id, dst_id, self.settings.num_walks);

    // From the resident frame, a snapshot, or a read-local sample (D14).
    match self.walk_score(ego_id, dst_id) {
      Some(score) => self.with_zero_opinion(dst_id, score),
      None => 0.0,
    }
  }

  pub(crate) fn fetch_all_raw_scores(
    &self,
    ego_id: NodeId,
    zero_opinion_factor: f64,
  ) -> Vec<(NodeId, NodeScore)> {
    log_trace!(
      "{} {} {}",
      ego_id,
      self.settings.num_walks,
      zero_opinion_factor
    );

    super::record_frame_access(ego_id);
    match self.mr.get_all_scores(ego_id, None) {
      Ok(scores) => {
        let scores = self.with_zero_opinions(scores);

        // Filter out nodes that have a direct negative edge from ego
        if self.settings.omit_neg_edges_scores {
          let before = scores.len();
          let (kept, dropped): (Vec<_>, Vec<_>) = scores
            .into_iter()
            .partition(|(dst_id, _)| {
              match self.mr.graph.edge_weight(ego_id, *dst_id) {
                Ok(Some(weight)) => weight > 0.0,
                _ => true,
              }
            });
          if !dropped.is_empty() {
            log_trace!(
              "omit_neg_edges_scores: ego_id={} before={} kept={} dropped={} dropped_ids={:?}",
              ego_id,
              before,
              kept.len(),
              dropped.len(),
              dropped.iter().map(|(id, _)| *id).collect::<Vec<_>>()
            );
          }
          kept
        } else {
          scores
        }
      },
      Err(e) => {
        log_trace!("{}", e);
        vec![]
      },
    }
  }
}
