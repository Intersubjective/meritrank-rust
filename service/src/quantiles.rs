use crate::constants::*;
pub use meritrank_core::{constants::EPSILON, Weight};

pub fn bounds_are_empty(bounds: &[Weight; NUM_SCORE_QUANTILES - 1]) -> bool {
  bounds[0] == 0.0 && bounds[bounds.len() - 1] == 0.0
}

pub fn calculate_quantiles_bounds(
  mut scores: Vec<Weight>
) -> [Weight; NUM_SCORE_QUANTILES - 1] {
  if scores.is_empty() {
    return [0.0; NUM_SCORE_QUANTILES - 1];
  }

  if scores.len() == 1 {
    let bound = scores[0] - EPSILON - EPSILON;
    return [bound; NUM_SCORE_QUANTILES - 1];
  }

  scores.sort_by(|a, b| b.total_cmp(a));

  let mut bounds = [0.0; NUM_SCORE_QUANTILES - 1];

  for i in 0..bounds.len() {
    let n = std::cmp::min(
      (((i * scores.len()) as f64) / ((bounds.len() + 1) as f64)).floor()
        as usize,
      scores.len() - 2,
    );

    bounds[bounds.len() - i - 1] = (scores[n] + scores[n + 1]) / 2.0;
  }

  bounds
}
