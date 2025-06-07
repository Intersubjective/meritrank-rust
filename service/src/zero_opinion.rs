//  ================================================
//
//    Zero opinion recalculation
//
//  ================================================

use lru::LruCache;
use meritrank_core::{constants::EPSILON, NodeId};
use simple_pagerank::Pagerank;

use crate::aug_multi_graph::*;
use crate::log::*;
use crate::nodes::*;
use crate::subgraph::*;

impl Subgraph {
  pub fn reduced_graph(
    &mut self,
    infos: &[NodeInfo],
    num_walks: usize,
    zero_opinion_factor: f64,
  ) -> Vec<(NodeId, NodeId, Weight)> {
    log_trace!();

    let mut all_edges = vec![];

    for src in 0..infos.len() {
      if let Some(data) = self.meritrank_data.graph.get_node_data(src) {
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
      .filter(|(_id, info)| info.kind == Some(NodeKind::User)) // Updated comparison
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
      match self.meritrank_data.calculate(*id, num_walks) {
        Ok(_) => {},
        Err(e) => log_error!("{}", e),
      };
    }

    let edges: Vec<(NodeId, NodeId, Weight)> = users
      .into_iter()
      .flat_map(|id| -> Vec<(NodeId, NodeId, Weight)> {
        self
          .fetch_all_raw_scores(id, zero_opinion_factor)
          .into_iter()
          .map(|(node_id, score)| (id, node_id, score))
          .filter(|(ego_id, node_id, score)| {
            let kind_opt = node_kind_from_id(infos, *node_id); // node_kind_from_id returns Option

            (kind_opt == Some(NodeKind::User) || kind_opt == Some(NodeKind::Beacon)) // Updated comparison
              && *score > EPSILON
              && ego_id != node_id
          })
          .collect()
      })
      .collect();

    let result: Vec<(NodeId, NodeId, Weight)> = edges
      .into_iter()
      .map(|(ego_id, dst_id, weight)| {
        let ego_kind = node_kind_from_id(infos, ego_id);
        let dst_kind = node_kind_from_id(infos, dst_id);
        (ego_id, ego_kind, dst_id, dst_kind, weight)
      })
      .filter(|(ego_id, ego_kind_opt, dst_id, dst_kind_opt, _)| { // ego_kind and dst_kind are Option
        ego_id != dst_id
          && *ego_kind_opt == Some(NodeKind::User) // Updated comparison
          && (*dst_kind_opt == Some(NodeKind::User) || *dst_kind_opt == Some(NodeKind::Beacon)) // Updated comparison
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
    zero_opinion_factor: f64,
  ) -> Vec<(NodeId, f64)> {
    log_trace!();

    let reduced = self.reduced_graph(infos, num_walks, zero_opinion_factor);

    if reduced.is_empty() {
      log_error!("Reduced graph is empty");
      return vec![];
    }

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
      if infos[id].kind == Some(NodeKind::User) { // Updated comparison
        match self.meritrank_data.calculate(id, num_walk) {
          Ok(_) => {},
          Err(e) => log_error!("{}", e),
        };
      }
    }
  }
}

impl AugMultiGraph {
  pub fn write_recalculate_zero(&mut self) {
    log_command!();

    let infos = self.node_infos.clone();

    for subgraph in self.subgraphs.values_mut() {
      //  Save the current state of the graph
      let data_bak = subgraph.meritrank_data.clone();

      subgraph.recalculate_all_users(&infos, 0);
      let nodes = subgraph.top_nodes(
        &infos,
        self.settings.top_nodes_limit,
        self.settings.zero_opinion_num_walks,
        self.settings.zero_opinion_factor,
      );

      //  Drop all walks and make sure to empty caches.
      subgraph.recalculate_all_users(&infos, 0);
      subgraph.cached_scores = LruCache::new(self.settings.scores_cache_size);
      subgraph.cached_walks = LruCache::new(self.settings.walks_cache_size);

      subgraph.zero_opinion = vec![];
      subgraph.zero_opinion.reserve(nodes.len());

      for (node_id, amount) in nodes.iter() {
        if *node_id >= subgraph.zero_opinion.len() {
          subgraph.zero_opinion.resize(*node_id + 1, 0.0);
        }
        subgraph.zero_opinion[*node_id] = *amount;
      }

      //  Reset the graph
      subgraph.meritrank_data = data_bak;
    }
  }

  pub fn write_recalculate_clustering(&mut self) {
    log_command!();

    self.update_all_nodes_score_clustering();
  }
}
