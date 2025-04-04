//  ================================================
//
//    Commands
//
//  ================================================

use meritrank_core::{constants::EPSILON, Graph, NodeId};
use petgraph::{
  graph::{DiGraph, NodeIndex},
  visit::EdgeRef,
};
use std::collections::hash_map::*;

use crate::astar::*;
use crate::aug_multi_graph::*;
use crate::bloom_filter::*;
use crate::constants::*;
use crate::log::*;
use crate::nodes::*;
use crate::subgraph::Subgraph;

pub fn read_version() -> &'static str {
  log_command!();
  VERSION
}

pub fn write_log_level(log_level: u32) {
  log_command!("{}", log_level);

  ERROR.store(log_level > 0, Ordering::Relaxed);
  WARNING.store(log_level > 1, Ordering::Relaxed);
  INFO.store(log_level > 2, Ordering::Relaxed);
  VERBOSE.store(log_level > 3, Ordering::Relaxed);
  TRACE.store(log_level > 4, Ordering::Relaxed);
}

impl AugMultiGraph {
  pub fn read_node_score(
    &mut self,
    context: &str,
    ego: &str,
    dst: &str,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?} {:?}", context, ego, dst);

    if !self.subgraphs.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    if !self.node_exists(ego) {
      log_error!("Node does not exist: {:?}", ego);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    if !self.node_exists(dst) {
      log_error!("Node does not exist: {:?}", dst);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    let ego_id = self.find_or_add_node_by_name(ego);
    let dst_id = self.find_or_add_node_by_name(dst);

    let (score_of_dst_from_ego, score_cluster_of_dst) =
      self.fetch_score(context, ego_id, dst_id);

    // Handle the case when get_object_owner returns None
    let (score_of_ego_from_dst, score_cluster_of_ego) =
      match self.get_object_owner(context, dst_id) {
        Some(dst_owner_id) => {
          self.fetch_score_cached(context, dst_owner_id, ego_id)
        },
        None => (0.0, 0), // Default values when no owner is found
      };

    [(
      ego.to_string(),
      dst.to_string(),
      score_of_dst_from_ego,
      score_of_ego_from_dst,
      score_cluster_of_dst,
      score_cluster_of_ego,
    )]
    .to_vec()
  }

  pub fn apply_filters_and_pagination(
    &mut self,
    scores: Vec<(NodeId, Weight, Cluster)>,
    context: &str,
    ego: &str,
    ego_id: NodeId,
    kind: NodeKind,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    let mut im: Vec<(NodeId, Weight, Cluster)> = scores
      .into_iter()
      .map(|(n, w, cluster)| {
        (n, node_kind_from_id(&self.node_infos, n), w, cluster)
      })
      .filter(|(_, target_kind, _, _)| {
        kind == NodeKind::Unknown || kind == *target_kind
      })
      .filter(|(_, _, score, _)| {
        score_gt < *score || (score_gte && score_gt <= *score)
      })
      .filter(|(_, _, score, _)| {
        *score < score_lt || (score_lte && score_lt >= *score)
      })
      .collect::<Vec<(NodeId, NodeKind, Weight, Cluster)>>()
      .into_iter()
      .filter(|(target_id, target_kind, _, _)| {
        if !hide_personal
          || (*target_kind != NodeKind::Comment
            && *target_kind != NodeKind::Beacon
            && *target_kind != NodeKind::Opinion)
        {
          return true;
        }
        match self
          .subgraph_from_context(context)
          .meritrank_data
          .graph
          .edge_weight(*target_id, ego_id)
        {
          Ok(Some(_)) => false,
          _ => true,
        }
      })
      .map(|(target_id, _, score, cluster)| (target_id, score, cluster))
      .collect();

    im.sort_by(|(_, a, _), (_, b, _)| b.abs().total_cmp(&a.abs()));

    let index = index as usize;
    let count = count as usize;

    let mut page: Vec<(String, String, Weight, Weight, Cluster, Cluster)> =
      vec![];
    page.reserve_exact(if count < im.len() { count } else { im.len() });

    for i in index..count {
      if i >= im.len() {
        break;
      }

      let score_value_of_dst = im[i].1;
      let score_cluster_of_dst = im[i].2;

      let (score_value_of_ego, score_cluster_of_ego) =
        match self.get_object_owner(context, im[i].0) {
          Some(dst_owner_id) => {
            self.fetch_score_cached(context, dst_owner_id, ego_id)
          },
          None => (0.0, 0), // Default values when no owner is found
        };

      page.push((
        ego.to_string(),
        node_name_from_id(&self.node_infos, im[i].0),
        score_value_of_dst,
        score_value_of_ego,
        score_cluster_of_dst,
        score_cluster_of_ego,
      ));
    }

    page
  }

  pub fn read_scores(
    &mut self,
    context: &str,
    ego: &str,
    kind_str: &str,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!(
      "{:?} {:?} {:?} {} {} {} {} {} {} {}",
      context,
      ego,
      kind_str,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count
    );

    let kind = match kind_from_prefix(kind_str) {
      Ok(x) => x,
      _ => {
        log_error!("Invalid node kind string: {:?}", kind_str);
        return vec![];
      },
    };

    if !self.subgraphs.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);

    let scores = self.fetch_all_scores(context, ego_id);

    return self.apply_filters_and_pagination(
      scores,
      context,
      ego,
      ego_id,
      kind,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
    );
  }

