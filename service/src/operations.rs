//  ================================================
//
//    Commands
//
//  ================================================

use meritrank_core::NodeId; // EPSILON and Graph removed
use petgraph::graph::{DiGraph, NodeIndex}; 
use std::collections::HashMap; 

use crate::aug_multi_graph::*; // Imports AugMultiGraph, NodeKind
use crate::constants::*; // For VERSION
use crate::log::*;
use crate::nodes::{Weight, Cluster}; // NodeInfo removed
// Subgraph removed

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
    crate::read_ops::read_node_score(self, context, ego, dst)
  }

  // Removed apply_filters_and_pagination

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
    crate::read_ops::read_scores(self, context, ego, kind_str, hide_personal, score_lt, score_lte, score_gt, score_gte, index, count)
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
    crate::read_ops::read_neighbors(self, context, ego, focus, direction, kind_str, hide_personal, score_lt, score_lte, score_gt, score_gte, index, count)
  }

  pub fn write_create_context(
    &mut self,
    context: &str,
  ) {
    crate::write_ops::write_create_context(self, context)
  }

  pub fn write_put_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    new_weight: f64,
    magnitude: i64,
  ) {
    crate::write_ops::write_put_edge(self, context, src, dst, new_weight, magnitude)
  }

  pub fn write_delete_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    _index: i64,
  ) {
    crate::write_ops::write_delete_edge(self, context, src, dst, _index)
  }

  pub fn write_delete_node(
    &mut self,
    context: &str,
    node: &str,
    _index: i64,
  ) {
    crate::write_ops::write_delete_node(self, context, node, _index)
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
    crate::read_ops::read_graph(self, context, ego, focus, positive_only, index, count)
  }

  // Removed collect_all_edges

  pub fn read_connected(
    &mut self,
    context: &str,
    ego: &str,
  ) -> Vec<(String, String)> {
    crate::read_ops::read_connected(self, context, ego)
  }

  pub fn read_node_list(&self) -> Vec<(String,)> {
    crate::read_ops::read_node_list(self)
  }

  pub fn read_edges(
    &mut self,
    context: &str,
  ) -> Vec<(String, String, Weight)> {
    crate::read_ops::read_edges(self, context)
  }

  pub fn read_mutual_scores(
    &mut self,
    context: &str,
    ego: &str,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    crate::read_ops::read_mutual_scores(self, context, ego)
  }

  pub fn write_reset(&mut self) {
    crate::write_ops::write_reset(self)
  }

  pub fn read_new_edges_filter(
    &mut self,
    src: &str,
  ) -> Vec<u8> {
    crate::read_ops::read_new_edges_filter(self, src)
  }

  pub fn write_new_edges_filter(
    &mut self,
    src: &str,
    filter_bytes: &[u8],
  ) {
    crate::write_ops::write_new_edges_filter(self, src, filter_bytes)
  }

  pub fn write_fetch_new_edges(
    &mut self,
    src: &str,
    prefix: &str,
  ) -> Vec<(String, Weight, Weight, Cluster, Cluster)> {
    crate::write_ops::write_fetch_new_edges(self, src, prefix)
  }

  pub fn write_set_zero_opinion(
    &mut self,
    context: &str,
    node: &str,
    score: Weight,
  ) {
    crate::write_ops::write_set_zero_opinion(self, context, node, score)
  }
}
