use crate::aug_graph::clustering::NodeCluster;
use crate::aug_graph::nodes::NodeKind;
use crate::aug_graph::AugGraph;
use bincode::{Decode, Encode};
use meritrank_core::NodeId;
pub type NodeName = String;
pub type NodeScore = f64;
use crate::log::*;
use crate::utils::quantiles::bounds_are_empty;

#[derive(Debug, Clone)]
pub struct ScoreResult {
  pub ego:             NodeName,
  pub target:          NodeName,
  pub score:           NodeScore,
  pub reverse_score:   NodeScore,
  pub cluster:         NodeCluster,
  pub reverse_cluster: NodeCluster,
}

#[derive(Debug, Encode, Decode)]
pub struct FilterOptions {
  node_kind:     Option<NodeKind>,
  hide_personal: bool,
  score_lt:      f64,
  score_lte:     bool,
  score_gt:      f64,
  score_gte:     bool,
  index:         u32,
  count:         u32,
}

impl Default for FilterOptions {
  fn default() -> Self {
    FilterOptions {
      node_kind:     None,
      hide_personal: false,
      score_lt:      f64::MAX,
      score_lte:     true,
      score_gt:      f64::MIN,
      score_gte:     true,
      index:         0,
      count:         u32::MAX,
    }
  }
}

impl AugGraph {
  pub fn read_scores(
    &self,
    ego: &str,
    filter_options: &FilterOptions,
  ) -> Vec<ScoreResult> {
    log_command!("{:?} {:?}", ego, filter_options);
    if let Some(ego_info) = self.nodes.get_by_name(ego) {
      let scores = self.fetch_all_scores(ego_info.id);
      self.apply_filters_and_pagination(scores, ego_info.id, filter_options, false)
    } else {
      log_error!("Ego not found: {:?}", ego);
      vec![]
    }
  }

  fn apply_filters_and_pagination(
    &self,
    scores: Vec<(NodeId, NodeScore, NodeCluster)>,
    ego_id: NodeId,
    filter_options: &FilterOptions,
    prioritize_ego_owned_nodes: bool,
  ) -> Vec<ScoreResult> {
    let mut filtered_sorted_scores =
      self._filter_and_sort_scores(scores, ego_id, filter_options);

    if prioritize_ego_owned_nodes {
      self._prioritize_ego_owned_items(&mut filtered_sorted_scores, ego_id);
    }

    self._paginate_and_format_items(
      filtered_sorted_scores,
      ego_id,
      filter_options.index,
      filter_options.count,
    )
  }

  fn _filter_and_sort_scores(
    &self,
    scores: Vec<(NodeId, NodeScore, NodeCluster)>,
    ego_id: NodeId,
    filter_options: &FilterOptions,
  ) -> Vec<(NodeId, NodeScore, NodeCluster)> {
    let mut filtered_scores: Vec<(NodeId, NodeScore, NodeCluster)> = scores
      .into_iter()
      .filter(|(node_id, score, _)| {
        self.nodes.get_by_id(*node_id).map_or(true, |node_info| {
          // Apply kind filter
          filter_options
            .node_kind
            .map_or(true, |filter_kind| node_info.kind == filter_kind)
            && !(filter_options.hide_personal
              && node_info.owner == Some(ego_id))
        }) && {
          // Apply score filters
          (*score > filter_options.score_gt
            || (!filter_options.score_gte && *score >= filter_options.score_gt))
            && (*score < filter_options.score_lt
              || (!filter_options.score_lte
                && *score <= filter_options.score_lt))
        }
      })
      .collect();

    filtered_scores.sort_by(|(_, a, _), (_, b, _)| b.abs().total_cmp(&a.abs()));
    filtered_scores
  }

  fn _prioritize_ego_owned_items(
    &self,
    items: &mut Vec<(NodeId, NodeScore, NodeCluster)>,
    ego_id: NodeId,
  ) {
    let mut insert_index = 0;
    for i in 0..items.len() {
      if let Some(node_info) = self.nodes.get_by_id(items[i].0) {
        if let Some(owner) = node_info.owner {
          if owner == ego_id {
            items.swap(i, insert_index);
            insert_index += 1;
          }
        }
      }
    }
  }