  pub fn read_neighbors(
    &mut self,
    context: &str,
    ego: &str,
    focus: &str,
    direction: i64,
    kind_str: &str,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!(
      "{:?} {} {} {} {:?} {} {} {} {} {} {} {}",
      context,
      ego,
      focus,
      direction,
      kind_str,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count
    );

    let kind = match kind_from_prefix(kind_str) {
      Ok(x) => x,
      _ => {
        log_error!("Invalid node kind string: {:?}", kind_str);
        return vec![];
      },
    };

    let dir = match neighbor_dir_from(direction) {
      Ok(x) => x,
      _ => {
        log_error!("Invalid neighbors direction: {}", direction);
        return vec![];
      },
    };

    let ego_id = self.find_or_add_node_by_name(ego);
    let focus_id = self.find_or_add_node_by_name(focus);

    let scores = self.fetch_neighbors(context, focus_id, dir);

    return self.apply_filters_and_pagination(
      scores,
      context,
      ego,
      ego_id,
      kind,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
    );
  }

  pub fn write_create_context(
    &mut self,
    context: &str,
  ) {
    log_command!("{:?}", context);
    self.subgraph_from_context(context);
  }

  pub fn write_put_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    new_weight: f64,
    magnitude: i64,
  ) {
    log_command!(
      "{:?} {:?} {:?} {} {}",
      context,
      src,
      dst,
      new_weight,
      magnitude
    );

    if magnitude < 0 {
      log_verbose!(
              "Negative magnitude detected: context={}, src={}, dst={}, magnitude={}. Converting to 0.",
              context, src, dst, magnitude
          );
    }

    let mag_clamped = magnitude.max(0) as u32;
    let src_id = self.find_or_add_node_by_name(src);
    let dst_id = self.find_or_add_node_by_name(dst);
    let (
      new_weight_scaled,
      mut new_min_weight,
      new_max_weight,
      new_mag_scale,
      rescale_factor,
    ) = self
      .vsids
      .scale_weight(context, src_id, new_weight, mag_clamped);

    // Check for small edges that need deletion
    let edge_deletion_threshold = new_max_weight * self.vsids.deletion_ratio;
    let can_delete_at_least_one_edge =
      new_min_weight <= edge_deletion_threshold;
    let must_rescale = rescale_factor > 1.0;
    // TODO: handle rewriting existing node case
    if can_delete_at_least_one_edge || must_rescale {
      // This means there is at least one edge to delete,
      // but maybe there is more, so we check everything.
      // In principle, we could have optimized this by storing the edges in a sorted heap structure.
      //new_min_weight = new_max_weight;
      let (edges_to_modify, new_min_weight_from_scan) = self
        .subgraph_from_context(context)
        .meritrank_data
        .graph
        .get_node_data(src_id)
        .unwrap()
        .get_outgoing_edges()
        .fold(
          (Vec::new(), new_min_weight),
          |(mut to_modify, min), (dest, weight)| {
            let abs_weight = if must_rescale {
              weight.abs() / rescale_factor
            } else {
              weight.abs()
            };

            if abs_weight <= edge_deletion_threshold {
              to_modify.push((dest, 0.0));
              (to_modify, min)
            } else {
              if must_rescale {
                to_modify.push((dest, weight / rescale_factor));
              }
              (to_modify, min.min(abs_weight))
            }
          },
        );
      new_min_weight = new_min_weight_from_scan;

      for (dst_id, weight) in edges_to_modify {
        log_verbose!(
          "Rescale or delete node: context={:?}, src={}, dst={}, new_weight={}",
          context,
          node_name_from_id(&self.node_infos, src_id),
          node_name_from_id(&self.node_infos, dst_id),
          weight
        );
        self.set_edge(context, src_id, dst_id, weight);
      }
    }
    self.set_edge(context, src_id, dst_id, new_weight_scaled);
    if must_rescale {
      log_verbose!(
          "Rescale performed: context={:?}, src={}, dst={}, normalized_new_weight={}",
          context,src,dst, new_weight_scaled);
    } else {
      log_verbose!(
          "Edge updated without rescale: context={:?}, src={}, dst={}, new_weight_scaled={}",
          context,src,dst,new_weight_scaled);
    }
    self.vsids.min_max_weights.insert(
      (context.to_string(), src_id),
      (new_min_weight, new_max_weight, new_mag_scale),
    );
  }

  pub fn write_delete_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    _index: i64,
  ) {
    log_command!("{:?} {:?} {:?}", context, src, dst);

    if !self.node_exists(src) || !self.node_exists(dst) {
      return;
    }

    let src_id = self.find_or_add_node_by_name(src);
    let dst_id = self.find_or_add_node_by_name(dst);

    self.set_edge(context, src_id, dst_id, 0.0);
  }

  pub fn write_delete_node(
    &mut self,
    context: &str,
    node: &str,
    _index: i64,
  ) {
    log_command!("{:?} {:?}", context, node);

    if !self.node_exists(node) {
      return;
    }

    let id = self.find_or_add_node_by_name(node);

    // Collect the outgoing edges first
    let outgoing_edges: Vec<NodeId> = self
      .subgraph_from_context(context)
      .meritrank_data
      .graph
      .get_node_data(id)
      .map(|data| {
        data
          .get_outgoing_edges()
          .into_iter()
          .map(|(n, _)| n)
          .collect()
      })
      .unwrap();

    // Then remove the edges
    for n in outgoing_edges {
      self.set_edge(context, id, n, 0.0);
    }
  }

  pub fn read_graph(
    &mut self,
    context: &str,
    ego: &str,
    focus: &str,
    positive_only: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    log_command!(
      "{:?} {:?} {:?} {} {} {}",
      context,
      ego,
      focus,
      positive_only,
      index,
      count
    );

    // Validate input parameters
    // Check if the context exists in the subgraphs
    if !self.subgraphs.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    // Check if the ego node exists
    if !self.node_exists(ego) {
      log_error!("Node does not exist: {:?}", ego);
      return vec![];
    }

    // Check if the focus node exists
    if !self.node_exists(focus) {
      log_error!("Node does not exist: {:?}", focus);
      return vec![];
    }

    // Get node IDs for ego and focus
    let ego_id = self.find_or_add_node_by_name(ego);
    let focus_id = self.find_or_add_node_by_name(focus);

    let force_read_graph_conn = self.settings.force_read_graph_conn;

    // Initialize data structures for building the graph
    // HashMap to map between NodeId and NodeIndex in the petgraph
    let mut indices = HashMap::<NodeId, NodeIndex>::new();
    let mut ids = HashMap::<NodeIndex, NodeId>::new();
    // Create a directed graph to represent the relationships
    let mut im_graph = DiGraph::<NodeId, Weight>::new();

    // Add the focus node to the graph as the starting point
    {
      let index = im_graph.add_node(focus_id);
      indices.insert(focus_id, index);
      ids.insert(index, focus_id);
    }

    // Clone node information for use in the function
    let node_infos = self.node_infos.clone();

    // Get the subgraph for the specified context
    let subgraph = self.get_subgraph_from_context(context);

    log_verbose!("Enumerate focus neighbors");

    // Get all normalized outbound neighbors of the focus node
    // This gives us all nodes directly connected to the focus node
    let focus_neighbors = subgraph.all_outbound_neighbors_normalized(focus_id);

    // Handle the case where ego and focus are the same node
    if ego_id == focus_id {
      log_verbose!("Ego is same as focus");
    } else {
      // Find the shortest path from ego to focus and add it to the graph
      add_shortest_path_to_graph(
        &subgraph,
        &node_infos,
        ego_id,
        focus_id,
        &mut indices,
        &mut ids,
        &mut im_graph,
      );
    }
    if force_read_graph_conn && !indices.contains_key(&ego_id) {
      add_edge_if_valid(
        &mut im_graph,
        &mut indices,
        &mut ids,
        ego_id,
        focus_id,
        1.0);
    }

    // Process each neighbor of the focus node
    for (dst_id, focus_dst_weight) in focus_neighbors {
      let dst_kind = node_kind_from_id(&node_infos, dst_id);
      // Skip if positive_only is true and the is not positive
      if positive_only && focus_dst_weight <= 0.0 {
        continue;
      }

      // If the neighbor is a User node, add it directly to the graph
      if dst_kind == NodeKind::User {
        // Inside the loop where the edge is being added
        add_edge_if_valid(
          &mut im_graph,
          &mut indices,
          &mut ids,
          focus_id,
          dst_id,
          focus_dst_weight,
        );
      }
      // If the neighbor is a Comment, Beacon, or Opinion node, process its neighbors
      // This handles indirect connections through non-user nodes
      else if dst_kind == NodeKind::Comment
        || dst_kind == NodeKind::Beacon
        || dst_kind == NodeKind::Opinion
      {
        // Get all neighbors of this non-user node
        let dst_neighbors = subgraph.all_outbound_neighbors_normalized(dst_id);

        // Process each neighbor of the non-user node
        for (ngh_id, dst_ngh_weight) in dst_neighbors {
          // Skip if conditions are not met
          if (positive_only && dst_ngh_weight <= 0.0)
            || ngh_id == focus_id
            || node_kind_from_id(&node_infos, ngh_id) != NodeKind::User
          {
            continue;
          }

          // Calculate the weight of the edge from focus to this neighbor
          // This represents the indirect connection through the non-user node
          let focus_ngh_weight = focus_dst_weight
            * dst_ngh_weight
            * if focus_dst_weight < 0.0 && dst_ngh_weight < 0.0 {
              -1.0
            } else {
              1.0
            };

          // Add an edge from focus to this neighbor
          add_edge_if_valid(
            &mut im_graph,
            &mut indices,
            &mut ids,
            focus_id,
            ngh_id,
            focus_ngh_weight,
          );
        }
      }
    }

    log_verbose!("Remove self references");

    // Remove self-loops (edges from a node to itself)
    for (_, src_index) in indices.iter() {
      let neighbors: Vec<_> = im_graph
        .edges(*src_index)
        .map(|edge| (edge.target(), edge.id()))
        .collect();

      for (dst_index, edge_id) in neighbors {
        if *src_index == dst_index {
          im_graph.remove_edge(edge_id);
        }
      }
    }
    self.collect_all_edges(
      &indices, &ids, &im_graph, context, ego_id, index, count,
    )
  }

  pub fn collect_all_edges(
    &mut self,
    indices: &HashMap<NodeId, NodeIndex>,
    ids: &HashMap<NodeIndex, NodeId>,
    im_graph: &DiGraph<NodeId, Weight>,
    context: &str,
    ego_id: NodeId,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    // Collect all edges from the graph
    let mut edge_ids = Vec::<(NodeId, NodeId, Weight)>::new();
    edge_ids.reserve_exact(indices.len() * 2); // ad hok

    log_verbose!("Build final array");

    // Extract all edges from the graph
    for (_, src_index) in indices {
      for edge in im_graph.edges(*src_index) {
        if let (Some(src_id), Some(dst_id)) =
          (ids.get(src_index), ids.get(&edge.target()))
        {
          let w = *edge.weight();
          // Skip edges with zero weight
          if w > -EPSILON && w < EPSILON {
            log_error!(
              "Got zero edge weight: {} -> {}",
              node_name_from_id(&self.node_infos, *src_id),
              node_name_from_id(&self.node_infos, *dst_id)
            );
          } else {
            // Check for duplicate edges
            let mut found = false;
            for (x, y, _) in edge_ids.iter() {
              if *src_id == *x && *dst_id == *y {
                found = true;
                break;
              }
            }
            if !found {
              edge_ids.push((*src_id, *dst_id, w));
            }
          }
        } else {
          log_error!("Got invalid node index");
        }
      }
    }

    edge_ids.sort_by(|(_, _, a), (_, _, b)| b.abs().total_cmp(&a.abs()));

    edge_ids
      .into_iter()
      .skip(index as usize)
      .take(count as usize)
      .map(|(src_id, dst_id, weight_of_dst)| {
        let (score_value_of_dst, score_cluster_of_dst) =
          self.fetch_score(context, ego_id, dst_id);
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(context, dst_id) {
            Some(dst_owner_id) => {
              self.fetch_score_cached(context, dst_owner_id, ego_id)
            },
            None => (0.0, 0), // Default values when no owner is found
          };

        (
          node_name_from_id(&self.node_infos, src_id),
          node_name_from_id(&self.node_infos, dst_id),
          weight_of_dst,
          score_value_of_dst,
          score_value_of_ego,
          score_cluster_of_dst,
          score_cluster_of_ego,
        )
      })
      .collect()
  }

  pub fn read_connected(
    &mut self,
    context: &str,
    ego: &str,
  ) -> Vec<(String, String)> {
    log_command!("{:?} {:?}", context, ego);

    if !self.subgraphs.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    if !self.node_exists(ego) {
      log_error!("Node does not exist: {:?}", ego);
      return vec![];
    }

    let src_id = self.find_or_add_node_by_name(ego);

    let outgoing_edges: Vec<_> = self
      .subgraph_from_context(context)
      .meritrank_data
      .graph
      .get_node_data(src_id)
      .unwrap()
      .get_outgoing_edges()
      .collect();

    outgoing_edges
      .into_iter()
      .map(|(dst_id, _)| {
        (ego.to_string(), node_name_from_id(&self.node_infos, dst_id))
      })
      .collect()
  }

  pub fn read_node_list(&self) -> Vec<(String,)> {
    log_command!();

    self
      .node_infos
      .iter()
      .map(|info| (info.name.clone(),))
      .collect()
  }

  pub fn read_edges(
    &mut self,
    context: &str,
  ) -> Vec<(String, String, Weight)> {
    log_command!("{:?}", context);

    if !self.subgraphs.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    let infos = self.node_infos.clone();

    let mut v: Vec<(String, String, Weight)> = vec![];
    v.reserve(infos.len() * 2); // ad hok

    for src_id in 0..infos.len() {
      let src_name = infos[src_id].name.as_str();

      match self
        .subgraph_from_context(context)
        .meritrank_data
        .graph
        .get_node_data(src_id)
      {
        Some(data) => {
          for (dst_id, weight) in data.get_outgoing_edges() {
            match infos.get(dst_id) {
              Some(x) => v.push((src_name.to_string(), x.name.clone(), weight)),
              None => log_error!("Node does not exist: {}", dst_id),
            }
          }
        },
        _ => {},
      };
    }

    v
  }

  pub fn read_mutual_scores(
    &mut self,
    context: &str,
    ego: &str,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?}", context, ego);

    if !self.subgraphs.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);
    let ranks = self.fetch_all_scores(context, ego_id);
    let mut v =
      Vec::<(String, String, Weight, Weight, Cluster, Cluster)>::new();

    v.reserve_exact(ranks.len());

    for (node, score_value_of_dst, score_cluster_of_dst) in ranks {
      let info = match self.node_infos.get(node) {
        Some(x) => x.clone(),
        None => NodeInfo {
          kind:       NodeKind::Unknown,
          name:       "".to_string(),
          seen_nodes: Vec::new(),
        },
      };
      if score_value_of_dst > 0.0 && info.kind == NodeKind::User {
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(context, node) {
            Some(dst_owner_id) => {
              self.fetch_score_cached(context, dst_owner_id, ego_id)
            },
            None => (0.0, 0), // Default values when no owner is found
          };

        v.push((
          ego.to_string(),
          info.name,
          score_value_of_dst,
          score_value_of_ego,
          score_cluster_of_dst,
          score_cluster_of_ego,
        ));
      }
    }

    v
  }

  pub fn write_reset(&mut self) {
    log_command!();

    self.reset();
  }

  pub fn read_new_edges_filter(
    &mut self,
    src: &str,
  ) -> Vec<u8> {
    log_command!("{:?}", src);

    if !self.node_exists(src) {
      log_error!("Node does not exist: {:?}", src);
      return vec![];
    }

    let src_id = self.find_or_add_node_by_name(src);

    let mut v: Vec<u8> = vec![];
    v.reserve_exact(self.node_infos[src_id].seen_nodes.len() * 8);

    for &x in &self.node_infos[src_id].seen_nodes {
      for i in 0..8 {
        v.push((x & (0xff << (8 * i)) >> (8 * i)) as u8);
      }
    }

    return v;
  }

  pub fn write_new_edges_filter(
    &mut self,
    src: &str,
    filter_bytes: &[u8],
  ) {
    log_command!("{:?} {:?}", src, filter_bytes);

    let src_id = self.find_or_add_node_by_name(src);

    let mut v: Vec<u64> = vec![];
    v.resize(((filter_bytes.len() + 7) / 8) * 8, 0);

    for i in 0..filter_bytes.len() {
      v[i / 8] = (filter_bytes[i] as u64) << (8 * (i % 8));
    }

    self.node_infos[src_id].seen_nodes = v;
  }

  pub fn write_fetch_new_edges(
    &mut self,
    src: &str,
    prefix: &str,
  ) -> Vec<(String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?}", src, prefix);

    let num_hashes = self.settings.filter_num_hashes;
    let max_size = self.settings.filter_max_size / 8;

    let src_id = self.find_or_add_node_by_name(src);

    if self.node_infos[src_id].seen_nodes.is_empty() {
      self.node_infos[src_id]
        .seen_nodes
        .resize((self.settings.filter_min_size + 7) / 8, 0);

      log_verbose!(
        "Create the bloom filter with {} bytes for {:?}",
        8 * self.node_infos[src_id].seen_nodes.len(),
        src
      );
    }

    //  Fetch new edges
    //

    let mut v: Vec<(String, Weight, Weight, Cluster, Cluster)> = vec![];

    for dst_id in 0..self.node_count {
      //  FIXME Probably we should use NodeKind here.
      if !self.node_infos[dst_id].name.starts_with(prefix) {
        continue;
      }

      let (score_value_of_dst, score_cluster_of_dst) =
        self.fetch_score("", src_id, dst_id);
      let (score_value_of_src, score_cluster_of_src) =
        self.fetch_score_cached("", src_id, dst_id);

      if score_value_of_dst < EPSILON {
        continue;
      }

      let bits = bloom_filter_bits(
        self.node_infos[src_id].seen_nodes.len(),
        num_hashes,
        dst_id,
      );

      if !bloom_filter_contains(&self.node_infos[src_id].seen_nodes, &bits) {
        v.push((
          self.node_infos[dst_id].name.clone(),
          score_value_of_dst,
          score_value_of_src,
          score_cluster_of_dst,
          score_cluster_of_src,
        ));
      }
    }

    //  Rebuild the bloom filter
    //

    let mut seen_nodes = vec![];

    seen_nodes.resize(
      std::cmp::min(self.node_infos[src_id].seen_nodes.len(), max_size),
      0,
    );

    loop {
      let mut saturated = false;

      for x in seen_nodes.iter_mut() {
        *x = 0;
      }

      for dst_id in 0..self.node_count {
        let bits = bloom_filter_bits(seen_nodes.len(), num_hashes, dst_id);
        let collision = bloom_filter_contains(&mut seen_nodes, &bits);

        if collision && seen_nodes.len() < max_size {
          //  Resize the bloom filter if it is saturated

          let n = seen_nodes.len() * 2;
          seen_nodes.resize(n, 0);

          log_verbose!(
            "Resize the bloom filter to {} bytes for {:?}",
            8 * n,
            src
          );

          saturated = true;
          break;
        }

        //  FIXME Probably we should use NodeKind here.
        if self.node_infos[dst_id].name.starts_with(prefix) {
          let num_walks = self.settings.num_walks;
          let k = self.settings.zero_opinion_factor;

          let score = self
            .subgraph_from_context("")
            .fetch_raw_score(src_id, dst_id, num_walks, k);

          if !(score < EPSILON) {
            bloom_filter_add(&mut seen_nodes, &bits);
          }
        } else {
          //  RUST!!!
          let len = self.node_infos[src_id].seen_nodes.len();

          let already_seen = bloom_filter_contains(
            &mut self.node_infos[src_id].seen_nodes,
            &bloom_filter_bits(len, num_hashes, dst_id),
          );

          if already_seen {
            bloom_filter_add(&mut seen_nodes, &bits);
          }
        }
      }

      if !saturated {
        if seen_nodes.len() >= max_size {
          log_warning!("Max bloom filer size is reached for {:?}", src);
        }

        self.node_infos[src_id].seen_nodes = seen_nodes;
        break;
      }
    }

    //  Return fetched edges
    //

    return v;
  }

  pub fn write_set_zero_opinion(
    &mut self,
    context: &str,
    node: &str,
    score: Weight,
  ) {
    log_command!("{:?} {} {}", context, node, score);

    let id = self.find_or_add_node_by_name(node);

    let zero_opinion = &mut self.subgraph_from_context(context).zero_opinion;

    if id >= zero_opinion.len() {
      zero_opinion.resize(id + 1, 0.0);
    }

    zero_opinion[id] = score;
  }
}
// Define a custom error enum for A* search
#[derive(Debug, Clone, PartialEq)]
pub enum AStarError {
  PathDoesNotExist(NodeId, NodeId),
  SearchExhausted(NodeId, NodeId),
  Other(String),
}

