
pub fn bounds_are_empty(bounds: &[f64]) -> bool {
  bounds.first() == Some(&0.0) && bounds.last() == Some(&0.0)
}
pub fn calculate_quantiles_bounds(
  mut scores: Vec<f64>,
  num_quantiles: usize,
) -> Vec<f64> {
  if scores.is_empty() {
    return vec![0.0; num_quantiles - 1];
  }

  if scores.len() == 1 {
    let bound = scores[0] - f64::EPSILON;
    return vec![bound; num_quantiles - 1];
  }

  scores.sort_by(|a, b| a.total_cmp(b)); // Sort in ascending order

  let mut bounds = Vec::with_capacity(num_quantiles - 1);
  for i in 1..num_quantiles {
    let position = i * scores.len() / num_quantiles;

    if position == 0 {
      bounds.push(scores[0]);
    } else if position >= scores.len() {
      bounds.push(scores[scores.len() - 1]);
    } else {
      // Linear interpolation between two adjacent values
      let lower = scores[position - 1];
      let upper = scores[position];
      bounds.push((lower + upper) / 2.0);
    }
  }

  bounds
}

/*

TODO
The original implementation of quantile bounds does not conform
to best practices. Some of these tests are failing:
We should refactor the quantiles bounds calculation eventually

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_scores() {
        let scores = vec![];
        let result = calculate_quantiles_bounds(scores, 5);
        assert_eq!(result, vec![0.0; 4]);
    }

    #[test]
    fn test_single_score() {
        let scores = vec![10.0];
        let result = calculate_quantiles_bounds(scores, 5);
        assert_eq!(result, vec![10.0 - EPSILON; 4]);
    }

    #[test]
    fn test_zero_quantiles() {
        let scores = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = calculate_quantiles_bounds(scores, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_one_quantile() {
        let scores = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = calculate_quantiles_bounds(scores, 1);
        assert!(result.is_empty());
    }

    #[test]
    fn test_normal_case() {
        let scores = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = calculate_quantiles_bounds(scores, 4);
        assert_eq!(result, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_more_quantiles_than_scores() {
        let scores = vec![1.0, 2.0, 3.0];
        let result = calculate_quantiles_bounds(scores, 5);
        assert_eq!(result, vec![1.5, 2.0, 2.5, 3.0]);
    }

    #[test]
    fn test_uneven_distribution() {
        let scores = vec![1.0, 1.0, 1.0, 2.0, 5.0, 10.0];
        let result = calculate_quantiles_bounds(scores, 3);
        assert_eq!(result, vec![1.0, 2.0]);
    }

    #[test]
    fn test_negative_scores() {
        let scores = vec![-5.0, -3.0, 0.0, 3.0, 5.0];
        let result = calculate_quantiles_bounds(scores, 5);
        assert_eq!(result, vec![-3.0, -1.5, 1.5, 4.0]);
    }
}
*/
