#[allow(unused_imports)]
#[cfg(test)]
mod tests {
  use super::*;
  use meritrank_core::common::sign;
  use meritrank_core::errors::MeritRankError;
  use meritrank_core::graph::{NodeId, EdgeId, Weight};
  use meritrank_core::random_walk::RandomWalk;
  use meritrank_core::walk_storage::{decide_skip_invalidation, decide_skip_invalidation_on_edge_addition, decide_skip_invalidation_on_edge_deletion};

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
  }


  #[test]
  fn test_decide_skip_invalidation() {
    let walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let edge: EdgeId = (2, 3);
    let step_recalc_probability = Some(0.5);
    let rng_seed = 1342; // Set the seed for the random number generator

    // Create a deterministic random number generator
    let mut rng = StdRng::seed_from_u64(rng_seed);

    // Test skipping invalidation on edge deletion
    let (may_skip, new_pos) =
        decide_skip_invalidation(&walk, 2, edge, None, Some(&mut rng));
    assert!(may_skip);
    assert_eq!(new_pos, 2);

    // Test skipping invalidation with step recalculation probability
    let (may_skip, new_pos) = decide_skip_invalidation(
      &walk,
      1,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    assert!(may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation without skipping
    let (may_skip, new_pos) = decide_skip_invalidation(
      &walk,
      0,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    assert!(!may_skip);
    assert_eq!(new_pos, 1);
  }

  #[test]
  fn test_walk_storage_decide_skip_invalidation_on_edge_deletion() {
    let walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let edge: EdgeId = (2, 3);

    // Test invalidation without skipping
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_deletion(&walk, 0, edge);
    assert!(!may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation with skipping
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_deletion(&walk, 1, edge);
    assert!(!may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation at the end of the walk
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_deletion(&walk, 2, edge);
    assert!(may_skip);
    assert_eq!(new_pos, 2);
  }

  #[test]
  fn test_decide_skip_invalidation_on_edge_addition() {
    use rand::random;

    let walk = RandomWalk::from_nodes(vec![
      1,
      2,
      1,
      3,
    ]);
    let edge: EdgeId = (1, 2);
    let step_recalc_probability = 0.5;
    let rng_seed = 1342; // Set the seed for the random number generator

    // Create a deterministic random number generator
    let mut rng = StdRng::seed_from_u64(rng_seed);

    // Test invalidation without skipping
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      0,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    assert!(!may_skip);
    assert_eq!(new_pos, 2);

    // Test invalidation with skipping
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      2,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    assert!(may_skip);
    assert_eq!(new_pos, 2);

    // Test invalidation with step recalculation
    let (may_skip, new_pos) = decide_skip_invalidation_on_edge_addition(
      &walk,
      1,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    // let should_skip = random::<Weight>() > step_recalc_probability;
    assert_eq!(may_skip, false);
    assert_eq!(new_pos, 2);
  }
}
