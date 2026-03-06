use crate::data::*;
use crate::helpers::*;
use crate::node_registry::*;
use crate::utils::log::*;

use meritrank_core::{constants::EPSILON, NodeId, Weight};
use petgraph::graph::{DiGraph, NodeIndex};

use std::collections::HashMap;

use super::AugGraph;

impl AugGraph {
  pub fn validate_read_graph_params_and_setup(
    &self,
    ego_str: &str,
    focus_str: &str,
  ) -> Result<
    (
      NodeId,
      NodeId,
      DiGraph<NodeId, Weight>,
      HashMap<NodeId, NodeIndex>,
      HashMap<NodeIndex, NodeId>,
    ),
    String,
  > {
    let ego_id = match self.nodes.get_by_name(ego_str) {
      Some(x) => x.id,
      None => {
        return Err(format!("Node not found: {:?}", ego_str));
      },
    };

    let focus_id = match self.nodes.get_by_name(focus_str) {
      Some(x) => x.id,
      None => {
        return Err(format!("Node not found: {:?}", focus_str));
      },
    };

    let mut indices = HashMap::<NodeId, NodeIndex>::new();
    let mut ids = HashMap::<NodeIndex, NodeId>::new();
    let mut im_graph = DiGraph::<NodeId, Weight>::new();

    // Add the focus node to the graph as the starting point
    let focus_node_index = im_graph.add_node(focus_id);
    indices.insert(focus_id, focus_node_index);
    ids.insert(focus_node_index, focus_id);

    Ok((ego_id, focus_id, im_graph, indices, ids))
  }

