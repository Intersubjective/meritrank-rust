use std::collections::HashMap;
use std::env;

type Edge = (String, String, String);
type SourceKey = (String, String);

#[derive(Clone, Default)]
pub struct VSIDSManager {
  weights: HashMap<Edge, f64>,
  max_indices: HashMap<SourceKey, f64>,
  bump_factor: f64,
  max_threshold: f64,
  deletion_ratio: f64,
}

impl VSIDSManager {
  pub fn new() -> Self {
    let bump_factor = env::var("VSIDS_BUMP")
      .ok()
      .and_then(|v| v.parse().ok())
      .unwrap_or(1.111_111);

    Self {
      weights: HashMap::new(),
      max_indices: HashMap::new(),
      bump_factor,
      max_threshold: 1e15,
      deletion_ratio: 1e-3,
    }
  }

  pub fn get_weight(
    &self,
    ctx: &str,
    src: &str,
    dst: &str,
  ) -> Option<f64> {
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
    base_weight: f64,
    bumps: u32,
  ) -> f64 {
    let edge = (ctx.to_string(), src.to_string(), dst.to_string());
    let source_key = (ctx.to_string(), src.to_string());
    let new_weight = base_weight * self.bump_factor.powi(bumps as i32);

    self.weights.insert(edge, new_weight);

    let current_max = self.max_indices.get(&source_key).copied().unwrap_or(0.0);
    let new_max = new_weight.max(current_max);
    self.max_indices.insert(source_key.clone(), new_max);

    if new_max > self.max_threshold {
      self.normalize(ctx, src);
      return self.get_weight(ctx, src, dst).unwrap();
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
      if max_weight > 0.0 {
        let normalized: Vec<(Edge, f64)> = self
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
  }

  fn cleanup_small_edges(
    &mut self,
    ctx: &str,
    src: &str,
    max_weight: f64,
  ) {
    let threshold = max_weight * self.deletion_ratio;

    let keys_to_remove: Vec<Edge> = self
      .weights
      .iter()
      .filter(|((c, s, _), &weight)| {
        c == ctx && s == src && weight <= threshold
      })
      .map(|(edge, _)| edge.clone())
      .collect();

    for edge in keys_to_remove {
      self.weights.remove(&edge);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normalization_with_cleanup() {
    let mut mgr = VSIDSManager::new();

    mgr.update_weight("test", "A", "B", 1e14, 0);
    mgr.update_weight("test", "A", "C", 1e14, 1);
    mgr.update_weight("test", "A", "D", 1.0, 0);

    mgr.update_weight("test", "A", "E", 1e14, 2);

    assert!(mgr.get_weight("test", "A", "D").is_none());
    assert!(mgr.get_weight("test", "A", "E").unwrap() <= mgr.max_threshold);
  }

  #[test]
  fn test_basic_weight_operations() {
    let mut mgr = VSIDSManager::new();
    mgr.update_weight("test", "A", "B", 1.0, 0);
    assert_eq!(mgr.get_weight("test", "A", "B"), Some(1.0));
    mgr.update_weight("test", "A", "B", 2.0, 0);
    assert_eq!(mgr.get_weight("test", "A", "B"), Some(2.0));
  }

  #[test]
  fn test_exponential_bump() {
    let mut mgr = VSIDSManager::new();
    mgr.update_weight("test", "A", "B", 1.0, 0);
    mgr.update_weight("test", "A", "C", 1.0, 1);
    let weight_b = mgr.get_weight("test", "A", "B").unwrap();
    let weight_c = mgr.get_weight("test", "A", "C").unwrap();
    assert!((weight_c / weight_b - mgr.bump_factor).abs() < f64::EPSILON);
  }

  #[test]
  fn test_normalization() {
    let mut mgr = VSIDSManager::new();
    mgr.update_weight("test", "A", "B", 1e16, 0);

    assert!(
      mgr.get_weight("test", "A", "B").unwrap() <= mgr.max_threshold,
      "Weight after update should be normalized to be below max_threshold"
    );

    mgr.update_weight("test", "A", "C", 1.0, 0);

    let weight_b = mgr.get_weight("test", "A", "B").unwrap();
    let weight_c = mgr.get_weight("test", "A", "C").unwrap();

    assert!(weight_b <= mgr.max_threshold);
    assert!(weight_c <= mgr.max_threshold);
  }

  #[test]
  fn test_context_isolation() {
    let mut mgr = VSIDSManager::new();
    mgr.update_weight("ctx1", "A", "B", 1.0, 0);
    mgr.update_weight("ctx2", "A", "B", 1.0, 1);
    let weight_ctx1 = mgr.get_weight("ctx1", "A", "B").unwrap();
    let weight_ctx2 = mgr.get_weight("ctx2", "A", "B").unwrap();
    assert!(weight_ctx2 > weight_ctx1);
  }
}
