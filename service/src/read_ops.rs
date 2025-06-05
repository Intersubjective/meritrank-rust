// service/src/read_ops.rs

use meritrank_core::{constants::EPSILON, NodeId};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

use crate::aug_multi_graph::{AugMultiGraph, Cluster}; // Weight removed
use crate::constants::VERSION;
use crate::log::*;
use crate::nodes::*;
use crate::protocol::{neighbor_dir_from, NEIGHBORS_INBOUND}; // Assuming neighbor_dir_from is pub in protocol
use crate::subgraph::Subgraph;
use meritrank_core::Weight; // Weight added directly
                            // use crate::bloom_filter::*; // Removed unused import
use crate::astar::*; // For A*
use meritrank_core::Graph; // For A*
use petgraph::visit::EdgeRef; // Added for edge.target() and edge.id()

// Standalone functions first
pub fn read_version() -> &'static str {
  log_command!();
  VERSION
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

    let (score_of_ego_from_dst, score_cluster_of_ego) =
      match self.get_object_owner(context, dst_id) {
        Some(dst_owner_id) => {
          self.fetch_score_cached(context, dst_owner_id, ego_id)
        },
        None => (0.0, 0),
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

  fn _filter_and_sort_scores(
    &mut self, // Needs &mut self because of subgraph_from_context
    scores: Vec<(NodeId, Weight, Cluster)>,
    context: &str,
    ego_id: NodeId,
    kind_filter: Option<NodeKind>, // Changed parameter name and type
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
  ) -> Vec<(NodeId, Weight, Cluster)> {
    let mut im: Vec<(NodeId, Weight, Cluster)> = scores
      .into_iter()
      .map(|(n, w, cluster)| {
        (n, node_kind_from_id(&self.node_infos, n), w, cluster) // node_kind_from_id now returns Option<NodeKind>
      })
      .filter(|(_id, opt_node_kind, _w, _c)| { // Filter by kind_filter
        match kind_filter {
          None => true,
          Some(filter_k) => *opt_node_kind == Some(filter_k),
        }
      })
      .filter_map(|(id, opt_node_kind, w, c)| { // Convert Option<NodeKind> to NodeKind, filtering out Nones
        opt_node_kind.map(|concrete_kind| (id, concrete_kind, w, c))
      })
      .filter(|(_, _, score, _)| {
        score_gt < *score || (score_gte && score_gt <= *score)
      })
      .filter(|(_, _, score, _)| {
        *score < score_lt || (score_lte && score_lt >= *score)
      })
      .collect::<Vec<(NodeId, NodeKind, Weight, Cluster)>>() // This collect now receives concrete NodeKind
      .into_iter()
      .filter(|(target_id, current_node_kind, _, _)| { // current_node_kind is NodeKind here
        if !hide_personal
          || (*current_node_kind != NodeKind::Comment
            && *current_node_kind != NodeKind::Beacon
            && *current_node_kind != NodeKind::Opinion)
        {
          return true;
        }
        match self
          .subgraph_from_context(context) // This requires &mut self for AugMultiGraph
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
    im
  }

  fn _prioritize_ego_owned_items(
    &mut self, // Changed to &mut self because get_object_owner needs &mut self
    items: &mut Vec<(NodeId, Weight, Cluster)>,
    context: &str,
    ego_id: NodeId,
  ) {
    let mut insert_index = 0;
    for i in 0..items.len() {
      if let Some(owner) = self.get_object_owner(context, items[i].0) {
        if owner == ego_id {
          items.swap(i, insert_index);
          insert_index += 1;
        }
      }
    }
  }

  fn _paginate_and_format_items(
    &mut self, // Needs &mut self for fetch_score_cached
    items: Vec<(NodeId, Weight, Cluster)>,
    ego: &str,
    ego_id: NodeId,
    context: &str,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    let index = index as usize;
    let count = count as usize;

    let mut page: Vec<(String, String, Weight, Weight, Cluster, Cluster)> =
      vec![];
    // Ensure correct capacity: min of remaining items and requested count
    let remaining_items = items.len().saturating_sub(index);
    let page_capacity = std::cmp::min(remaining_items, count);
    page.reserve_exact(page_capacity);

    // Iterate only over the items that will be part of the current page
    for i in index..std::cmp::min(index + count, items.len()) {
      // This check is actually redundant due to loop bounds, but kept for safety / explicitness
      // if i >= items.len() {
      //   break;
      // }

      let score_value_of_dst = items[i].1;
      let score_cluster_of_dst = items[i].2;

      let (score_value_of_ego, score_cluster_of_ego) =
        match self.get_object_owner(context, items[i].0) {
          Some(dst_owner_id) => {
            self.fetch_score_cached(context, dst_owner_id, ego_id) // Requires &mut self for AugMultiGraph
          },
          None => (0.0, 0),
        };

      page.push((
        ego.to_string(),
        node_name_from_id(&self.node_infos, items[i].0),
        score_value_of_dst,
        score_value_of_ego,
        score_cluster_of_dst,
        score_cluster_of_ego,
      ));
    }
    page
  }

  fn apply_filters_and_pagination(
    &mut self,
    scores: Vec<(NodeId, Weight, Cluster)>,
    context: &str,
    ego: &str,
    ego_id: NodeId,
    kind_filter: Option<NodeKind>, // Changed parameter
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
    prioritize_ego_owned_nodes: bool,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    let mut filtered_sorted_scores = self._filter_and_sort_scores(
      scores,
      context,
      ego_id,
      kind_filter, // Pass Option<NodeKind>
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
    );

    if prioritize_ego_owned_nodes {
      self._prioritize_ego_owned_items(
        &mut filtered_sorted_scores,
        context,
        ego_id,
      );
    }

    self._paginate_and_format_items(
      filtered_sorted_scores,
      ego,
      ego_id,
      context,
      index,
      count,
    )
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

    let kind_opt = node_kind_from_prefix(kind_str); // Use new function from nodes.rs

    if !self.subgraphs.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);
    let scores = self.fetch_all_scores(context, ego_id);

    self.apply_filters_and_pagination(
      scores,
      context,
      ego,
      ego_id,
      kind_opt, // Pass Option<NodeKind>
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
      false,
    )
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

    let kind_opt = node_kind_from_prefix(kind_str); // Use new function from nodes.rs

    let dir = match neighbor_dir_from(direction) {
      Ok(x) => x,
      _ => {
        log_error!("Invalid neighbors direction: {}", direction);
        return vec![];
      },
    };

    let ego_id = self.find_or_add_node_by_name(ego);
    let focus_id = self.find_or_add_node_by_name(focus);

    // Handling the special case - dirty hack - of returning
    // poll results through the neighbors method.
    if kind == NodeKind::PollVariant
      && kind_from_name(ego) == NodeKind::User
      && kind_from_name(focus) == NodeKind::Poll
      && direction == NEIGHBORS_INBOUND
    {
      log_info!("Returning poll results through read_neighbors - ego: {}, focus: {}", ego, focus);
      return if let Some(poll_result) = self
        .get_subgraph_from_context(context)
        .poll_store
        .get_poll_results(ego_id, focus_id)
      {
        poll_result
          .iter()
          .map(|(opt, w)| {
            (
              focus.to_string(),
              node_name_from_id(&self.node_infos, *opt),
              *w,
              0.0,
              0,
              0,
            )
          })
          .collect()
      } else {
        log_warning!("No poll result found for ego: {}, focus: {}", ego, focus);
        vec![]
      }
    }

    let mut scores = self.fetch_neighbors(context, ego_id, focus_id, dir);

    if kind_opt == Some(NodeKind::Opinion) && direction == NEIGHBORS_INBOUND { // Use kind_opt
      scores.retain(|&(node_id, _, _)| {
        self.get_object_owner(context, node_id) != Some(focus_id)
      });
    }

    self.apply_filters_and_pagination(
      scores,
      context,
      ego,
      ego_id,
      kind_opt, // Pass Option<NodeKind>
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
      true,
    )
  }

  // Helper for read_graph: Validates parameters and sets up initial graph structures.
  fn _validate_read_graph_params_and_setup(
    &mut self, // find_or_add_node_by_name is &mut self
    context_str: &str,
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
    if !self.subgraphs.contains_key(context_str) {
      return Err(format!("Context does not exist: {:?}", context_str));
    }
    if !self.node_exists(ego_str) {
      return Err(format!("Node does not exist: {:?}", ego_str));
    }
    if !self.node_exists(focus_str) {
      return Err(format!("Node does not exist: {:?}", focus_str));
    }

    let ego_id = self.find_or_add_node_by_name(ego_str);
    let focus_id = self.find_or_add_node_by_name(focus_str);

    let mut indices = HashMap::<NodeId, NodeIndex>::new();
    let mut ids = HashMap::<NodeIndex, NodeId>::new();
    let mut im_graph = DiGraph::<NodeId, Weight>::new();

    // Add the focus node to the graph as the starting point
    let focus_node_index = im_graph.add_node(focus_id);
    indices.insert(focus_id, focus_node_index);
    ids.insert(focus_node_index, focus_id);

    Ok((ego_id, focus_id, im_graph, indices, ids))
  }

  // Helper for read_graph: Adds shortest path from ego to focus and handles force_read_graph_conn.
  fn _add_shortest_path_and_forced_connections(
    // Removed &self
    force_read_graph_conn: bool, // Added parameter
    ego_id: NodeId,
    focus_id: NodeId,
    im_graph: &mut DiGraph<NodeId, Weight>,
    indices: &mut HashMap<NodeId, NodeIndex>,
    ids: &mut HashMap<NodeIndex, NodeId>,
    subgraph: &Subgraph,
    node_infos: &Vec<NodeInfo>,
  ) {
    if ego_id == focus_id {
      log_verbose!("Ego is same as focus");
    } else {
      // Call existing private helper
      add_shortest_path_to_graph(
        subgraph, node_infos, ego_id, focus_id, indices, ids, im_graph,
      );
    }

    if force_read_graph_conn && !indices.contains_key(&ego_id) {
      // Use parameter
      // Call existing private helper
      add_edge_if_valid(im_graph, indices, ids, ego_id, focus_id, 1.0);
    }
  }

  // Helper for read_graph: Processes focus neighbors to populate the graph.
  fn _add_focus_neighbor_connections(
    // Removed &self
    focus_id: NodeId,
    im_graph: &mut DiGraph<NodeId, Weight>,
    indices: &mut HashMap<NodeId, NodeIndex>,
    ids: &mut HashMap<NodeIndex, NodeId>,
    subgraph: &Subgraph,
    node_infos: &Vec<NodeInfo>, // Passed in
    positive_only: bool,
    focus_neighbors: &[(NodeId, Weight)], // Passed as slice
  ) {
    log_verbose!("Enumerate focus neighbors");
    for (dst_id, focus_dst_weight) in focus_neighbors.iter() {
      let dst_kind_opt = node_kind_from_id(node_infos, *dst_id); // Returns Option<NodeKind>
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
        let dst_neighbors = subgraph.all_outbound_neighbors_normalized(*dst_id);
        for (ngh_id, dst_ngh_weight) in dst_neighbors {
          if (positive_only && dst_ngh_weight <= 0.0)
            || ngh_id == focus_id
            || node_kind_from_id(node_infos, ngh_id) != Some(NodeKind::User)
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

  // Helper for read_graph: Removes self-references from the graph.
  fn _remove_self_references_from_im_graph(
    im_graph: &mut DiGraph<NodeId, Weight>,
    indices: &HashMap<NodeId, NodeIndex>,
  ) {
    log_verbose!("Remove self references");
    for (_, src_index) in indices.iter() {
      let mut edges_to_remove = Vec::new();
      for edge in im_graph.edges(*src_index) {
        if edge.target() == *src_index {
          edges_to_remove.push(edge.id());
        }
      }
      for edge_id in edges_to_remove {
        im_graph.remove_edge(edge_id);
      }
    }
  }

  pub fn read_graph(
    &mut self,
    context_str: &str, // Renamed for clarity from context
    ego_str: &str,     // Renamed for clarity from ego
    focus_str: &str,   // Renamed for clarity from focus
    positive_only: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    log_command!(
      "{:?} {:?} {:?} {} {} {}",
      context_str,
      ego_str,
      focus_str,
      positive_only,
      index,
      count
    );

    let (ego_id, focus_id, mut im_graph, mut indices, mut ids) = match self
      ._validate_read_graph_params_and_setup(context_str, ego_str, focus_str)
    {
      Ok(data) => data,
      Err(msg) => {
        log_error!("{}", msg);
        return vec![];
      },
    };

    // Clone node_infos once for use by helpers
    let node_infos = self.node_infos.clone();
    // Read settings field before mutable borrow for subgraph
    let force_read_graph_conn = self.settings.force_read_graph_conn;
    // Get subgraph once (requires &mut self)
    let subgraph = self.get_subgraph_from_context(context_str); // This makes read_graph &mut self

    Self::_add_shortest_path_and_forced_connections(
      // Call as static-like or pass relevant fields from self
      force_read_graph_conn, // Pass variable
      ego_id,
      focus_id,
      &mut im_graph,
      &mut indices,
      &mut ids,
      subgraph,
      &node_infos,
    );

    let focus_neighbors = subgraph.all_outbound_neighbors_normalized(focus_id);
    // Call _add_focus_neighbor_connections without self as it no longer needs it
    Self::_add_focus_neighbor_connections(
      focus_id,
      &mut im_graph,
      &mut indices,
      &mut ids,
      subgraph,
      &node_infos,
      positive_only,
      &focus_neighbors,
    );

    Self::_remove_self_references_from_im_graph(&mut im_graph, &indices);

    self.collect_all_edges(
      &indices,
      &ids,
      &im_graph,
      context_str,
      ego_id,
      index,
      count,
    )
  }

  fn _extract_unique_edges_from_graph_data(
    &self, // For self.node_infos used in node_name_from_id
    indices: &HashMap<NodeId, NodeIndex>,
    ids: &HashMap<NodeIndex, NodeId>,
    im_graph: &DiGraph<NodeId, Weight>,
  ) -> Vec<(NodeId, NodeId, Weight)> {
    log_verbose!("Build final array of unique edges from graph data");
    let mut edge_ids = Vec::<(NodeId, NodeId, Weight)>::new();
    // Pre-allocate with a reasonable guess, though actual number of unique edges can vary.
    edge_ids.reserve_exact(indices.len() * 2);

    for src_index in indices.values() {
      for edge in im_graph.edges(*src_index) {
        if let (Some(&src_id), Some(&dst_id)) = // Dereference here
          (ids.get(src_index), ids.get(&edge.target()))
        {
          let w = *edge.weight();
          if w > -EPSILON && w < EPSILON {
            // Check for zero weight
            log_error!(
              "Got zero edge weight: {} -> {}",
              node_name_from_id(&self.node_infos, src_id),
              node_name_from_id(&self.node_infos, dst_id)
            );
          } else {
            // Check for duplicate edges before pushing
            let mut found = false;
            for (x, y, _) in edge_ids.iter() {
              if src_id == *x && dst_id == *y {
                found = true;
                break;
              }
            }
            if !found {
              edge_ids.push((src_id, dst_id, w));
            }
          }
        } else {
          log_error!("Got invalid node index during edge extraction");
        }
      }
    }
    edge_ids
  }

  fn _sort_paginate_and_format_graph_edges(
    &mut self, // For fetch_score, get_object_owner, fetch_score_cached
    mut edge_ids: Vec<(NodeId, NodeId, Weight)>, // Mutable for sorting
    context_str: &str,
    ego_id: NodeId,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    edge_ids.sort_by(|(_, _, a), (_, _, b)| b.abs().total_cmp(&a.abs()));

    edge_ids
      .into_iter()
      .skip(index as usize)
      .take(count as usize)
      .map(|(src_id, dst_id, weight_of_dst)| {
        let (score_value_of_dst, score_cluster_of_dst) =
          self.fetch_score(context_str, ego_id, dst_id);
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(context_str, dst_id) {
            Some(dst_owner_id) => {
              self.fetch_score_cached(context_str, dst_owner_id, ego_id)
            },
            None => (0.0, 0),
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

  fn collect_all_edges(
    &mut self,
    indices: &HashMap<NodeId, NodeIndex>,
    ids: &HashMap<NodeIndex, NodeId>,
    im_graph: &DiGraph<NodeId, Weight>,
    context_str: &str, // Renamed from context for consistency
    ego_id: NodeId,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    let unique_edges =
      self._extract_unique_edges_from_graph_data(indices, ids, im_graph);
    self._sort_paginate_and_format_graph_edges(
      unique_edges,
      context_str,
      ego_id,
      index,
      count,
    )
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
    v.reserve(infos.len() * 2);

    for src_id in 0..infos.len() {
      let src_name = infos[src_id].name.as_str();
      if let Some(data) = self
        .subgraph_from_context(context)
        .meritrank_data
        .graph
        .get_node_data(src_id)
      {
        for (dst_id, weight) in data.get_outgoing_edges() {
          match infos.get(dst_id) {
            Some(x) => v.push((src_name.to_string(), x.name.clone(), weight)),
            None => log_error!("Node does not exist: {}", dst_id),
          }
        }
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
      // Ensure info.kind is Option<NodeKind> and default is None
      let info = self.node_infos.get(node).cloned().unwrap_or_else(|| NodeInfo {
        kind: None, // Default to None if node info is missing
        name: "".to_string(),
        seen_nodes: Vec::new(),
      });
      if score_value_of_dst > 0.0 && info.kind == Some(NodeKind::User) { // Compare with Some(NodeKind::User)
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(context, node) {
            Some(dst_owner_id) => {
              self.fetch_score_cached(context, dst_owner_id, ego_id)
            },
            None => (0.0, 0),
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
    v
  }
}

// === A* CODE MOVED FROM operations.rs ===

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
  // Now private
  graph: &Graph,
  ego_id: NodeId,
  focus_id: NodeId,
) -> Result<Vec<NodeId>, AStarError> {
  let mut open: Vec<Node<NodeId, Weight>> = vec![];
  let mut closed: Vec<Node<NodeId, Weight>> = vec![];

  open.resize(1024, Node::default());
  closed.resize(1024, Node::default());

  let mut astar_state = init(&mut open, ego_id, focus_id, 0.0);
  let mut steps = 0;
  let mut neighbor = None;
  let mut status = Status::PROGRESS;

  for _ in 0..10000 {
    // Max 10000 iterations
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
      Status::SUCCESS | Status::FAIL => break,
      Status::PROGRESS => {},
    };
  }

  log_verbose!("Did {} A* iterations", steps);

  if status == Status::SUCCESS {
    log_verbose!("Path found");
    let mut ego_to_focus: Vec<NodeId> = vec![0; astar_state.num_closed];
    let n = path(&closed, &astar_state, &mut ego_to_focus);
    ego_to_focus.resize(n, 0);
    Ok(ego_to_focus)
  } else if status == Status::FAIL {
    Err(AStarError::PathDoesNotExist(ego_id, focus_id))
  } else {
    Err(AStarError::SearchExhausted(ego_id, focus_id))
  }
}

// Helper method to find the shortest path from ego to focus and add it to the graph
fn add_shortest_path_to_graph(
  // Now private
  subgraph: &Subgraph,
  node_infos: &Vec<NodeInfo>, // Already available in read_ops scope via self.node_infos
  ego_id: NodeId,
  focus_id: NodeId,
  indices: &mut HashMap<NodeId, NodeIndex>,
  ids: &mut HashMap<NodeIndex, NodeId>,
  im_graph: &mut DiGraph<NodeId, Weight>,
) {
  log_verbose!("Search shortest path");
  let ego_to_focus = match perform_astar_search(
    // Direct call
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

  let mut edges = Vec::<(NodeId, NodeId, Weight)>::new();
  edges.reserve_exact(ego_to_focus.len().saturating_sub(1)); // Avoid underflow if len is 0

  log_verbose!("Process shortest path");
  for k in 0..ego_to_focus.len().saturating_sub(1) {
    // Avoid underflow
    let a = ego_to_focus[k];
    let b = ego_to_focus[k + 1];
    let a_kind_opt = node_kind_from_id(node_infos, a); // Returns Option<NodeKind>
    let b_kind_opt = node_kind_from_id(node_infos, b); // Returns Option<NodeKind>
    let a_b_weight = subgraph.edge_weight_normalized(a, b);

    if k + 2 == ego_to_focus.len() {
      if a_kind_opt == Some(NodeKind::User) {
        edges.push((a, b, a_b_weight));
      } else {
        log_verbose!("Ignore node {}", node_name_from_id(node_infos, a));
      }
    } else if b_kind_opt != Some(NodeKind::User) {
      log_verbose!("Ignore node {}", node_name_from_id(node_infos, b));
      if k + 2 < ego_to_focus.len() {
        // Boundary check
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
      }
    } else if a_kind_opt == Some(NodeKind::User) { // If b_kind was Some(User) or other non-User, and a_kind is User
      edges.push((a, b, a_b_weight));
    } else {
      log_verbose!("Ignore node {}", node_name_from_id(node_infos, a));
    }
  }

  log_verbose!("Add path to the graph");
  for (src, dst, weight) in edges {
    if let std::collections::hash_map::Entry::Vacant(e) = indices.entry(src) {
      let index = im_graph.add_node(src);
      e.insert(index);
      ids.insert(index, src);
    }
    add_edge_if_valid(im_graph, indices, ids, src, dst, weight); // Direct call
  }
}

fn add_edge_if_valid(
  // Now private
  im_graph: &mut DiGraph<NodeId, Weight>,
  indices: &mut HashMap<NodeId, NodeIndex>,
  ids: &mut HashMap<NodeIndex, NodeId>,
  src_id: NodeId,
  dst_id: NodeId,
  focus_dst_weight: Weight,
) {
  if let std::collections::hash_map::Entry::Vacant(e) = indices.entry(src_id) {
    let index = im_graph.add_node(src_id);
    e.insert(index);
    ids.insert(index, src_id);
  }
  if let std::collections::hash_map::Entry::Vacant(e) = indices.entry(dst_id) {
    let index = im_graph.add_node(dst_id);
    e.insert(index);
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
