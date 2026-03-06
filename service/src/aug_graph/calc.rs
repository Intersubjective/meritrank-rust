use crate::data::*;
use crate::node_registry::*;
use crate::utils::log::*;

use meritrank_core::{constants::EPSILON, NodeId, Weight};
use moka::sync::Cache;
use simple_pagerank::Pagerank;

use super::AugGraph;

impl AugGraph {
  pub fn calculate(
    &mut self,
    ego: NodeName,
  ) {
    log_trace!("{:?}", ego);

    let kind = match node_kind_from_prefix(&ego) {
      Some(x) => x,
      None => {
        log_error!("Failed to get node kind for {:?}", ego);
        return;
      },
    };

    if kind != NodeKind::User {
      log_error!("Non-user node used as ego for calculation (rejected): {:?}", ego);
      return;
    }

    let ego_id = self.nodes.register(&mut self.mr, ego, kind);

    match self.mr.calculate(ego_id, self.settings.num_walks) {
      Ok(_) => {},
      Err(e) => log_error!("{}", e),
    };
  }

  pub fn reduced_graph(
    &mut self,
    infos: &[NodeInfo],
    num_walks: usize,
  ) -> Vec<(NodeId, NodeId, Weight)> {
    log_trace!();

    let mut all_edges = vec![];

    for src in 0..infos.len() {
      if let Some(data) = self.mr.graph.get_node_data(src) {
        for (dst, _) in &data.pos_edges {
          all_edges.push((src, *dst));
        }

        for (dst, _) in &data.neg_edges {
          all_edges.push((src, *dst));
        }
      }
    }

    let users: Vec<NodeId> = infos
      .iter()
      .enumerate()
      .filter(|(_id, info)| info.kind == NodeKind::User)
      .filter(|(id, _info)| {
        for (src, dst) in &all_edges {
          if *id == *src || *id == *dst {
            return true;
          }
        }
        false
      })
      .map(|(id, _info)| id)
      .collect();

    if users.is_empty() {
      return vec![];
    }

    for id in users.iter() {
      match self.mr.calculate(*id, num_walks) {
        Ok(_) => {},
        Err(e) => log_error!("{}", e),
      };
    }

    let edges: Vec<(NodeId, NodeId, Weight)> = users
      .into_iter()
      .flat_map(|id| -> Vec<(NodeId, NodeId, Weight)> {
        self
          .fetch_all_raw_scores(id, 0.0)
          .into_iter()
          .map(|(node_id, score)| (id, node_id, score))
          .filter(|(ego_id, node_id, score)| {
            let kind = infos[*node_id].kind;

            (kind == NodeKind::User || kind == NodeKind::Beacon)
              && *score > EPSILON
              && ego_id != node_id
          })
          .collect()
      })
      .collect();

    let result: Vec<(NodeId, NodeId, Weight)> = edges
      .into_iter()
      .map(|(ego_id, dst_id, weight)| {
        let ego_kind = infos[ego_id].kind;
        let dst_kind = infos[dst_id].kind;
        (ego_id, ego_kind, dst_id, dst_kind, weight)
      })
      .filter(|(ego_id, ego_kind_opt, dst_id, dst_kind_opt, _)| {
        ego_id != dst_id
          && *ego_kind_opt == NodeKind::User
          && (*dst_kind_opt == NodeKind::User
            || *dst_kind_opt == NodeKind::Beacon)
      })
      .map(|(ego_id, _, dst_id, _, weight)| (ego_id, dst_id, weight))
      .collect();

    result
  }

  pub fn top_nodes(
    &mut self,
    infos: &[NodeInfo],
    top_nodes_limit: usize,
    num_walks: usize,
  ) -> Vec<(NodeId, f64)> {
    log_trace!();

    let reduced = self.reduced_graph(infos, num_walks);

    if reduced.is_empty() {
      log_error!("Reduced graph is empty");
      return vec![];
    }

    // TODO: remove PageRank in favor of direct sum of scores
    // Actually, instead of calculating page rank it is
    // possible to just sum all the scores by each user for each other.
    // The result should be the same after normalization.
    let mut pr = Pagerank::<NodeId>::new();

    reduced
      .iter()
      .filter(|(_src, _dst, weight)| !(*weight > -EPSILON && *weight < EPSILON))
      .for_each(|(src, dst, _weight)| {
        pr.add_edge(*src, *dst);
      });

    log_verbose!("Calculate page rank");
    pr.calculate();

    let (nodes, scores): (Vec<NodeId>, Vec<f64>) = pr
        .nodes()  // already sorted by score
        .into_iter()
        .take(top_nodes_limit)
        .unzip();

    let res = nodes.into_iter().zip(scores).collect::<Vec<_>>();

    if res.is_empty() {
      log_error!("No top nodes");
    }

    res
  }

  pub fn recalculate_all_users(
    &mut self,
    infos: &[NodeInfo],
    num_walk: usize,
  ) {
    log_trace!("{}", num_walk);

    for id in 0..infos.len() {
      if (id % 100) == 90 {
        log_verbose!("{}%", (id * 100) / infos.len());
      }
      if infos[id].kind == NodeKind::User {
        match self.mr.calculate(id, num_walk) {
          Ok(_) => {},
          Err(e) => log_error!("{}", e),
        };
      }
    }
  }

  pub(crate) fn recalculate_zero(&mut self) {
    log_command!();

    let infos = self.nodes.id_to_info.clone();

    //  Save the current state of the graph
    let data_bak = self.mr.clone();

    self.recalculate_all_users(&infos, 0);
    let nodes = self.top_nodes(
      &infos,
      self.settings.top_nodes_limit,
      self.settings.zero_opinion_num_walks,
    );

    //  Drop all walks and make sure to empty caches.
    self.recalculate_all_users(&infos, 0);
    self.cached_scores = Cache::new(self.settings.scores_cache_size as u64);
    // self.cached_walks = LruCache::new(self.settings.walks_cache_size);

    self.zero_opinion = vec![];
    self.zero_opinion.reserve(nodes.len());

    for (node_id, amount) in nodes.iter() {
      if *node_id >= self.zero_opinion.len() {
        self.zero_opinion.resize(*node_id + 1, 0.0);
      }
      self.zero_opinion[*node_id] = *amount;
    }

    //  Reset the graph
    self.mr = data_bak;
  }
}
