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

type Edge = (String, String, String);
type SourceKey = (String, String);
type Weight = f64;

#[derive(Clone, Debug)]
pub struct VSIDSManager {
  weights: HashMap<Edge, Weight>,
  max_indices: HashMap<SourceKey, Weight>,
  bump_factor: Weight,
  max_threshold: Weight,
  deletion_ratio: Weight,
  cache_size: usize,
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
      cache_size: 10000,
    }
  }

  #[inline]
  pub fn get_weight(
    &self,
    ctx: &str,
    src: &str,
    dst: &str,
  ) -> Option<Weight> {
    self
      .weights
      .get(&(ctx.to_string(), src.to_string(), dst.to_string()))
      .copied()
  }

  pub fn update_weight(
    &mut self,
    ctx: &str,
    src: &str,
    dst: &str,
    base_weight: Weight,
    bumps: u32,
  ) -> Weight {
    if base_weight.is_nan() || base_weight <= 0.0 {
      return base_weight;
    }

    let edge = (ctx.to_string(), src.to_string(), dst.to_string());
    let source_key = (ctx.to_string(), src.to_string());
    let new_weight = base_weight * self.bump_factor.powi(bumps as i32);

    if self.weights.len() >= self.cache_size {
      self.prune_weights();
    }

    self.weights.insert(edge, new_weight);

    let current_max = self.max_indices.get(&source_key).copied().unwrap_or(0.0);
    let new_max = new_weight.max(current_max);
    self.max_indices.insert(source_key.clone(), new_max);

    if new_max > self.max_threshold {
      self.normalize(ctx, src);
      return self.get_weight(ctx, src, dst).unwrap_or(new_weight);
    }

    self.cleanup_small_edges(ctx, src, new_max);
    new_weight
  }

  fn normalize(
    &mut self,
    ctx: &str,
    src: &str,
  ) {
    let source_key = (ctx.to_string(), src.to_string());
    if let Some(&max_weight) = self.max_indices.get(&source_key) {
      if max_weight <= 0.0 {
        return;
      }

      let normalized: Vec<_> = self
        .weights
        .iter()
        .filter(|((c, s, _), _)| c == ctx && s == src)
        .map(|(edge, &weight)| (edge.clone(), weight / max_weight))
        .collect();

      for (edge, weight) in normalized {
        self.weights.insert(edge, weight);
      }

      self.max_indices.insert(source_key, 1.0);
    }
  }

  fn cleanup_small_edges(
    &mut self,
    ctx: &str,
    src: &str,
    max_weight: Weight,
  ) {
    let threshold = max_weight * self.deletion_ratio;
    self.weights.retain(|(c, s, _), weight| {
      let keep = !(c == ctx && s == src && *weight <= threshold);
      if !keep {
        println!("Removing edge: ({}, {}, _) with weight {}", c, s, *weight);
      }
      keep
    });
  }

  fn prune_weights(&mut self) {
    let target_size = self.cache_size * 9 / 10;
    if self.weights.len() <= target_size {
      return;
    }

    let mut weights: Vec<_> = self.weights.drain().collect();
    weights.sort_unstable_by(|a, b| {
      b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    weights.truncate(target_size);

    self.weights = weights.into_iter().collect();
    self.update_max_indices();
  }

  fn update_max_indices(&mut self) {
    self.max_indices.clear();
    for ((ctx, src, _), weight) in &self.weights {
      let key = (ctx.clone(), src.clone());
      let entry = self.max_indices.entry(key).or_insert(0.0);
      *entry = (*entry).max(*weight);
    }
  }

  pub fn clear(&mut self) {
    self.weights.clear();
    self.max_indices.clear();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_vsids_manager() {
    let manager = VSIDSManager::new();

    assert_eq!(manager.weights.len(), 0);
    assert_eq!(manager.max_indices.len(), 0);
    assert_eq!(manager.bump_factor, 1.111_111);
    assert_eq!(manager.max_threshold, 1e15);
    assert_eq!(manager.deletion_ratio, 1e-3);
    assert_eq!(manager.cache_size, 10000);
  }

  #[test]
  fn test_get_weight() {
    let mut manager = VSIDSManager::new();

    let ctx = "context";
    let src = "source";
    let dst = "destination";
    let weight = 10.0;

    assert_eq!(manager.get_weight(ctx, src, dst), None);

    manager.update_weight(ctx, src, dst, weight, 2);

    assert_eq!(
      manager.get_weight(ctx, src, dst),
      Some(weight * 1.111_111f64.powi(2))
    );
  }

  #[test]
  fn test_update_weight() {
    let mut manager = VSIDSManager::new();

    let ctx = "context";
    let src = "source";
    let dst = "destination";
    let base_weight = 10.0;

    let weight = manager.update_weight(ctx, src, dst, base_weight, 0);
    assert_eq!(weight, base_weight);

    let new_weight = manager.update_weight(ctx, src, dst, base_weight, 2);
    assert_eq!(new_weight, base_weight * 1.111_111f64.powi(2));
  }

  #[test]
  fn test_normalize() {
    let mut manager = VSIDSManager::new();

    let ctx = "context";
    let src = "source";
    let dst = "destination";
    let base_weight = 10.0;

    manager.update_weight(ctx, src, dst, base_weight, 0);

    let another_dst = "another_destination";
    manager.update_weight(ctx, src, another_dst, base_weight * 0.5, 0);

    manager.normalize(ctx, src);

    let normalized_weight = manager.get_weight(ctx, src, dst).unwrap();
    let normalized_another_weight =
      manager.get_weight(ctx, src, another_dst).unwrap();

    assert!(normalized_weight <= 1.0);
    assert!(normalized_another_weight <= 1.0);
  }

  // #[test]
  // fn test_cleanup_small_edges() {
  //   let mut manager = VSIDSManager::new();

  //   let ctx = "ctx";
  //   let src = "src";

  //   manager.update_weight(ctx, src, "small_edge", 0.1, 1);
  //   manager.update_weight(ctx, src, "other_edge", 0.2, 1);

  //   assert_eq!(manager.get_weight(ctx, src, "small_edge"), Some(0.1));
  //   assert_eq!(manager.get_weight(ctx, src, "other_edge"), Some(0.2));

  //   let max_weight = 0.2;
  //   manager.cleanup_small_edges(ctx, src, max_weight);

  //   assert!(manager.get_weight(ctx, src, "small_edge").is_none());
  //   assert_eq!(manager.get_weight(ctx, src, "other_edge"), Some(0.2));
  // }

  #[test]
  fn test_prune_weights() {
    let mut manager = VSIDSManager::new();

    for i in 0..15000 {
      manager.update_weight("ctx", "src", &format!("dst{}", i), 10.0, 0);
    }

    manager.prune_weights();
    assert!(manager.weights.len() <= manager.cache_size * 9 / 10);
  }

  #[test]
  fn test_clear() {
    let mut manager = VSIDSManager::new();
    let ctx = "context";
    let src = "source";
    let dst = "destination";

    manager.update_weight(ctx, src, dst, 10.0, 0);
    manager.update_weight(ctx, src, "another_destination", 5.0, 0);

    assert!(manager.get_weight(ctx, src, dst).is_some());
    assert!(manager
      .get_weight(ctx, src, "another_destination")
      .is_some());

    manager.clear();

    assert!(manager.get_weight(ctx, src, dst).is_none());
    assert!(manager
      .get_weight(ctx, src, "another_destination")
      .is_none());
  }
}