impl std::fmt::Display for AStarError {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    match self {
      AStarError::PathDoesNotExist(from, to) => {
        write!(f, "Path does not exist from {} to {}", from, to)
      },
      AStarError::SearchExhausted(from, to) => {
        write!(f, "Unable to find a path from {} to {}", from, to)
      },
      AStarError::Other(msg) => write!(f, "{}", msg),
    }
  }
}

impl std::error::Error for AStarError {}

fn perform_astar_search(
  graph: &Graph,
  ego_id: NodeId,
  focus_id: NodeId,
) -> Result<Vec<NodeId>, AStarError> {
  //  ================================
  //
  //    A* search
  //

  let mut open: Vec<Node<NodeId, Weight>> = vec![];
  let mut closed: Vec<Node<NodeId, Weight>> = vec![];

  open.resize(1024, Node::default());
  closed.resize(1024, Node::default());

  let mut astar_state = init(&mut open, ego_id, focus_id, 0.0);

  let mut steps = 0;
  let mut neighbor = None;
  let mut status = Status::PROGRESS;

  //  Do 10000 iterations max

  for _ in 0..10000 {
    steps += 1;

    status =
      iteration(&mut open, &mut closed, &mut astar_state, neighbor.clone());

    match status.clone() {
      Status::NEIGHBOR(request) => match graph.get_node_data(request.node) {
        None => neighbor = None,
        Some(data) => {
          let kv: Vec<_> =
            data.pos_edges.iter().skip(request.index).take(1).collect();

          if kv.is_empty() {
            neighbor = None;
          } else {
            let n = kv[0].0;
            let mut w = *kv[0].1;

            if data.pos_sum > EPSILON {
              w /= data.pos_sum;
            }

            neighbor = Some(Link::<NodeId, Weight> {
              neighbor:       *n,
              exact_distance: if w.abs() < EPSILON {
                1_000_000.0
              } else {
                1.0 / w
              },
              estimate:       0.0,
            });
          }
        },
      },
      Status::OUT_OF_MEMORY => {
        open.resize(open.len() * 2, Node::default());
        closed.resize(closed.len() * 2, Node::default());
      },
      Status::SUCCESS => break,
      Status::FAIL => break,
      Status::PROGRESS => {},
    };
  }

  log_verbose!("Did {} A* iterations", steps);

  if status == Status::SUCCESS {
    log_verbose!("Path found");

    let mut ego_to_focus: Vec<NodeId> = vec![];
    ego_to_focus.resize(astar_state.num_closed, 0);
    let n = path(&closed, &astar_state, &mut ego_to_focus);
    ego_to_focus.resize(n, 0);

    //for node in ego_to_focus.iter() {
    //  log_verbose!("Path: {}", node_name_from_id(&self.node_infos, *node));
    //}

    Ok(ego_to_focus)
  } else if status == Status::FAIL {
    Err(AStarError::PathDoesNotExist(ego_id, focus_id))
  } else {
    Err(AStarError::SearchExhausted(ego_id, focus_id))
  }
}

