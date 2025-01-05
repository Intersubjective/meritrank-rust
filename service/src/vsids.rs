use crate::operations::AugMultiGraph;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};

//////////////////////////////////////////////////////////
//   VSIDS (Variable State Independent Decaying Sum)    //
//======================================================//
//                                                      //
// VSIDS is necessary because originally, MeritRank     //
// was designed to work for a network with a stable     //
// number of nodes and edges, without a "time" aspect.  //
// As new nodes and edges are repeatedly added, the     //
// total sum of the weights of outgoing edges keeps     //
// increasing. This results in a diminishing impact     //
// on not only the newly added edges but also on the    //
// older edges, causing the entropy to dominate.        //
//                                                      //
// In this implementation, the `VSIDSManager` is        //
// responsible for managing edge weights in a graph-    //
// like structure. It adjusts the weights of edges      //
// based on a configurable bump factor and performs     //
// essential operations such as normalizing weights,    //
// pruning insignificant edges, and cleaning up         //
// outdated data to maintain efficiency.                //
//                                                      //
// The algorithm uses an exponential decay mechanism    //
// to handle the inflation problem of edge weights,     //
// where each time an edge is updated, the other        //
// edges outgoing from the same node are decayed by     //
// a factor. This helps to prioritize the most recent   //
// updates, ensuring that the "importance" of edges     //
// remains dynamic and reflects current relevance.      //
//                                                      //
// Key features include:                                //
// - Dynamic weight adjustment based on updates         //
// - Decay of older edges to prevent "weight inflation" //
// - Normalization when weight thresholds are reached   //
// - Pruning and cleanup strategies to ensure memory    //
//   efficiency and avoid excessive growth in data      //
// - Handling of floating point limits through          //
//   renormalization.                                   //
//                                                      //
// The decay mechanism in VSIDS helps mitigate this     //
// issue by adjusting the weight of edges over time,    //
// ensuring the newer edges retain more relevance.      //
//////////////////////////////////////////////////////////

type Edge = (String, String, String);
type SourceKey = (String, String);
type Weight = f64;
pub type NodeId = usize;

#[derive(Debug)]
pub enum GraphOp {
  SetEdge {
    context: String,
    src_id: NodeId,
    dst_id: NodeId,
    weight: f64,
  },
  RemoveEdge {
    context: String,
    src_id: NodeId,
    dst_id: NodeId,
  },
}

#[derive(Clone, Debug)]
pub struct VSIDSManager {
  weights: HashMap<Edge, Weight>,
  max_indices: HashMap<SourceKey, Weight>,
  bump_factor: Weight,
  max_threshold: Weight,
  deletion_ratio: Weight,
}

impl VSIDSManager {
  pub fn new() -> Self {
    let bump_factor = env::var("VSIDS_BUMP")
      .ok()
      .and_then(|v| v.parse().ok())
      .unwrap_or(1.111_111);
    Self {
      weights: HashMap::with_capacity(1000),
      max_indices: HashMap::with_capacity(100),
      bump_factor,
      max_threshold: 1e15,
      deletion_ratio: 1e-3,
    }
  }

