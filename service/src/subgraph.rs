use lru::LruCache;
use meritrank_core::{constants::EPSILON, MeritRank, NodeId, Weight};

use crate::constants::*;
use crate::log::*;
use crate::nodes::*;
use crate::quantiles::*;

#[derive(Clone)]
pub struct Subgraph {
  pub meritrank_data:        MeritRank,
  pub zero_opinion:          Vec<Weight>,
  pub cached_scores:         LruCache<(NodeId, NodeId), Weight>,
  pub cached_walks:          LruCache<NodeId, ()>,
  pub cached_score_clusters: Vec<ScoreClustersByKind>,
}

impl Subgraph {
  pub fn cache_score_add(
    &mut self,
    ego: NodeId,
    dst: NodeId,
    score: Weight,
  ) {
    log_trace!("{} {} {}", ego, dst, score);
    self.cached_scores.put((ego, dst), score);
  }

  pub fn cache_score_get(
    &mut self,
    ego: NodeId,
    dst: NodeId,
  ) -> Option<Weight> {
    log_trace!("{} {}", ego, dst);
    self.cached_scores.get(&(ego, dst)).copied()
  }

  pub fn cache_walk_add(
    &mut self,
    ego: NodeId,
  ) {
    log_trace!("{}", ego);

    if let Some((old_ego, _)) = self.cached_walks.push(ego, ()) {
      if old_ego != ego {
        log_verbose!("Drop walks {}", old_ego);

        // HACK!!!
        // We "drop" the walks by recalculating the node with 0.
        match self.meritrank_data.calculate(old_ego, 0) {
          Ok(()) => {},
          Err(e) => {
            log_error!("{}", e);
          },
        }
      }
    }
  }

  pub fn cache_walk_get(
    &mut self,
    ego: NodeId,
  ) -> bool {
    log_trace!();

    self.cached_walks.get(&ego).is_some()
  }

  pub fn edge_weight(
    &mut self,
    src: NodeId,
    dst: NodeId,
  ) -> Weight {
    log_trace!("{} {}", src, dst);

    self
      .meritrank_data
      .graph
      .edge_weight(src, dst)
      .unwrap_or(None)
      .unwrap_or(0.0)
  }

  pub fn edge_weight_normalized(
    &self,
    src: NodeId,
    dst: NodeId,
  ) -> Weight {
    log_trace!("{} {}", src, dst);

    let pos_sum = match self.meritrank_data.graph.get_node_data(src) {
      Some(x) => {
        if x.pos_sum < EPSILON {
          log_warning!(
            "Unable to normalize node weight, positive sum is zero."
          );
          1.0
        } else {
          x.pos_sum
        }
      },

      None => 1.0,
    };

    self
      .meritrank_data
      .graph
      .edge_weight(src, dst)
      .unwrap_or(None)
      .unwrap_or(0.0)
      / pos_sum
  }

  pub fn all_outbound_neighbors_normalized(
    &self,
    node: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{}", node);

    let mut v = vec![];

    match self.meritrank_data.graph.get_node_data(node) {
      None => {},
      Some(data) => {
        v.reserve_exact(data.pos_edges.len() + data.neg_edges.len());

        let abs_sum = if data.pos_sum < EPSILON {
          log_warning!(
            "Unable to normalize node weight, positive sum is zero."
          );
          1.0
        } else {
          data.abs_sum()
        };

        for x in &data.pos_edges {
          v.push((*x.0, *x.1 / abs_sum));
        }

        for x in &data.neg_edges {
          v.push((*x.0, -*x.1 / abs_sum));
        }
      },
    }
    v
  }

  pub fn with_zero_opinion(
    &mut self,
    dst_id: NodeId,
    score: Weight,
    zero_opinion_factor: f64,
  ) -> Weight {
    log_trace!("{} {}", dst_id, score);

    let zero_score = match self.zero_opinion.get(dst_id) {
      Some(x) => *x,
      _ => 0.0,
    };
    score * (1.0 - zero_opinion_factor) + zero_opinion_factor * zero_score
  }

