use meritrank_core::{NodeId, Weight};

use std::collections::HashMap;
use std::env;

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

#[derive(Debug)]
pub enum GraphOp {
  SetEdge {
    context: String,
    src_id:  NodeId,
    dst_id:  NodeId,
    weight:  Weight,
  },
  RemoveEdge {
    context: String,
    src_id:  NodeId,
    dst_id:  NodeId,
  },
}

#[derive(Clone, Debug)]
pub struct VSIDSManager {
  pub(crate) min_max_weights: HashMap<(String, NodeId), (Weight, Weight, u32)>,
  pub bump_factor:       Weight,
  pub(crate) rescale_threshold: Weight,
  pub(crate) deletion_ratio:    Weight,
}

impl Default for VSIDSManager {
  fn default() -> Self {
    Self::new()
  }
}

impl VSIDSManager {
  pub fn new() -> Self {
    let bump_factor = env::var("VSIDS_BUMP")
      .ok()
      .and_then(|v| v.parse().ok())
      .unwrap_or(1.111_111);
    Self {
      min_max_weights: HashMap::with_capacity(100),
      bump_factor,
      rescale_threshold: 1e15,
      deletion_ratio: 1e-3,
    }
  }

  pub fn scale_weight(
    &self,
    context: &str,
    src_id: NodeId,
    new_weight: Weight,
    new_magnitude: u32,
  ) -> (Weight, Weight, Weight, u32, f64) {
    let src_key = (context.to_string(), src_id);
    let (current_min, current_max, current_mag_scale) = self
      .min_max_weights
      .get(&src_key)
      .copied()
      .unwrap_or((f64::MAX, 0.0, 0));

    let new_scale_factor = self
      .bump_factor
      .powi(new_magnitude as i32 - current_mag_scale as i32);
    let mut scaled_weight = new_weight * new_scale_factor;
    let scaled_weight_abs = scaled_weight.abs();

    let current_scale_factor = self.bump_factor.powi(current_mag_scale as i32);

    let mut updated_min = current_min.min(scaled_weight_abs);
    let mut updated_max = current_max.max(scaled_weight_abs);
    let mut rescale_factor = 1.0;
    let mut updated_mag_scale = current_mag_scale;
    if scaled_weight_abs > self.rescale_threshold {
      // Have to rescale everything, so reuse the new weight unscaled
      scaled_weight = new_weight;
      updated_min = current_min / current_scale_factor;
      updated_max = new_weight;
      updated_mag_scale = new_magnitude;
      rescale_factor = current_scale_factor;
    };

    (
      scaled_weight,
      updated_min,
      updated_max,
      updated_mag_scale,
      rescale_factor,
    )
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
  fn test_basic_weight_update() {
    let vsids = setup_vsids();
    let (weight, min, max, _, _) = vsids.scale_weight("test", 1, 1.0, 1);

    assert!((weight - 1.111_111).abs() < 1e-6);
    assert_eq!(min, weight.abs());
    assert_eq!(max, weight.abs());
  }

  #[test]
  fn test_deletion_of_small_edges() {
    let vsids = setup_vsids();

    let (_, _, _, _, _) = vsids.scale_weight("test", 1, 1.0, 0);

    let small_weight = vsids.deletion_ratio / 2.0;
    let (weight, min, max, _, _) =
      vsids.scale_weight("test", 1, small_weight, 0);

    assert!(weight.abs() < vsids.deletion_ratio);
    assert!(min <= small_weight);
    assert!(max >= small_weight);
  }
}
