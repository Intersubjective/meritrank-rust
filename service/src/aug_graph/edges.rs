use crate::data::*;
use crate::node_registry::*;
use crate::utils::log::*;
use crate::vsids::Magnitude;

use meritrank_core::{NodeId, Weight};

use super::{AugGraph, AugGraphError};

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
    new_min_weight = self.apply_edge_rescales_and_deletions(
      src_id,
      new_min_weight, // Pass current new_min_weight
      edge_deletion_threshold,
      rescale_factor,
      must_rescale,
    );

    match self.mr.set_edge(src_id, dst_id, new_weight_scaled) {
      Ok(_) => {},
      Err(e) => {
        log_error!("{}", e);
      },
    };

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

  fn apply_edge_rescales_and_deletions(
    &mut self,
    src_id: NodeId,
    current_min_weight: Weight,
    edge_deletion_threshold: Weight,
    rescale_factor: f64,
    must_rescale: bool,
  ) -> Weight {
    let node_data = match self.mr.graph.get_node_data(src_id) {
      Some(x) => x,
      None => {
        log_error!("Unable to get node data.");
        return 0.0;
      },
    };

    let (edges_to_modify, new_min_weight_from_scan) =
      node_data.get_outgoing_edges().fold(
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
      match self.mr.set_edge(src_id, dst_id_iter, weight_iter) {
        Ok(_) => {},
        Err(e) => {
          log_error!("{}", e);
        },
      };
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

    match self.reg_owner_and_get_ids(src.clone(), dst.clone()) {
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

  /// Bulk-load edges without creating walks. Clears walks first; VSIDS is not reset.
  /// Used for cold start. Walks are created lazily on first read via ensure_calculated.
  pub fn bulk_load_edges(
    &mut self,
    edges: Vec<OpWriteEdge>,
  ) {
    self.mr.clear_walks();
    for edge in edges {
      match self.reg_owner_and_get_ids(edge.src.clone(), edge.dst.clone()) {
        Ok((src_id, dst_id)) => {
          self.set_edge_by_id(src_id, dst_id, edge.amount, edge.magnitude);
        },
        Err(e) => match e {
          AugGraphError::SelfReference => {
            log_error!("Bulk load: self-reference skipped");
          },
          AugGraphError::IncorrectNodeKinds(s, d) => {
            log_error!("Bulk load: bad node kinds {} -> {}, skipped", s, d);
          },
        },
      }
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

    let opt_src_kind = node_kind_from_prefix(&src);
    let opt_dst_kind = node_kind_from_prefix(&dst);

    match (opt_src_kind, opt_dst_kind) {
      (Some(NodeKind::User), Some(NodeKind::User)) => {
        let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
        let dst_id = self.nodes.register(&mut self.mr, dst, NodeKind::User);
        Ok((src_id, dst_id))
      },
      (Some(src_kind), Some(NodeKind::User)) => {
        let dst_id = self.nodes.register(&mut self.mr, dst, NodeKind::User);
        let src_id =
          self
            .nodes
            .register_with_owner(&mut self.mr, src, src_kind, dst_id);
        Ok((src_id, dst_id))
      },
      (Some(NodeKind::User), Some(dst_kind)) => {
        let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
        let dst_id = self.nodes.register(&mut self.mr, dst, dst_kind);
        Ok((src_id, dst_id))
      },
      (Some(_), Some(_)) => Err(AugGraphError::IncorrectNodeKinds(src, dst)),
      _ => Err(AugGraphError::IncorrectNodeKinds(src, dst)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::data::NodeKind;
  use crate::settings::Settings;

  #[test]
  fn ownership_assigned_on_nonuser_to_user_edge() {
    let mut aug = AugGraph::new(Settings::default());
    aug.set_edge("O1".into(), "U1".into(), 1.0, 0);

    let o1 = aug.nodes.get_by_name("O1").unwrap();
    let u1 = aug.nodes.get_by_name("U1").unwrap();
    assert_eq!(o1.kind, NodeKind::Opinion);
    assert_eq!(o1.owner, Some(u1.id));
    assert_eq!(u1.kind, NodeKind::User);
    assert_eq!(u1.owner, None);

    assert_eq!(aug.get_object_owner(o1.id), Some(u1.id));
    assert_eq!(aug.get_object_owner(u1.id), Some(u1.id));
  }

  #[test]
  fn ownership_stable_across_subsequent_edges() {
    let mut aug = AugGraph::new(Settings::default());
    aug.set_edge("O1".into(), "U1".into(), 1.0, 0);
    aug.set_edge("O1".into(), "U2".into(), 1.0, 0);

    let o1 = aug.nodes.get_by_name("O1").unwrap();
    let u1 = aug.nodes.get_by_name("U1").unwrap();
    let u2 = aug.nodes.get_by_name("U2").unwrap();
    assert_eq!(o1.owner, Some(u1.id));
    assert_eq!(u2.owner, None);
  }

  fn default_graph() -> AugGraph {
    AugGraph::new(Settings {
      num_walks:              50,
      zero_opinion_num_walks: 100,
      zero_opinion_factor:    0.0,
      ..Settings::default()
    })
  }

  #[test]
  fn edge_uncontexted() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 1.5, 0);

    let edges: Vec<_> = graph
      .mr
      .graph
      .get_node_data(graph.nodes.get_by_name("U1").unwrap().id)
      .unwrap()
      .get_outgoing_edges()
      .collect();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].1, 1.5);
  }

  #[test]
  fn delete_nodes() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);

    let u1_id = graph.nodes.get_by_name("U1").unwrap().id;
    let u2_id = graph.nodes.get_by_name("U2").unwrap().id;

    graph.mr.set_edge(u1_id, u2_id, 0.0).unwrap();

    let data = graph.mr.graph.get_node_data(u1_id);
    let edge_count = match data {
      Some(d) => d.get_outgoing_edges().count(),
      None => 0,
    };
    assert_eq!(edge_count, 0);
  }

  #[test]
  fn regression_delete_self_reference_panic() {
    let mut graph = default_graph();
    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
    // Self-reference should be rejected gracefully (no panic)
    graph.set_edge("U1".into(), "U1".into(), 1.0, 0);
  }
}
