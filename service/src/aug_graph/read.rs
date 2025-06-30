use crate::aug_graph::clustering::NodeCluster;
use crate::aug_graph::node_registry::NodeInfo;
use crate::aug_graph::nodes::NodeKind;
use crate::aug_graph::{AugGraph, NodeName, NodeScore};
use bincode::{Decode, Encode};
use meritrank_core::{NodeId, Weight};

use crate::log::*;

#[derive(Debug, Clone)]
pub struct ScoreResult {
  pub ego:             NodeName,
  pub target:          NodeName,
  pub score:           NodeScore,
  pub reverse_score:   NodeScore,
  pub cluster:         NodeCluster,
  pub reverse_cluster: NodeCluster,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NeighborDirection {
  All,
  Outbound,
  Inbound,
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
      if ego_info.kind != NodeKind::User {
        log_warning!("Trying to use non-user as ego {}", ego);
        return vec![];
      }
      let scores = self.fetch_all_scores(ego_info);
      self.apply_filters_and_pagination(scores, ego_info, filter_options, false)
    } else {
      log_warning!("Ego not found: {:?}", ego);
      vec![]
    }
  }

  fn apply_filters_and_pagination(
    &self,
    scores: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    filter_options: &FilterOptions,
    prioritize_ego_owned_nodes: bool,
  ) -> Vec<ScoreResult> {
    let mut filtered_sorted_scores =
      self._filter_and_sort_scores(scores, ego_info, filter_options);

    if prioritize_ego_owned_nodes {
      self._prioritize_ego_owned_items(&mut filtered_sorted_scores, ego_info);
    }

    self._paginate_and_format_items(
      filtered_sorted_scores,
      ego_info,
      filter_options.index,
      filter_options.count,
    )
  }

  fn _filter_and_sort_scores(
    &self,
    scores: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    filter_options: &FilterOptions,
  ) -> Vec<(NodeInfo, NodeScore, NodeCluster)> {
    let mut filtered_scores: Vec<(NodeInfo, NodeScore, NodeCluster)> = scores
      .into_iter()
      .filter(|(node_info, score, _)| {
        // Apply kind filter
        filter_options
          .node_kind
          .map_or(true, |filter_kind| node_info.kind == filter_kind)
          && !(filter_options.hide_personal
            && node_info.owner == Some(ego_info.id))
          && {
            // Apply score filters
            (*score > filter_options.score_gt
              || (!filter_options.score_gte
                && *score >= filter_options.score_gt))
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
    items: &mut Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
  ) {
    let mut insert_index = 0;
    for i in 0..items.len() {
      if let Some(owner) = items[i].0.owner {
        if owner == ego_info.id {
          items.swap(i, insert_index);
          insert_index += 1;
        }
      }
    }
  }

  fn _paginate_and_format_items(
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
          self.fetch_score_cached(target_info.id, ego_info.id);
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

  fn fetch_all_scores(
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

  fn fetch_all_raw_scores(
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

    pub fn fetch_neighbors(
        &self,
        ego_id: NodeId,
        focus_id: NodeId,
        dir: NeighborDirection,
    ) -> Vec<(NodeId, Weight, NodeCluster)> {
        log_trace!("{} {} {:?}", ego_id, focus_id, dir);

        let node_data = match self.mr.graph.get_node_data(focus_id) {
            Some(data) => data,
            None => {
                log_warning!("Node not found: {}", focus_id);
                return vec![];
            }
        };

        let edges: Vec<_> = match dir {
            NeighborDirection::Outbound => node_data.pos_edges.iter().collect(),
            NeighborDirection::Inbound => node_data.neg_edges.iter().collect(),
            NeighborDirection::All => node_data.pos_edges.iter().chain(node_data.neg_edges.iter()).collect(),
        };

        edges.into_iter()
            .map(|(dst_id, &weight)| {
                let (_score, cluster) = self.fetch_score_cached(ego_id, *dst_id);
                (*dst_id, weight, cluster)
            })
            .collect()
    }
}