  fn with_zero_opinions(
    &self,
    scores: Vec<(NodeId, Weight)>,
    zero_opinion_factor: f64,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{}", zero_opinion_factor);

    let k = zero_opinion_factor;

    let mut res: Vec<(NodeId, Weight)> = vec![];
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

    return res
      .into_iter()
      .filter(|(_id, score)| *score != 0.0)
      .collect::<Vec<_>>();
  }

  pub fn fetch_all_raw_scores(
    &mut self,
    ego_id: NodeId,
    num_walks: usize,
    zero_opinion_factor: f64,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{} {} {}", ego_id, num_walks, zero_opinion_factor);

    if self.cache_walk_get(ego_id) {
      let data = &self.meritrank_data;
      match data.get_ranks(ego_id, None) {
        Ok(scores) => {
          for (dst_id, score) in &scores {
            self.cache_score_add(ego_id, *dst_id, *score);
          }
          self.with_zero_opinions(scores, zero_opinion_factor)
        },
        Err(e) => {
          log_error!("{}", e);
          vec![]
        },
      }
    } else {
      match self.meritrank_data.calculate(ego_id, num_walks) {
        Ok(()) => {
          self.cache_walk_add(ego_id);
        },
        Err(e) => {
          log_error!("{}", e);
          return vec![];
        },
      }
      match self.meritrank_data.get_ranks(ego_id, None) {
        Ok(scores) => {
          for (dst_id, score) in &scores {
            self.cache_score_add(ego_id, *dst_id, *score);
          }
          self.with_zero_opinions(scores, zero_opinion_factor)
        },
        Err(e) => {
          log_error!("{}", e);
          vec![]
        },
      }
    }
  }

  pub fn fetch_raw_score(
    &mut self,
    ego_id: NodeId,
    dst_id: NodeId,
    num_walks: usize,
    zero_opinion_factor: f64,
  ) -> Weight {
    log_trace!(
      "{} {} {} {}",
      ego_id,
      dst_id,
      num_walks,
      zero_opinion_factor
    );

    if !self.cache_walk_get(ego_id) {
      if let Err(e) = self.meritrank_data.calculate(ego_id, num_walks) {
        log_error!("Failed to calculate: {}", e);
        return 0.0;
      }
      self.cache_walk_add(ego_id);
    }

    match self.meritrank_data.get_node_score(ego_id, dst_id) {
      Ok(score) => {
        self.cache_score_add(ego_id, dst_id, score);
        self.with_zero_opinion(dst_id, score, zero_opinion_factor)
      },
      Err(e) => {
        log_error!("Failed to get node score: {}", e);
        0.0
      },
    }
  }

  fn calculate_score_clusters_bounds(
    &mut self,
    ego: NodeId,
    kind: NodeKind,
    num_walks: usize,
    zero_opinion_factor: f64,
    node_ids: &[NodeId],
  ) -> [Weight; NUM_SCORE_QUANTILES - 1] {
    log_trace!("{} {:?} {} {}", ego, kind, num_walks, zero_opinion_factor);

    let scores: Vec<Weight> = node_ids
      .into_iter()
      .map(|dst| {
        self.fetch_raw_score(ego, *dst, num_walks, zero_opinion_factor)
      })
      .filter(|score| *score >= EPSILON)
      .collect();

    if scores.is_empty() {
      return [0.0; NUM_SCORE_QUANTILES - 1];
    }

    calculate_quantiles_bounds(scores)
  }

  pub fn update_node_score_clustering(
    &mut self,
    ego: NodeId,
    kind: NodeKind,
    time_secs: u64,
    node_count: usize,
    num_walks: usize,
    zero_opinion_factor: f64,
    node_ids: &[NodeId],
  ) {
    log_trace!(
      "{} {:?} {} {} {} {}",
      ego,
      kind,
      time_secs,
      node_count,
      num_walks,
      zero_opinion_factor
    );

    if ego >= node_count {
      log_error!("Node does not exist: {}", ego);
      return;
    }

    let bounds = self.calculate_score_clusters_bounds(
      ego,
      kind,
      num_walks,
      zero_opinion_factor,
      node_ids,
    );

    self.cached_score_clusters.resize(node_count, Default::default());

    self.cached_score_clusters[ego][kind].updated_sec = time_secs;
    self.cached_score_clusters[ego][kind].bounds = bounds;
  }
}