  fn _paginate_and_format_items(
    &self,
    items: Vec<(NodeId, NodeScore, NodeCluster)>,
    ego_id: NodeId,
    index: u32,
    count: u32,
  ) -> Vec<ScoreResult> {
    let start = index as usize;
    let end = (index + count) as usize;

    items[start..end.min(items.len())]
      .iter()
      .map(|(target_id, score, cluster)| {
        let (reverse_score, reverse_cluster) =
          self.fetch_score_cached(*target_id, ego_id);
        ScoreResult {
          ego: self.nodes.get_name(ego_id).unwrap_or_default(),
          target: self.nodes.get_name(*target_id).unwrap_or_default(),
          score: *score,
          reverse_score,
          cluster: *cluster,
          reverse_cluster,
        }
      })
      .collect()
  }
  pub fn apply_score_clustering(
    &self,
    ego_id: NodeId,
    score: NodeScore,
    kind: NodeKind,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{} {} {}", ego_id, score, kind);

    if score < f64::EPSILON {
      return (score, 0);
    }

    if self.nodes.get_kind_by_id(ego_id) != Some(NodeKind::User) {
      log_warning!("Trying to use non-user as ego {}", ego_id);
      return (score, 0);
    }

    let cluster = if let Some(score_cluster) =
      self.cached_score_clusters.get(&(ego_id, kind))
    {
      let bounds = &score_cluster.bounds;
      if bounds_are_empty(bounds) {
        1
      } else {
        bounds.iter().take_while(|&bound| score > *bound).count() + 1
      }
    } else {
      // If not in cache, trigger an update and return default cluster
      self.update_node_score_clustering(ego_id, kind);
      1
    };

    (score, cluster)
  }

  fn fetch_all_scores(
    &self,
    ego_id: NodeId,
  ) -> Vec<(NodeId, NodeScore, NodeCluster)> {
    log_trace!("{}", ego_id);
    self
      .fetch_all_raw_scores(ego_id, self.settings.zero_opinion_factor)
      .iter()
      .map(|(dst_id, score)| {
        let kind_opt = self.nodes.get_kind_by_id(*dst_id);
        let cluster = if let Some(kind) = kind_opt {
          self.apply_score_clustering(ego_id, *score, kind).1
        } else {
          0 // Default cluster for nodes with no kind
        };
        (*dst_id, *score, cluster)
      })
      .collect()
  }

  fn with_zero_opinions(
    &self,
    scores: Vec<(NodeId, NodeScore)>,
    zero_opinion_factor: f64,
  ) -> Vec<(NodeId, NodeScore)> {
    log_trace!("{}", zero_opinion_factor);

    let k = zero_opinion_factor;

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
    zero_opinion_factor: f64,
  ) -> NodeScore {
    log_trace!(
      "{} {} {} {}",
      ego_id,
      dst_id,
      self.settings.num_walks,
      zero_opinion_factor
    );

    match self.mr.get_node_score(ego_id, dst_id) {
      Ok(score) => {
        //self.cache_score_add(ego_id, dst_id, score);
        self.with_zero_opinion(dst_id, score, zero_opinion_factor)
      },
      Err(e) => {
        log_error!("Failed to get node score: {}", e);
        0.0
      },
    }
  }

  pub fn fetch_all_raw_scores(
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
        // TODO: CACHES
        /*
        for (dst_id, score) in &scores {
          self.cache_score_add(ego_id, *dst_id, *score);
        }
         */
        let scores = self.with_zero_opinions(scores, zero_opinion_factor);

        // Filter out nodes that have a direct negative edge from ego
        if self.settings.omit_neg_edges_scores {
          scores
            .into_iter()
            .filter(|(dst_id, _)| {
              // Check if there's a direct edge and if it's negative
              match self.mr.graph.edge_weight(ego_id, *dst_id) {
                Ok(Some(weight)) => weight > 0.0, // Keep only positive edges
                _ => true, // Keep if no direct edge exists
              }
            })
            .collect()
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