  pub fn update_weight(
    &mut self,
    edges_data: &[(NodeId, f64)],
    ctx: &str,
    src_id: NodeId,
    dst_id: NodeId,
    base_weight: f64,
    bumps: u32,
  ) -> (f64, Vec<GraphOp>) {
    if base_weight.is_nan() {
      return (base_weight, vec![]);
    }

    let mut ops = Vec::new();
    let new_weight = base_weight * self.bump_factor.powi(bumps as i32);
    let src_key = (ctx.to_string(), src_id.to_string());

    // Update max_indices with the new weight if it's larger
    let current_max = self.max_indices.get(&src_key).copied().unwrap_or(0.0);
    if new_weight.abs() > current_max {
      self.max_indices.insert(src_key.clone(), new_weight.abs());
    }

    if new_weight.abs() > self.max_threshold {
      if let Some(max_weight) = self.max_indices.get(&src_key).copied() {
        // Normalize all existing edges
        for &(dst, weight) in edges_data {
          let normalized_weight = weight / max_weight;
          ops.push(GraphOp::SetEdge {
            context: ctx.to_string(),
            src_id,
            dst_id: dst,
            weight: normalized_weight,
          });
        }
        // Reset max index after normalization
        self.max_indices.insert(src_key.clone(), 1.0);
      }
    }

    // Check for small edges that need deletion
    if let Some(&max_weight) = self.max_indices.get(&src_key) {
      let threshold = max_weight * self.deletion_ratio;
      for &(dst, weight) in edges_data {
        if weight.abs() <= threshold {
          ops.push(GraphOp::RemoveEdge {
            context: ctx.to_string(),
            src_id,
            dst_id: dst,
          });
        }
      }
    }

    // Add the new edge operation
    ops.push(GraphOp::SetEdge {
      context: ctx.to_string(),
      src_id,
      dst_id,
      weight: new_weight,
    });

    (new_weight, ops)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::env;

  fn setup_vsids() -> VSIDSManager {
    env::remove_var("VSIDS_BUMP");
    VSIDSManager::new()
  }

  #[test]
  fn test_new_default_values() {
    let vsids = setup_vsids();
    assert_eq!(vsids.bump_factor, 1.111_111);
    assert_eq!(vsids.max_threshold, 1e15);
    assert_eq!(vsids.deletion_ratio, 1e-3);
    assert!(vsids.weights.is_empty());
    assert!(vsids.max_indices.is_empty());
  }

  #[test]
  fn test_basic_weight_update() {
    let mut vsids = setup_vsids();
    let edges_data = vec![];
    let (weight, ops) = vsids.update_weight(&edges_data, "test", 1, 2, 1.0, 1);

    assert_eq!(weight, 1.111_111);
    assert_eq!(ops.len(), 1);

    match &ops[0] {
      GraphOp::SetEdge {
        context,
        src_id,
        dst_id,
        weight,
      } => {
        assert_eq!(context, "test");
        assert_eq!(*src_id, 1);
        assert_eq!(*dst_id, 2);
        assert!((weight - 1.111_111).abs() < 1e-6);
      },
      _ => panic!("Expected SetEdge operation"),
    }
  }

  #[test]
  fn test_threshold_normalization() {
    let mut vsids = setup_vsids();

    let (_, _) =
      vsids.update_weight(&[], "test", 1, 2, vsids.max_threshold / 2.0, 0);

    let edges_data = vec![
      (2, vsids.max_threshold / 2.0),
      (3, vsids.max_threshold / 4.0),
    ];

    let (_, ops) = vsids.update_weight(
      &edges_data,
      "test",
      1,
      4,
      vsids.max_threshold * 2.0,
      0,
    );

    assert!(ops.len() > 1, "Expected normalization operations");

    for op in ops.iter() {
      if let GraphOp::SetEdge {
        weight,
        ..
      } = op
      {
        if *weight != vsids.max_threshold * 2.0 {
          assert!(
            *weight <= 1.0,
            "Expected normalized weight <= 1.0, got {}",
            weight
          );
        }
      }
    }
  }

  #[test]
  fn test_deletion_of_small_edges() {
    let mut vsids = setup_vsids();

    let (_, _) = vsids.update_weight(&[], "test", 1, 2, 1.0, 0);

    let small_weight = vsids.deletion_ratio / 2.0; // Guaranteed to be below threshold
    let edges_data = vec![(2, small_weight)];

    let (_, ops) = vsids.update_weight(&edges_data, "test", 1, 3, 1.0, 0);

    let has_deletion = ops.iter().any(
      |op| matches!(op, GraphOp::RemoveEdge { dst_id, .. } if *dst_id == 2),
    );
    assert!(has_deletion, "Expected to find edge deletion operation");
  }

  #[test]
  fn test_context_isolation() {
    let mut vsids = setup_vsids();

    let (_, _) = vsids.update_weight(&[], "context1", 1, 2, 1.0, 1);
    let (_, _) = vsids.update_weight(&[], "context2", 1, 2, 1.0, 1);

    assert!(
      vsids
        .max_indices
        .contains_key(&("context1".to_string(), "1".to_string())),
      "Expected max_indices entry for context1"
    );
    assert!(
      vsids
        .max_indices
        .contains_key(&("context2".to_string(), "1".to_string())),
      "Expected max_indices entry for context2"
    );
  }
}