  pub fn edge_weight_normalized(
    &self,
    src: NodeId,
    dst: NodeId,
  ) -> Weight {
    log_trace!("{} {}", src, dst);

    let pos_sum = match self.mr.graph.get_node_data(src) {
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
      .mr
      .graph
      .edge_weight(src, dst)
      .unwrap_or(None)
      .unwrap_or(0.0)
      / pos_sum
  }

  fn add_shortest_path_to_graph(
    &self,
    node_infos: &Vec<NodeInfo>,
    ego_id: NodeId,
    focus_id: NodeId,
    indices: &mut HashMap<NodeId, NodeIndex>,
    ids: &mut HashMap<NodeIndex, NodeId>,
    im_graph: &mut DiGraph<NodeId, Weight>,
  ) -> Vec<(NodeId, NodeId, Weight)> {
    log_trace!();

    let ego_to_focus =
      match perform_astar_search(&self.mr.graph, ego_id, focus_id) {
        Ok(path) => path,
        Err(_) => return vec![],
      };

    let mut edges = Vec::<(NodeId, NodeId, Weight)>::new();
    edges.reserve_exact(ego_to_focus.len().saturating_sub(1));

    log_verbose!("Process shortest path.");

    for k in 0..ego_to_focus.len().saturating_sub(1) {
      let a = ego_to_focus[k];
      let b = ego_to_focus[k + 1];
      let a_info_opt = node_infos.get(a);
      let b_info_opt = node_infos.get(b);
      let a_b_weight = self.edge_weight_normalized(a, b);

      let a_kind_opt = match a_info_opt {
        Some(info) => Some(info.kind),
        None => None,
      };

      let b_kind_opt = match b_info_opt {
        Some(info) => Some(info.kind),
        None => None,
      };

      if k + 2 == ego_to_focus.len() {
        if a_kind_opt == Some(NodeKind::User) {
          edges.push((a, b, a_b_weight));
        } else {
          log_verbose!("Ignore node: {:?}", node_infos[a].name);
        }
      } else if b_kind_opt != Some(NodeKind::User) {
        log_verbose!("Ignore node: {:?}", node_infos[b].name);
        if k + 2 < ego_to_focus.len() {
          let c = ego_to_focus[k + 2];
          let b_c_weight = self.edge_weight_normalized(b, c);
          let a_c_weight = a_b_weight
            * b_c_weight
            * if a_b_weight < 0.0 && b_c_weight < 0.0 {
              -1.0
            } else {
              1.0
            };
          edges.push((a, c, a_c_weight));
        }
      } else if a_kind_opt == Some(NodeKind::User) {
        edges.push((a, b, a_b_weight));
      } else {
        log_verbose!("Ignore node: {:?}", node_infos[a].name);
      }
    }

    log_verbose!("Add path to the graph.");

    for (src, dst, weight) in &edges {
      if let std::collections::hash_map::Entry::Vacant(e) = indices.entry(*src) {
        let index = im_graph.add_node(*src);
        e.insert(index);
        ids.insert(index, *src);
      }
      add_edge_if_valid(im_graph, indices, ids, *src, *dst, *weight);
    }

    edges
  }

  pub fn all_outbound_neighbors_normalized(
    &self,
    node: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{}", node);

    let mut v = vec![];

    match self.mr.graph.get_node_data(node) {
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

  fn add_focus_neighbor_connections(
    &self,
    focus_id: NodeId,
    im_graph: &mut DiGraph<NodeId, Weight>,
    indices: &mut HashMap<NodeId, NodeIndex>,
    ids: &mut HashMap<NodeIndex, NodeId>,
    node_infos: &Vec<NodeInfo>,
    positive_only: bool,
    focus_neighbors: &[(NodeId, Weight)],
  ) {
    log_trace!();

    for (dst_id, focus_dst_weight) in focus_neighbors.iter() {
      let dst_kind_opt = match node_infos.get(*dst_id) {
        Some(x) => Some(x.kind),
        None => None,
      };

      if positive_only && *focus_dst_weight <= 0.0 {
        continue;
      }

      if dst_kind_opt == Some(NodeKind::User) {
        add_edge_if_valid(
          im_graph,
          indices,
          ids,
          focus_id,
          *dst_id,
          *focus_dst_weight,
        );
      } else if dst_kind_opt == Some(NodeKind::Comment)
        || dst_kind_opt == Some(NodeKind::Beacon)
        || dst_kind_opt == Some(NodeKind::Opinion)
      {
        let dst_neighbors = self.all_outbound_neighbors_normalized(*dst_id);
        for (ngh_id, dst_ngh_weight) in dst_neighbors {
          if (positive_only && dst_ngh_weight <= 0.0)
            || ngh_id == focus_id
            || match node_infos.get(ngh_id) {
              Some(x) => Some(x.kind),
              None => None,
            } != Some(NodeKind::User)
          {
            continue;
          }
          let focus_ngh_weight = (*focus_dst_weight)
            * dst_ngh_weight
            * if *focus_dst_weight < 0.0 && dst_ngh_weight < 0.0 {
              -1.0
            } else {
              1.0
            };
          add_edge_if_valid(
            im_graph,
            indices,
            ids,
            focus_id,
            ngh_id,
            focus_ngh_weight,
          );
        }
      }
    }
  }

  fn format_graph_edges(
    &self,
    edges: Vec<(NodeId, NodeId, Weight)>,
    ego_id: NodeId,
  ) -> Vec<GraphResult> {
    edges
      .into_iter()
      .map(|(src_id, dst_id, weight_of_dst)| {
        let (score_value_of_dst, score_cluster_of_dst) =
          self.fetch_score(ego_id, dst_id);
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(dst_id) {
            Some(dst_owner_id) => self.fetch_score_cached(dst_owner_id, ego_id),
            None => (0.0, 0),
          };

        GraphResult {
          src:             self.nodes.id_to_info[src_id].name.clone(),
          dst:             self.nodes.id_to_info[dst_id].name.clone(),
          weight:          weight_of_dst,
          score:           score_value_of_dst,
          reverse_score:   score_value_of_ego,
          cluster:         score_cluster_of_dst,
          reverse_cluster: score_cluster_of_ego,
        }
      })
      .collect()
  }

  fn collect_all_edges(
    &self,
    indices: &HashMap<NodeId, NodeIndex>,
    ids: &HashMap<NodeIndex, NodeId>,
    im_graph: &DiGraph<NodeId, Weight>,
    ego_id: NodeId,
    path_edges: Vec<(NodeId, NodeId, Weight)>,
    index: u32,
    count: u32,
  ) -> Vec<GraphResult> {
    let mut unique_edges =
      extract_unique_edges_from_graph_data(indices, ids, im_graph);

    unique_edges.retain(|&(src, dst, _)| {
      !path_edges
        .iter()
        .any(|&(p_src, p_dst, _)| src == p_src && dst == p_dst)
    });

    unique_edges.sort_by(|(_, _, a), (_, _, b)| b.abs().total_cmp(&a.abs()));

    let path_length = path_edges.len();
    let mut all_edges = path_edges;
    all_edges.extend(unique_edges);

    let paginated_edges = if (index as usize) <= path_length {
      let path_remaining = path_length - (index as usize);
      let mut result: Vec<_> = all_edges
        [index as usize..path_length]
        .to_vec();
      let remaining_count = count.saturating_sub(path_remaining as u32);
      result.extend(
        all_edges[path_length..]
          .iter()
          .take(remaining_count as usize)
          .cloned(),
      );
      result
    } else {
      all_edges[path_length..]
        .iter()
        .skip((index as usize).saturating_sub(path_length))
        .take(count as usize)
        .cloned()
        .collect()
    };

    self.format_graph_edges(paginated_edges, ego_id)
  }

  pub fn read_graph(
    &self,
    data: OpReadGraph,
  ) -> Vec<GraphResult> {
    log_command!("{:?}", data);

    let ego_str = &data.ego;
    let focus_str = &data.focus;
    let positive_only = data.positive_only;
    let index = data.index;
    let count = data.count;

    let (ego_id, focus_id, mut im_graph, mut indices, mut ids) =
      match self.validate_read_graph_params_and_setup(ego_str, focus_str) {
        Ok(data) => data,
        Err(msg) => {
          log_error!("{}", msg);
          return vec![];
        },
      };

    if let Some(ego_info) = self.nodes.get_by_name(ego_str) {
      if !self.ensure_ego_is_user(ego_str, ego_info) {
        return vec![];
      }
    }

    let node_infos = self.nodes.id_to_info.clone();
    let force_read_graph_conn = self.settings.force_read_graph_conn;

    let mut path_edges = if ego_id == focus_id {
      log_verbose!("Ego is same as focus");
      vec![]
    } else {
      self.add_shortest_path_to_graph(
        &node_infos,
        ego_id,
        focus_id,
        &mut indices,
        &mut ids,
        &mut im_graph,
      )
    };

    if force_read_graph_conn && !indices.contains_key(&ego_id) {
      add_edge_if_valid(
        &mut im_graph,
        &mut indices,
        &mut ids,
        ego_id,
        focus_id,
        1.0,
      );
      path_edges.push((ego_id, focus_id, 1.0));
    }

    let mut focus_neighbors = self.all_outbound_neighbors_normalized(focus_id);
    focus_neighbors
      .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    self.add_focus_neighbor_connections(
      focus_id,
      &mut im_graph,
      &mut indices,
      &mut ids,
      &node_infos,
      positive_only,
      &focus_neighbors,
    );

    remove_self_references_from_im_graph(&mut im_graph, &indices);

    self.collect_all_edges(
      &indices,
      &ids,
      &im_graph,
      ego_id,
      path_edges,
      index as u32,
      count as u32,
    )
  }
}
