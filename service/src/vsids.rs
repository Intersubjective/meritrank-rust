use crate::data::Weight;
use meritrank_core::NodeId;

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

#[derive(Clone, Debug)]
pub struct VSIDSManager {
  min_max_weights: HashMap<NodeId, (Weight, Weight, Magnitude)>,
  bump_factor:     Weight,
  rescale_threshold: Weight,
  pub(crate) deletion_ratio: Weight,
}
pub type Magnitude = u32;

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
      .unwrap_or(1.03);
    Self {
      min_max_weights: HashMap::with_capacity(100),
      bump_factor,
      rescale_threshold: 1e15,
      deletion_ratio: 1e-3,
    }
  }

  /// Computes scaled weight and updated (min, max, mag_scale); does not mutate state.
  fn compute_scale(
    &self,
    src_id: NodeId,
    new_weight: Weight,
    new_magnitude: Magnitude,
  ) -> (Weight, Weight, Weight, Magnitude, f64) {
    let (current_min, current_max, current_mag_scale) = self
      .min_max_weights
      .get(&src_id)
      .copied()
      .unwrap_or((f64::MAX, 0.0, 0));

    let new_scale_factor = self
      .bump_factor
      .powi((new_magnitude - current_mag_scale) as i32);
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

  /// Applies an edge update: computes scaled weight, persists (min, max, mag_scale) for this src,
  /// and returns (scaled_weight, rescale_factor, new_max_weight, updated_min) for the caller.
  pub fn apply_edge_update(
    &mut self,
    src_id: NodeId,
    new_weight: Weight,
    new_magnitude: Magnitude,
  ) -> (Weight, f64, Weight, Weight) {
    let (scaled_weight, updated_min, updated_max, updated_mag_scale, rescale_factor) =
      self.compute_scale(src_id, new_weight, new_magnitude);
    self
      .min_max_weights
      .insert(src_id, (updated_min, updated_max, updated_mag_scale));
    (scaled_weight, rescale_factor, updated_max, updated_min)
  }

  /// Updates the stored min for this src after the caller has applied rescales/deletions.
  /// Call this once you have the actual min from the graph (e.g. from apply_edge_rescales_and_deletions).
  pub fn finish_edge_update(&mut self, src_id: NodeId, actual_min: Weight) {
    if let Some(&(_, max, mag)) = self.min_max_weights.get(&src_id) {
      self.min_max_weights.insert(src_id, (actual_min, max, mag));
    }
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
    let mut vsids = setup_vsids();
    let (weight, _rescale_factor, new_max, _updated_min) =
      vsids.apply_edge_update(1, 1.0, 1);

    // magnitude 1 with default bump 1.03: scale = 1.03^1 = 1.03
    assert!((weight - 1.03).abs() < 1e-6);
    assert!((new_max - weight.abs()).abs() < 1e-9);
  }

  #[test]
  fn test_deletion_of_small_edges() {
    let mut vsids = setup_vsids();

    let _ = vsids.apply_edge_update(1, 1.0, 0);

    let small_weight = vsids.deletion_ratio / 2.0;
    let (weight, _rescale_factor, new_max, _) =
      vsids.apply_edge_update(1, small_weight, 0);

    assert!(weight.abs() < vsids.deletion_ratio);
    assert!(new_max >= small_weight);
  }

  #[test]
  fn test_state_updated_after_apply() {
    let mut vsids = setup_vsids();
    let (w1, _, max1, _) = vsids.apply_edge_update(1, 1.0, 0);
    let (w2, _, max2, _) = vsids.apply_edge_update(1, 2.0, 0);
    // Second call sees state from first: max is at least as large, weights are positive
    assert!(w1 > 0.0 && w2 > 0.0);
    assert!(max2 >= max1);
  }

  #[test]
  fn test_finish_edge_update_corrects_min() {
    let mut vsids = setup_vsids();
    let _ = vsids.apply_edge_update(1, 1.0, 0);
    vsids.finish_edge_update(1, 0.5);
    // Next apply should use the corrected min (0.5) in its state
    let (weight, _, new_max, _) = vsids.apply_edge_update(1, 0.6, 0);
    assert!(weight > 0.0);
    assert!(new_max >= 0.6);
  }
}
