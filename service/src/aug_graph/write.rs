use meritrank_core::{NodeId, Weight};
use crate::aug_graph::nodes::{NodeKind};
use crate::log::*;

impl AugGraph {
  pub fn set_edge(
    &mut self,
    src: NodeId,
    dst: NodeId,
    amount: Weight,
  ) {
    log_trace!("{} {} {}", src, dst, amount);

    if src == dst {
      log_error!("Self-reference is not allowed.");
      return;
    }
    
    let src_kind_opt = &self.node., src);

    let src_kind_opt = node_kind_from_id(&self.node_infos, src);
    let dst_kind_opt = node_kind_from_id(&self.node_infos, dst);

    match (src_kind_opt, dst_kind_opt) {
      (Some(NodeKind::User), Some(NodeKind::User)) => {
        self.subgraph_from_context(context);

        for (enum_context, subgraph) in &mut self.subgraphs {
          log_verbose!(
              "Set user edge in {:?}: {} -> {} for {}",
              enum_context,
              src,
              dst,
              amount
            );
          subgraph.meritrank_data.set_edge(src, dst, amount);
        }
      },
      (Some(NodeKind::User), Some(NodeKind::PollVariant)) => {
        match self
          .subgraph_from_context(context)
          .poll_store
          .add_user_vote(src, dst, amount)
        {
          Ok(_) => {
            log_verbose!(
                "Set User -> PollOption edge in {:?}: {} -> {} for {}",
                context,
                src,
                dst,
                amount
              );
          },
          Err(e) => {
            log_error!(
              "Failed to add user vote: User {} -> PollOption {} with amount {}. Error: {}",
              src,
              dst,
              amount,
              e
          );
          },
        }
      },
      (Some(NodeKind::PollVariant), Some(NodeKind::Poll)) => {
        match self
          .subgraph_from_context(context)
          .poll_store
          .add_poll_option(src, dst)
        {
          Ok(_) => {
            log_verbose!(
                "Set PollOption -> Poll edge in {:?}: {} -> {} for {}",
                context,
                src,
                dst,
                amount
              );
          },
          Err(e) => {
            log_error!(
                "Failed to add poll option: PollOption {} -> Poll {}. Error: {}",
                src,
                dst,
                e
              );
          },
        }
      },
      (src_kind, dst_kind)
      if src_kind == Some(NodeKind::PollVariant)
        || src_kind == Some(NodeKind::Poll)
        || dst_kind == Some(NodeKind::PollVariant)
        || dst_kind == Some(NodeKind::Poll) =>
        {
          log_warning!("Unexpected edge type: {:?} -> {:?} in context {:?}. No action taken.", src_kind_opt, dst_kind_opt, context);
        },
      _ => {
        if context.is_empty() {
          log_verbose!("Set edge in \"\": {} -> {} for {}", src, dst, amount);
          self
            .subgraph_from_context(context)
            .meritrank_data
            .set_edge(src, dst, amount);
        } else {
          let null_weight =
            self.subgraph_from_context("").edge_weight(src, dst);
          let old_weight =
            self.subgraph_from_context(context).edge_weight(src, dst);
          let delta = null_weight + amount - old_weight;

          log_verbose!("Set edge in \"\": {} -> {} for {}", src, dst, delta);
          self
            .subgraph_from_context("")
            .meritrank_data
            .set_edge(src, dst, delta);

          log_verbose!(
              "Set edge in {:?}: {} -> {} for {}",
              context,
              src,
              dst,
              amount
            );
          self
            .subgraph_from_context(context)
            .meritrank_data
            .set_edge(src, dst, amount);
        }
      },
    }
  }

}