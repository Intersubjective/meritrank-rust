#[allow(unused_imports)]
#[cfg(test)]
mod tests {
  use super::*;
  use indexmap::indexmap;
  use meritrank::graph::{NodeId, EdgeId};
  use meritrank::random_walk::RandomWalk;
  use meritrank::walk_storage::WalkStorage;

  /*
  #[test]
  fn test_walk_storage_add_walk() {
    let mut walk_storage = WalkStorage::new();

    let walk = RandomWalk::from_nodes(vec![
      1, 2, 3, 4,
    ]);
    let start_pos = 1;

    let new_walk_id = walk_storage.add_walk(walk.clone());
    walk_storage.add_walk_to_bookkeeping(new_walk_id, start_pos);

    let walk_storage_str = format!("{:?}", walk_storage);
    let expected_walks_str = format!(
      "WalkStorage {{ walks: {:?} }}",
      indexmap! {
        2 => indexmap! {
          walk.get_walk_id() => PosWalk::new(walk.clone(), 1),
        },
        3 => indexmap! {
          walk.get_walk_id() => PosWalk::new(walk.clone(), 2),
        },
        4 => indexmap! {
          walk.get_walk_id() => PosWalk::new(walk.clone(), 3),
        },
      }
    );

    assert_eq!(walk_storage_str, expected_walks_str);

    // TODO: Finish this test!

    // // assert_eq!(walk_storage.get_walks(), expected_walks);
    // assert_eq!(walk_storage.len(), expected_walks.len());
    // for (node, walks) in expected_walks {
    //   assert_eq!(walk_storage.get(&node), Some(&walks));
    // }
  }

   */


  #[test]
  fn test_walk_storage_drop_walks_from_node() {
    let mut walk_storage = WalkStorage::new();

    let walk1 = RandomWalk::from_nodes(vec![ 1, 2, 3, ]);
    let walk2 = RandomWalk::from_nodes(vec![ 1, 4, 5, ]);
    let walk3 = RandomWalk::from_nodes(vec![ 2, 3, 4, ]);

    let walkid1 = walk_storage.get_next_free_walkid();
    let walkid2 = walk_storage.get_next_free_walkid();
    let walkid3 = walk_storage.get_next_free_walkid();

    walk_storage.get_walk_mut(walkid1).unwrap().extend(walk1.get_nodes());
    walk_storage.get_walk_mut(walkid2).unwrap().extend(walk2.get_nodes());
    walk_storage.get_walk_mut(walkid3).unwrap().extend(walk3.get_nodes());

    walk_storage.add_walk_to_bookkeeping(walkid1, 0);
    walk_storage.add_walk_to_bookkeeping(walkid2, 0);
    walk_storage.add_walk_to_bookkeeping(walkid3, 0);

    walk_storage.drop_walks_from_node(1);

    let walk_storage_str = format!("{:?}", walk_storage);
    let expected_visits_str = format!(
      "WalkStorage {{ walks: {:?} }}",
      indexmap! {
        2 => indexmap! {
          walkid3 =>  0,
        },
        3 => indexmap! {
          walkid3 => 1,
        },
        4 => indexmap! {
          walkid3 => 2,
        },
      }
    );

    assert_eq!(walk_storage_str, expected_visits_str);

    assert_eq!(walk_storage.get_walks().len(), 3);
    assert_eq!(
      walk_storage
        .get_walks()
        .get(&1)
        .map(|m| format!("{:?}", m)),
      None.map(|()| "".to_string())
    );
    assert_eq!(walk_storage.get_walks()[&2].len(), 1);
    assert_eq!(walk_storage.get_walks()[&3].len(), 1);
    assert_eq!(walk_storage.get_walks()[&4].len(), 1);

    // Make sure that the walks are reused
    assert_eq!(walk_storage.get_next_free_walkid(), 0);
  }


