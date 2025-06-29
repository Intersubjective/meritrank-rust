use meritrank_core::{NodeId, Weight};
use crate::aug_graph::{AugGraph, NodeName};
use crate::aug_graph::nodes::{node_kind_from_prefix, NodeKind};
use crate::log::*;

impl AugGraph {
  pub fn set_edge(
    &mut self,
    src_str: NodeName,
    dst_str: NodeName,
    amount: Weight,
  ) {
    log_trace!("{} {} {}", src_str, dst_str, amount);

    if src_str == dst_str {
      log_error!("Self-reference is not allowed.");
      return;
    }
    if node_kind_from_prefix(&src_str).is_none() || node_kind_from_prefix(&dst_str).is_none() {
      log_error!("Can't find node kind for {} or {}.", src_str, dst_str);
      return;
      
    }
    let src_id = self.nodes.register(src_str.clone(), node_kind_from_prefix(&src_str).unwrap());
    let dst_id = self.nodes.register(dst_str.clone(), node_kind_from_prefix(&dst_str).unwrap());
    self.mr.set_edge(src_id, dst_id, amount);
  }
  
  

}