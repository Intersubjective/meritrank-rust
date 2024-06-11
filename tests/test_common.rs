#[allow(unused_imports)]
#[cfg(test)]
mod tests {
  use super::*;
  use meritrank::common::sign;
  use meritrank::errors::MeritRankError;
  use meritrank::graph::{NodeId, EdgeId, Weight};
  use meritrank::random_walk::RandomWalk;
  use meritrank::walk::{WalkId, WalkIdGenerator};
  use meritrank::walk_storage::WalkStorage;

  use rand::rngs::StdRng;
  use rand::SeedableRng;

  #[test]
  fn test_walk_id_generator() {
    // let generator = WalkIdGenerator::new();
    let walk_id1 = WalkIdGenerator::new().get_id();
    let walk_id2 = WalkIdGenerator::new().get_id();
    assert_ne!(walk_id1, walk_id2);
  }

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
    let storage = WalkStorage::new();
    let walk = RandomWalk::from_nodes(vec![1, 2]);
    let pos = 0;
    let edge: EdgeId = (1, 2);
    let step_recalc_probability = 0.5;

    let rng_seed = 1234;

    // Create a deterministic random number generator
    let mut rng = StdRng::seed_from_u64(rng_seed);

    let (_may_skip, _new_pos) = storage.decide_skip_invalidation_on_edge_addition(
      &walk,           // walk: This is the walk that is being invalidated
      pos,  // pos: This is the position in the walk that is being invalidated
      edge, // edge: This is the edge that is being added
      step_recalc_probability, // The probability that the step will be recalculated
      Some(&mut rng), // rng: This is the random number generator
    );
  }
}
