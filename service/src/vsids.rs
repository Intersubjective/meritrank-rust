use std::collections::HashMap;
use std::env;

type Edge = (String, String, String);

#[derive(Clone, Default)]
pub struct VSIDSManager {
    weights: HashMap<Edge, f64>,
    bump_factor: f64,
    max_threshold: f64,
}

impl VSIDSManager {
    pub fn new() -> Self {
        const DEFAULT_BUMP: f64 = 1.111_111;

        let bump_factor = env::var("VSIDS_BUMP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_BUMP);

        Self {
            weights: HashMap::new(),
            bump_factor,
            max_threshold: 1e15,
        }
    }

    pub fn get_weight(&self, ctx: &str, src: &str, dst: &str) -> Option<f64> {
        self.weights
            .get(&(ctx.to_string(), src.to_string(), dst.to_string()))
            .copied()
    }

    pub fn update_weight(&mut self, ctx: &str, src: &str, dst: &str, base_weight: f64, bumps: u32) -> f64 {
        let edge = (ctx.to_string(), src.to_string(), dst.to_string());
        let adjusted_weight = base_weight * self.bump_factor.powi(bumps as i32);

        self.weights.insert(edge.clone(), adjusted_weight);

        if let Some(max_weight) = self.max_source_weight(ctx, src) {
            if max_weight > self.max_threshold {
                self.normalize(ctx, src);
            }
        }

        self.weights[&edge]
    }

    fn max_source_weight(&self, ctx: &str, src: &str) -> Option<f64> {
        self.weights
            .iter()
            .filter(|((edge_ctx, edge_src, _), _)| edge_ctx == ctx && edge_src == src)
            .map(|(_, &weight)| weight)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    fn normalize(&mut self, ctx: &str, src: &str) {
        let max_weight = match self.max_source_weight(ctx, src) {
            Some(w) if w > 0.0 => w,
            _ => return,
        };

        for ((edge_ctx, edge_src, _), weight) in self.weights.iter_mut() {
            if edge_ctx == ctx && edge_src == src {
                *weight /= max_weight;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
