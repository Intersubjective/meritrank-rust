// service/src/write_ops.rs
use meritrank_core::constants::EPSILON;
use std::sync::atomic::Ordering;

use crate::graph_logic::aug_multi_graph::{AugMultiGraph, Cluster}; // NodeId and Weight removed
use crate::utils::bloom_filter::{
  bloom_filter_add, bloom_filter_bits, bloom_filter_contains,
};
use crate::utils::log::*;
use crate::graph_logic::nodes::*;
use meritrank_core::{NodeId, Weight}; // NodeId and Weight added directly

pub fn write_log_level(log_level: u32) {
  log_command!("{}", log_level);

  ERROR.store(log_level > 0, Ordering::Relaxed);
  WARNING.store(log_level > 1, Ordering::Relaxed);
  INFO.store(log_level > 2, Ordering::Relaxed);
  VERBOSE.store(log_level > 3, Ordering::Relaxed);
  TRACE.store(log_level > 4, Ordering::Relaxed);
}

impl AugMultiGraph {
  pub fn write_create_context(
    &mut self,
    context: &str,
  ) {
    log_command!("{:?}", context);
    self.subgraph_from_context(context);
  }

  fn _apply_edge_rescales_and_deletions(
    &mut self,
    context: &str,
    src_id: NodeId,
    current_min_weight: Weight,
    edge_deletion_threshold: Weight,
    rescale_factor: f64,
    must_rescale: bool,
  ) -> Weight {
    let (edges_to_modify, new_min_weight_from_scan) = self
      .subgraph_from_context(context)
      .meritrank_data
      .graph
      .get_node_data(src_id)
      .unwrap() // Kept as per original, might panic
      .get_outgoing_edges()
      .fold(
        (Vec::new(), current_min_weight), // Use passed current_min_weight
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
            // If not must_rescale, but we are in this block, it implies can_delete_at_least_one_edge is true.
            // Edges that are not rescaled and not deleted are not added to `edges_to_modify`.
            // This preserves the original logic where only edges needing change (deletion or rescale) are processed.
            (to_modify, min.min(abs_weight))
          }
        },
      );

    for (dst_id_iter, weight_iter) in edges_to_modify {
      log_verbose!(
        "Rescale or delete node: context={:?}, src={}, dst={}, new_weight={}",
        context,
        node_name_from_id(&self.node_infos, src_id), // self.node_infos is accessible
        node_name_from_id(&self.node_infos, dst_id_iter),
        weight_iter
      );
      self.set_edge(context, src_id, dst_id_iter, weight_iter);
    }
    new_min_weight_from_scan // Return the updated min_weight
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
      mut new_min_weight, // This will be potentially updated by the helper
      new_max_weight,
      new_mag_scale,
      rescale_factor,
    ) = self
      .vsids
      .scale_weight(context, src_id, new_weight, mag_clamped);

    let edge_deletion_threshold = new_max_weight * self.vsids.deletion_ratio;
    let can_delete_at_least_one_edge =
      new_min_weight <= edge_deletion_threshold;
    let must_rescale = rescale_factor > 1.0;

    if can_delete_at_least_one_edge || must_rescale {
      new_min_weight = self._apply_edge_rescales_and_deletions(
        context,
        src_id,
        new_min_weight, // Pass current new_min_weight
        edge_deletion_threshold,
        rescale_factor,
        must_rescale,
      );
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
    _index: i64, // _index is unused
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
    _index: i64, // _index is unused
  ) {
    log_command!("{:?} {:?}", context, node);

    if !self.node_exists(node) {
      return;
    }

    let id = self.find_or_add_node_by_name(node);

    let outgoing_edges: Vec<NodeId> = self
      .subgraph_from_context(context)
      .meritrank_data
      .graph
      .get_node_data(id)
      .map(|data| data.get_outgoing_edges().map(|(n, _)| n).collect())
      .unwrap_or_default(); // Use unwrap_or_else as per prompt

    for n in outgoing_edges {
      self.set_edge(context, id, n, 0.0);
    }
  }

  pub fn write_reset(&mut self) {
    log_command!();
    self.reset();
  }

  pub fn write_new_edges_filter(
    &mut self,
    src: &str,
    filter_bytes: &[u8],
  ) {
    log_command!("{:?} {:?}", src, filter_bytes);

    let src_id = self.find_or_add_node_by_name(src);

    let mut v: Vec<u64> = vec![0; filter_bytes.len().div_ceil(8)]; // Corrected resize

    for i in 0..filter_bytes.len() {
      v[i / 8] |= (filter_bytes[i] as u64) << (8 * (i % 8)); // Corrected logic
    }

    self.node_infos[src_id].seen_nodes = v;
  }

  fn _initialize_source_node_filter_if_empty(
    &mut self,
    src_id: NodeId,
  ) {
    if self.node_infos[src_id].seen_nodes.is_empty() {
      let min_filter_elements = self.settings.filter_min_size.div_ceil(8);
      self.node_infos[src_id]
        .seen_nodes
        .resize(min_filter_elements, 0);
      log_verbose!(
        "Create the bloom filter with {} bytes for node ID {}", // Changed log to use ID
        8 * self.node_infos[src_id].seen_nodes.len(),
        src_id
      );
    }
  }

  fn _fetch_new_edges_using_filter(
    &mut self,
    src_id: NodeId,
    prefix: &str,
    num_hashes: usize,
  ) -> Vec<(String, Weight, Weight, Cluster, Cluster)> {
    let mut new_edges: Vec<(String, Weight, Weight, Cluster, Cluster)> = vec![];
    let current_filter_len = self.node_infos[src_id].seen_nodes.len();

    // Avoid processing if filter length is 0, as bloom_filter_bits might behave unexpectedly
    // or it simply means no items can be "contained".
    if current_filter_len == 0 {
      log_warning!(
        "Source node {} filter is empty, cannot fetch new edges based on it.",
        src_id
      );
      return new_edges;
    }

    for dst_id in 0..self.node_count {
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

      let bits = bloom_filter_bits(current_filter_len, num_hashes, dst_id);

      if !bloom_filter_contains(&self.node_infos[src_id].seen_nodes, &bits) {
        new_edges.push((
          self.node_infos[dst_id].name.clone(),
          score_value_of_dst,
          score_value_of_src,
          score_cluster_of_dst,
          score_cluster_of_src,
        ));
      }
    }
    new_edges
  }

  fn _rebuild_source_node_filter(
    &mut self,
    src_id: NodeId,
    prefix: &str,
    num_hashes: usize,
    max_filter_elements: usize,
  ) {
    let mut new_filter_data = vec![];

    let initial_filter_elements =
      if self.node_infos[src_id].seen_nodes.is_empty() {
        self.settings.filter_min_size.div_ceil(8)
      } else {
        self.node_infos[src_id].seen_nodes.len()
      };
    new_filter_data.resize(
      std::cmp::min(initial_filter_elements, max_filter_elements),
      0,
    );

    // Handle cases where max_filter_elements is 0, which means filter should be empty.
    if max_filter_elements == 0 {
      if !self.node_infos[src_id].seen_nodes.is_empty() {
        log_verbose!(
          "Max filter elements is 0, clearing filter for node ID {}",
          src_id
        );
      }
      self.node_infos[src_id].seen_nodes = vec![];
      return;
    }

    loop {
      let mut saturated = false;
      for x in new_filter_data.iter_mut() {
        *x = 0;
      }

      let current_filter_elements = new_filter_data.len();
      // If current_filter_elements is 0 after resize (e.g. initial was 0 and max_filter_elements is also 0),
      // then skip processing as bloom_filter_bits might not work as expected.
      // This case is now handled by the early exit for max_filter_elements == 0.

      for dst_id_filter in 0..self.node_count {
        let bits =
          bloom_filter_bits(current_filter_elements, num_hashes, dst_id_filter);
        let collision = bloom_filter_contains(&mut new_filter_data, &bits);

        if collision && current_filter_elements < max_filter_elements {
          let next_size_elements =
            std::cmp::min(current_filter_elements * 2, max_filter_elements);
          if next_size_elements > current_filter_elements {
            // Ensure growth
            new_filter_data.resize(next_size_elements, 0);
            log_verbose!(
              "Resize the bloom filter to {} bytes for node ID {}", // Changed log
              8 * new_filter_data.len(),
              src_id
            );
            saturated = true;
            break;
          } else {
            // Cannot grow further due to max_filter_elements, treat as saturated for this iteration.
            // This means we will try to populate with current size, and if still collisions, it is what it is.
          }
        }

        if self.node_infos[dst_id_filter].name.starts_with(prefix) {
          let k = self.settings.zero_opinion_factor;
          let score = self.subgraph_from_context("").fetch_raw_score(
            src_id,
            dst_id_filter,
            k,
          );

          if !(score < EPSILON) {
            bloom_filter_add(&mut new_filter_data, &bits);
          }
        } else {
          let original_filter_len = self.node_infos[src_id].seen_nodes.len();
          if original_filter_len > 0 {
            // Only check original if it was populated
            let already_seen_in_original = bloom_filter_contains(
              &self.node_infos[src_id].seen_nodes,
              &bloom_filter_bits(
                original_filter_len,
                num_hashes,
                dst_id_filter,
              ),
            );
            if already_seen_in_original {
              bloom_filter_add(&mut new_filter_data, &bits);
            }
          }
        }
      }

      if !saturated {
        if new_filter_data.len() >= max_filter_elements {
          log_warning!(
            "Max bloom filter size is reached for node ID {}",
            src_id
          ); // Changed log
        }
        self.node_infos[src_id].seen_nodes = new_filter_data;
        break;
      }
    }
  }

  pub fn write_fetch_new_edges(
    &mut self,
    src: &str,
    prefix: &str,
  ) -> Vec<(String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?}", src, prefix);

    let num_hashes = self.settings.filter_num_hashes;
    // filter_max_size is in bytes, convert to number of u64 elements
    let max_filter_elements = self.settings.filter_max_size / 8;

    let src_id = self.find_or_add_node_by_name(src);

    self._initialize_source_node_filter_if_empty(src_id);

    let new_edges_found =
      self._fetch_new_edges_using_filter(src_id, prefix, num_hashes);

    self._rebuild_source_node_filter(
      src_id,
      prefix,
      num_hashes,
      max_filter_elements,
    );

    new_edges_found
  }

  pub fn write_set_zero_opinion(
    &mut self,
    context: &str,
    node: &str,
    score: Weight,
  ) {
    log_command!("{:?} {} {}", context, node, score);

    let id = self.find_or_add_node_by_name(node);
    let zero_opinion_map =
      &mut self.subgraph_from_context(context).zero_opinion;

    if id >= zero_opinion_map.len() {
      zero_opinion_map.resize(id + 1, 0.0);
    }
    zero_opinion_map[id] = score;
  }
}
