use crate::nodes::{node_kind_from_prefix, NodeKind};
use crate::vsids::Magnitude;
use crate::aug_graph::{AugGraph, NodeName};
use crate::utils::log::*;
use meritrank_core::{NodeId, Weight};

#[derive(Debug)]
enum AugGraphError {
  SelfReference,
  IncorrectNodeKinds(NodeName, NodeName),
}

impl AugGraph {
  fn set_edge_by_id(
    &mut self,
    src_id: NodeId,
    dst_id: NodeId,
    amount: Weight,
    magnitude: Magnitude,
  ) {
    log_trace!();

    let (
      new_weight_scaled,
      mut new_min_weight, // This will be potentially updated by the helper
      new_max_weight,
      new_mag_scale,
      rescale_factor,
    ) = self.vsids.scale_weight(src_id, amount, magnitude);

    let edge_deletion_threshold = new_max_weight * self.vsids.deletion_ratio;
    // let can_delete_at_least_one_edge = new_min_weight <= edge_deletion_threshold;
    let must_rescale = rescale_factor > 1.0;

    //  FIXME: This condition doesn't allow to create new edges at all.
    // if can_delete_at_least_one_edge || must_rescale {
      new_min_weight = self._apply_edge_rescales_and_deletions(
        src_id,
        new_min_weight, // Pass current new_min_weight
        edge_deletion_threshold,
        rescale_factor,
        must_rescale,
      );

      self.mr.set_edge(src_id, dst_id, amount);

      //  FIXME: Ad hok fix!!!
      log_verbose!("RECALCULATE");
      let _ = self.mr.calculate(src_id, self.settings.num_walks);

      if must_rescale {
        log_verbose!(
          "Rescale performed: src={}, dst={}, normalized_new_weight={}",
          src_id,
          dst_id,
          new_weight_scaled
        );
      } else {
        log_verbose!(
          "Edge updated without rescale: src={}, dst={}, new_weight_scaled={}",
          src_id,
          dst_id,
          new_weight_scaled
        );
      }
      self
        .vsids
        .min_max_weights
        .insert(src_id, (new_min_weight, new_max_weight, new_mag_scale));
    // }
  }

  fn _apply_edge_rescales_and_deletions(
    &mut self,
    src_id: NodeId,
    current_min_weight: Weight,
    edge_deletion_threshold: Weight,
    rescale_factor: f64,
    must_rescale: bool,
  ) -> Weight {
    let node_data = match self
      .mr
      .graph
      .get_node_data(src_id) {
      Some(x) => x,
      None => {
        log_error!("Unable to get node data.");
        return 0.0;
      },
    };

    let (edges_to_modify, new_min_weight_from_scan) = 
      node_data
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
        "Rescale or delete edge: src={}, dst={}, new_weight={}",
        src_id,
        dst_id_iter,
        weight_iter
      );
      self.mr.set_edge(src_id, dst_id_iter, weight_iter);
    }
    new_min_weight_from_scan // Return the updated min_weight
  }

  pub fn set_edge(
    &mut self,
    src: NodeName,
    dst: NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) {
    log_trace!("{:?} {:?} {}", src, dst, amount);

    match self.reg_owner_and_get_ids(src, dst) {
      Ok((src_id, dst_id)) => {
        self.set_edge_by_id(src_id, dst_id, amount, magnitude);
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
        let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
        let dst_id = self.nodes.register(&mut self.mr, dst, NodeKind::User);
        Ok((src_id, dst_id))
      },
      (Some(src_kind), Some(NodeKind::User)) => {
        let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
        let dst_id = self.nodes.register_with_owner(&mut self.mr, dst, src_kind, src_id);
        Ok((src_id, dst_id))
      },
      _ => Err(AugGraphError::IncorrectNodeKinds(src, dst)),
    }
  }
}
