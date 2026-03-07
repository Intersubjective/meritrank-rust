use crate::data::*;
use crate::helpers::*;
use crate::node_registry::*;
use crate::utils::{log::*, quantiles::*};

use meritrank_core::{NodeId, Weight};

use super::AugGraph;

impl AugGraph {
  pub fn update_node_score_clustering(
    &self,
    ego: NodeId,
    kind: NodeKind,
  ) -> super::ClusterGroupBounds {
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
    log_trace!("{} {} {:?}", ego_id, score, kind);

    if score < f64::EPSILON {
      //  Clusterize only positive scores.
      return (score, 0);
    }

    let bounds: &Vec<Weight> = &self
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

  pub fn read_scores(
    &self,
    data: OpReadScores,
  ) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let ego = data.ego;
    let filter_options = data.score_options;

    if let Some(ego_info) = self.nodes.get_by_name(&ego) {
      if !self.ensure_ego_is_user(&ego, ego_info) {
        return vec![];
      }
      let scores = self.fetch_all_scores(ego_info);
      self.apply_filters_and_pagination(
        scores,
        ego_info,
        &filter_options,
        false,
      )
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

    if !self.ensure_ego_is_user(&ego, ego_info) {
      return vec![];
    }

    let dst_id = match self.nodes.get_by_name(&dst) {
      Some(x) => x.id,
      None => {
        log_error!("Node not found: {:?}", dst);
        return vec![];
      },
    };

    let (score, cluster) = self.apply_score_clustering(
      ego_info.id,
      self.fetch_raw_score(ego_info.id, dst_id),
      ego_info.kind,
    );
    let (reverse_score, reverse_cluster) =
      match self.get_object_owner(dst_id) {
        Some(dst_owner_id) => self.fetch_score_cached(dst_owner_id, ego_info.id),
        None => (0.0, 0),
      };

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
    self.apply_score_clustering(
      ego,
      self.fetch_raw_score(ego, dst),
      self.nodes.id_to_info[ego].kind,
    )
  }

  pub(crate) fn apply_filters_and_pagination(
    &self,
    scores: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    filter_options: &FilterOptions,
    prioritize_ego_owned_nodes: bool,
  ) -> Vec<ScoreResult> {
    let mut filtered_sorted_scores =
      filter_and_sort_scores(scores, ego_info, filter_options);

    if prioritize_ego_owned_nodes {
      prioritize_ego_owned_items(&mut filtered_sorted_scores, ego_info);
    }

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
    let start = index as usize;
    let end = (index + count) as usize;

    items[start..end.min(items.len())]
      .iter()
      .map(|(target_info, score, cluster)| {
        let (reverse_score, reverse_cluster) =
          match self.get_object_owner(target_info.id) {
            Some(owner_id) => self.fetch_score_cached(owner_id, ego_info.id),
            None => (0.0, 0),
          };
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

  pub fn fetch_score_cached(
    &self,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{} {}", dst_id, ego_id);

    let score = match self.cached_scores.get(&(ego_id, dst_id)) {
      Some(score) => self.with_zero_opinion(dst_id, score),
      None => self.fetch_raw_score(ego_id, dst_id),
    };

    let kind_opt = self
      .nodes
      .get_by_id(dst_id)
      .and_then(|node_info| Some(node_info.kind));

    if let Some(kind) = kind_opt {
      self.apply_score_clustering(ego_id, score, kind)
    } else {
      (score, 0) // Default cluster if kind is None
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
          let cluster = self
            .apply_score_clustering(ego_info.id, *score, node_info.kind)
            .1;
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

    match self.mr.get_node_score(ego_id, dst_id) {
      Ok(score) => {
        self.cached_scores.insert((ego_id, dst_id), score);
        self.with_zero_opinion(dst_id, score)
      },
      Err(e) => {
        log_error!("Failed to get node score: {}", e);
        0.0
      },
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

    match self.mr.get_all_scores(ego_id, None) {
      Ok(scores) => {
        for (dst_id, score) in &scores {
          self.cached_scores.insert((ego_id, *dst_id), *score);
        }
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
        log_error!("{}", e);
        vec![]
      },
    }
  }
}