  /*
 #[test]
fn test_walk_storage_get_walks_through_node() {
    let mut walk_storage = WalkStorage::new();

    let walk1 = RandomWalk::from_nodes(vec![1, 2, 3]);
    let walk2 = RandomWalk::from_nodes(vec![2, 4, 5]);
    let walk3 = RandomWalk::from_nodes(vec![1, 3, 4]);

    let walk1_id = walk_storage.add_walk(walk1.clone());
    walk_storage.add_walk_to_bookkeeping(walk1_id, 0);

    let walk2_id = walk_storage.add_walk(walk2.clone());
    walk_storage.add_walk_to_bookkeeping(walk2_id, 0);

    let walk3_id = walk_storage.add_walk(walk3.clone());
    walk_storage.add_walk_to_bookkeeping(walk3_id, 0);

    let walks_filter_len = walk_storage
        .get_walks_through_node_filtered(1, |pos_walk| pos_walk.get_walk().len() > 2);
    assert_eq!(walks_filter_len.len(), 2);
    assert!(walks_filter_len.iter().any(|pw| pw.get_walk_id() == walk1_id));
    assert!(walks_filter_len.iter().any(|pw| pw.get_walk_id() == walk3_id));

    let walks_filter_node = walk_storage.get_walks_through_node_filtered(2, |pos_walk| {
        pos_walk.get_current_node() == 2
            && pos_walk.get_walk().contains(&2)
    });
    assert_eq!(walks_filter_node.len(), 2);
    assert!(walks_filter_node.iter().any(|pw| pw.get_walk_id() == walk1_id));
    assert!(walks_filter_node.iter().any(|pw| pw.get_walk_id() == walk2_id));

    let walks_no_match = walk_storage.get_walks_through_node_filtered(3, |pos_walk| {
        pos_walk.get_current_node() == 5
    });
    assert_eq!(walks_no_match.len(), 0);
}

   */



  use rand::rngs::StdRng;
  use rand::SeedableRng;

  #[test]
  fn test_walk_storage_decide_skip_invalidation() {
    let walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let edge: EdgeId = (2, 3);
    let step_recalc_probability = 0.5;
    let rng_seed = 1342; // Set the seed for the random number generator

    // Create a deterministic random number generator
    let mut rng = StdRng::seed_from_u64(rng_seed);

    // Test skipping invalidation on edge deletion
    let storage = WalkStorage::new();
    let (may_skip, new_pos) =
      storage.decide_skip_invalidation(&walk, 2, edge, 0.0, Some(&mut rng));
    assert!(may_skip);
    assert_eq!(new_pos, 2);

    // Test skipping invalidation with step recalculation probability
    let storage = WalkStorage::new();
    let (may_skip, new_pos) = storage.decide_skip_invalidation(
      &walk,
      1,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    assert!(may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation without skipping
    let storage = WalkStorage::new();
    let (may_skip, new_pos) = storage.decide_skip_invalidation(
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
    let storage = WalkStorage::new();
    let walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let edge: EdgeId = (2, 3);

    // Test invalidation without skipping
    let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_deletion(&walk, 0, edge);
    assert!(!may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation with skipping
    let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_deletion(&walk, 1, edge);
    assert!(!may_skip);
    assert_eq!(new_pos, 1);

    // Test invalidation at the end of the walk
    let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_deletion(&walk, 2, edge);
    assert!(may_skip);
    assert_eq!(new_pos, 2);
  }

  #[test]
  fn test_walk_storage_decide_skip_invalidation_on_edge_addition() {
    use rand::random;

    let storage = WalkStorage::new();
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
    let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_addition(
      &walk,
      0,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    assert!(!may_skip);
    assert_eq!(new_pos, 2);

    // Test invalidation with skipping
    let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_addition(
      &walk,
      2,
      edge,
      step_recalc_probability,
      Some(&mut rng),
    );
    assert!(may_skip);
    assert_eq!(new_pos, 2);

    // Test invalidation with step recalculation
    let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_addition(
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
