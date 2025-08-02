use crate::data::*;
use crate::node_registry::*;
use crate::utils::{astar::*, log::*};

use meritrank_core::{constants::EPSILON, Graph, NodeId, Weight};
use petgraph::{
  graph::{DiGraph, NodeIndex},
  visit::EdgeRef,
};

use std::collections::HashMap;

pub fn perform_astar_search(
  graph: &Graph,
  ego_id: NodeId,
  focus_id: NodeId,
) -> Result<Vec<NodeId>, ()> {
  log_trace!();

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
    log_error!("Path not found.");
    Err(())
  } else {
    log_error!("Too many iterations.");
    Err(())
  }
}

pub fn add_edge_if_valid(
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

pub fn remove_self_references_from_im_graph(
  im_graph: &mut DiGraph<NodeId, Weight>,
  indices: &HashMap<NodeId, NodeIndex>,
) {
  log_trace!();

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

pub fn extract_unique_edges_from_graph_data(
  indices: &HashMap<NodeId, NodeIndex>,
  ids: &HashMap<NodeIndex, NodeId>,
  im_graph: &DiGraph<NodeId, Weight>,
) -> Vec<(NodeId, NodeId, Weight)> {
  log_trace!();

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
          log_error!("Got zero edge weight: {} -> {}", src_id, dst_id);
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

pub fn filter_and_sort_scores(
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

pub fn prioritize_ego_owned_items(
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
