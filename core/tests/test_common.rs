#[allow(unused_imports)]
#[cfg(test)]
mod tests {
  use super::*;
  use meritrank_core::common::sign;
  use meritrank_core::errors::MeritRankError;
  use meritrank_core::graph::{EdgeId, NodeId, Weight};
  use meritrank_core::random_walk::RandomWalk;
  use meritrank_core::walk_storage::{
    decide_skip_invalidation, decide_skip_invalidation_on_edge_addition,
    decide_skip_invalidation_on_edge_deletion,
  };

  use rand::rngs::StdRng;
  use rand::SeedableRng;

  #[test]
  fn test_sign() {
    assert_eq!(sign(5), 1);
    assert_eq!(sign(-3.5), -1);
    assert_eq!(sign(0), 0);
    assert_eq!(sign(10.2), 1);
    assert_eq!(sign(-7), -1);
  }

  #[test]
  fn test_meritrank_error_display() {
    let error = MeritRankError::NodeDoesNotExist;
    assert_eq!(error.to_string(), "Node does not exist");

    let error = MeritRankError::SelfReferenceNotAllowed;
    assert_eq!(error.to_string(), "Self-reference is not allowed");

    let error = MeritRankError::RandomChoiceError;
    assert_eq!(error.to_string(), "Random choice error");

    let error = MeritRankError::InternalFatalError(None);
    assert_eq!(error.to_string(), "Internal fatal error");

    let error =
      MeritRankError::InternalFatalError(Some("rank::set_edge_ get_node_data(src) None"));
    assert_eq!(
      error.to_string(),
      "Internal fatal error: rank::set_edge_ get_node_data(src) None"
    );
  }

  #[test]
  fn test_decide_skip_invalidation() {
    let walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let edge: EdgeId = (2, 3);
    // (prob_pos_segment, prob_neg_segment); walk has no negative segment so only first is used
    let step_recalc_probability = Some((0.5, 0.5));
    let rng_seed = 1342; // Set the seed for the random number generator

    // Create a deterministic random number generator
    let mut rng = StdRng::seed_from_u64(rng_seed);

    // Test skipping invalidation on edge deletion
    let (may_skip, new_pos) =
      decide_skip_invalidation(&walk, 2, edge, None, Some(&mut rng)).unwrap();
    assert!(may_skip);
    assert_eq!(new_pos, 2);

    // Test skipping invalidation with step recalculation probability
    let (may_skip, new_pos) = decide_skip_invalidation(
      &walk,
      1,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    )
    .unwrap();
    assert!(may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation without skipping
    let (may_skip, new_pos) = decide_skip_invalidation(
      &walk,
      0,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    )
    .unwrap();
    assert!(!may_skip);
    assert_eq!(new_pos, 1);
  }

  #[test]
  fn test_walk_storage_decide_skip_invalidation_on_edge_deletion() {
    let walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let edge: EdgeId = (2, 3);

    // Test invalidation without skipping
    let (may_skip, new_pos) =
      decide_skip_invalidation_on_edge_deletion(&walk, 0, edge).unwrap();
    assert!(!may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation with skipping
    let (may_skip, new_pos) =
      decide_skip_invalidation_on_edge_deletion(&walk, 1, edge).unwrap();
    assert!(!may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation at the end of the walk
    let (may_skip, new_pos) =
      decide_skip_invalidation_on_edge_deletion(&walk, 2, edge).unwrap();
    assert!(may_skip);
    assert_eq!(new_pos, 2);
  }

  #[test]
  fn test_decide_skip_invalidation_on_edge_addition() {
    let walk = RandomWalk::from_nodes(vec![1, 2, 1, 3]);
    let edge: EdgeId = (1, 2);
    // (prob_pos_segment, prob_neg_segment); walk has no negative segment
    let (prob_pos, prob_neg) = (0.5, 0.5);
    let rng_seed = 1342; // Set the seed for the random number generator

    // Create a deterministic random number generator
    let mut rng = StdRng::seed_from_u64(rng_seed);

    // Test invalidation without skipping
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      0,
      edge,
      prob_pos,
      prob_neg,
      Some(&mut rng),
    )
    .unwrap();
    assert!(!may_skip);
    assert_eq!(new_pos, 2);

    // Test invalidation with skipping
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      2,
      edge,
      prob_pos,
      prob_neg,
      Some(&mut rng),
    )
    .unwrap();
    assert!(may_skip);
    assert_eq!(new_pos, 2);

    // Test invalidation with step recalculation
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      1,
      edge,
      prob_pos,
      prob_neg,
      Some(&mut rng),
    )
    .unwrap();
    assert_eq!(may_skip, false);
    assert_eq!(new_pos, 2);
  }

  /// Walk with negative segment: nodes [0..neg_start) positive, [neg_start..) negative.
  /// Negative new edge at a position in the negative segment must always be skipped
  /// (prob_neg_segment = 0 for negative edges).
  #[test]
  fn test_decide_skip_invalidation_on_edge_addition_negative_segment_negative_edge(
  ) {
    let mut walk = RandomWalk::from_nodes(vec![0, 1, 2, 3]); // 0->1->2->3
    walk.negative_segment_start = Some(2); // positions 2,3 are in negative segment
    let edge: EdgeId = (2, 99); // edge from node 2 (in negative segment)
    // Negative new edge: prob_neg_segment = 0, so we must never invalidate at pos 2
    let (prob_pos, prob_neg) = (0.5, 0.0);

    let mut rng = StdRng::seed_from_u64(12345);
    // Scan from pos 0: first occurrence of node 2 is at index 2 (in negative segment)
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      0,
      edge,
      prob_pos,
      prob_neg,
      Some(&mut rng),
    )
    .unwrap();
    // Must skip: at position 2 we use prob_neg = 0, so we never invalidate
    assert!(may_skip);
    assert_eq!(new_pos, 2);
  }

  /// Positive new edge at a position in the negative segment uses prob_neg_segment.
  #[test]
  fn test_decide_skip_invalidation_on_edge_addition_negative_segment_positive_edge(
  ) {
    let mut walk = RandomWalk::from_nodes(vec![0, 1, 2, 3]);
    walk.negative_segment_start = Some(2);
    let edge: EdgeId = (2, 99);
    // Positive new edge: prob_neg_segment > 0 so invalidation at pos 2 is possible
    let (prob_pos, prob_neg) = (0.0, 1.0); // 100% prob in negative segment

    let mut rng = StdRng::seed_from_u64(99999);
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      0,
      edge,
      prob_pos,
      prob_neg,
      Some(&mut rng),
    )
    .unwrap();
    // With prob_neg = 1.0 we always invalidate at the first occurrence (index 2)
    assert!(!may_skip);
    assert_eq!(new_pos, 2);
  }
}