// Helper method to find the shortest path from ego to focus and add it to the graph
fn add_shortest_path_to_graph(
  subgraph: &Subgraph,
  node_infos: &Vec<NodeInfo>,
  ego_id: NodeId,
  focus_id: NodeId,
  indices: &mut HashMap<NodeId, NodeIndex>,
  ids: &mut HashMap<NodeIndex, NodeId>,
  im_graph: &mut DiGraph<NodeId, Weight>,
) {
  // Find the shortest path from ego to focus using A* search
  log_verbose!("Search shortest path");

  // Perform A* search to find the path from ego to focus
  // This helps establish a connection between the ego and focus nodes
  let ego_to_focus = match perform_astar_search(
    &subgraph.meritrank_data.graph,
    ego_id,
    focus_id,
  ) {
    Ok(path) => path,
    Err(AStarError::PathDoesNotExist(from, to)) => {
      log_verbose!("Path does not exist from {} to {}", from, to);
      return;
    },
    Err(error) => {
      log_error!("{}", error);
      return;
    },
  };

  // Process the path found by A* search
  let mut edges = Vec::<(NodeId, NodeId, Weight)>::new();
  edges.reserve_exact(ego_to_focus.len() - 1);

  log_verbose!("Process shortest path");

  // Process each edge in the path
  for k in 0..ego_to_focus.len() - 1 {
    let a = ego_to_focus[k];
    let b = ego_to_focus[k + 1];

    let a_kind = node_kind_from_id(node_infos, a);
    let b_kind = node_kind_from_id(node_infos, b);

    let a_b_weight = subgraph.edge_weight_normalized(a, b);

    // Handle different cases based on node types and position in the path
    // This logic determines which edges to include in the final graph
    if k + 2 == ego_to_focus.len() {
      // Last edge in the path
      if a_kind == NodeKind::User {
        edges.push((a, b, a_b_weight));
      } else {
        log_verbose!("Ignore node {}", node_name_from_id(node_infos, a));
      }
    } else if b_kind != NodeKind::User {
      // Skip non-user nodes in the middle of the path
      // Create a direct edge from a to c (skipping b)
      log_verbose!("Ignore node {}", node_name_from_id(node_infos, b));
      let c = ego_to_focus[k + 2];
      let b_c_weight = subgraph.edge_weight_normalized(b, c);
      let a_c_weight = a_b_weight
        * b_c_weight
        * if a_b_weight < 0.0 && b_c_weight < 0.0 {
          -1.0
        } else {
          1.0
        };
      edges.push((a, c, a_c_weight));
    } else if a_kind == NodeKind::User {
      // Include edges between user nodes
      edges.push((a, b, a_b_weight));
    } else {
      log_verbose!("Ignore node {}", node_name_from_id(node_infos, a));
    }
  }

  log_verbose!("Add path to the graph");

  // Add all edges from the path to the graph
  for (src, dst, weight) in edges {
    // Add nodes if they don't exist yet
    if !indices.contains_key(&src) {
      let index = im_graph.add_node(src);
      indices.insert(src, index);
      ids.insert(index, src);
    }

    // Add the edge to the graph
    add_edge_if_valid(im_graph, indices, ids, src, dst, weight);
  }
}
fn add_edge_if_valid(
  im_graph: &mut DiGraph<NodeId, Weight>,
  indices: &mut HashMap<NodeId, NodeIndex>,
  ids: &mut HashMap<NodeIndex, NodeId>,
  src_id: NodeId,
  dst_id: NodeId,
  focus_dst_weight: Weight,
) {
  // Add the node to the graph if it doesn't exist yet
  if !indices.contains_key(&src_id) {
    let index = im_graph.add_node(src_id);
    indices.insert(src_id, index);
    ids.insert(index, src_id);
  }
  if !indices.contains_key(&dst_id) {
    let index = im_graph.add_node(dst_id);
    indices.insert(dst_id, index);
    ids.insert(index, dst_id);
  }
  if let (Some(focus_idx), Some(dst_idx)) =
    (indices.get(&src_id), indices.get(&dst_id))
  {
    im_graph.add_edge(*focus_idx, *dst_idx, focus_dst_weight);
  } else {
    log_error!("Got invalid node id");
  }
}
