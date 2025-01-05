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
    if base_weight.is_nan() || base_weight <= 0.0 {
      return (base_weight, vec![]);
    }

    let mut ops = Vec::new();

    let new_weight = base_weight * self.bump_factor.powi(bumps as i32);
    let src_key = (ctx.to_string(), src_id.to_string());

    if new_weight > self.max_threshold {
      if let Some(max_weight) = self.max_indices.get(&src_key).copied() {
        for &(dst, weight) in edges_data {
          let normalized_weight = weight / max_weight;
          ops.push(GraphOp::SetEdge {
            context: ctx.to_string(),
            src_id,
            dst_id,
            weight: normalized_weight,
          });
        }

        self.max_indices.insert(src_key.clone(), 1.0);
      }
    }

    if let Some(&max_weight) =
      self.max_indices.get(&(ctx.to_string(), src_id.to_string()))
    {
      let threshold = max_weight * self.deletion_ratio;

      for &(dst, weight) in edges_data {
        if weight <= threshold {
          ops.push(GraphOp::RemoveEdge {
            context: ctx.to_string(),
            src_id,
            dst_id: dst,
          });
        }
      }
    }

    ops.push(GraphOp::SetEdge {
      context: ctx.to_string(),
      src_id,
      dst_id,
      weight: new_weight,
    });

    (new_weight, ops)
  }
}
