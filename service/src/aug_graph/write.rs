use crate::aug_graph::nodes::{node_kind_from_prefix, NodeKind};
use crate::aug_graph::{AugGraph, NodeName};
use crate::log::*;
use meritrank_core::{NodeId, Weight};

#[derive(Debug)]
enum AugGraphError {
  SelfReference,
  IncorrectNodeKinds(NodeName, NodeName),
}

impl AugGraph {
  pub fn set_edge(
    &mut self,
    src: NodeName,
    dst: NodeName,
    amount: Weight,
  ) {
    log_trace!("{} {} {}", src, dst, amount);

    match self.reg_owner_and_get_ids(src, dst) {
      Ok((src_id, dst_id)) => {
        self.mr.set_edge(src_id, dst_id, amount);
      },
      Err(e) => match e {
        AugGraphError::SelfReference => {
          log_error!("Self-reference is not allowed.")
        },
        AugGraphError::IncorrectNodeKinds(s, d) => {
          log_error!("Incorrect node kinds combination {} -> {}.", s, d)
        },
      },
    }
  }

  fn reg_owner_and_get_ids(
    &mut self,
    src: NodeName,
    dst: NodeName,
  ) -> Result<(NodeId, NodeId), AugGraphError> {
    if src == dst {
      return Err(AugGraphError::SelfReference);
    }

    let (src, dst) = (src.clone(), dst.clone());
    match (node_kind_from_prefix(&src), node_kind_from_prefix(&dst)) {
      (Some(NodeKind::User), Some(NodeKind::User)) => {
        let src_id = self.nodes.register(src, NodeKind::User);
        let dst_id = self.nodes.register(dst, NodeKind::User);
        Ok((src_id, dst_id))
      },
      (Some(src_kind), Some(NodeKind::User)) => {
        let src_id = self.nodes.register(src, NodeKind::User);
        let dst_id = self.nodes.register_with_owner(dst, src_kind, src_id);
        Ok((src_id, dst_id))
      },
      _ => Err(AugGraphError::IncorrectNodeKinds(src, dst)),
    }
  }
}
