#[allow(unused_imports)]
#[cfg(test)]
mod tests {
  use super::*;
  use meritrank_core::{NodeId, RandomWalk, WalkId, Weight};

  use integer_hasher::IntMap;
  use std::collections::HashMap;
  use std::mem::offset_of;

  #[test]
  fn test_random_walk_new() {
    let random_walk = RandomWalk::new();
    assert!(random_walk.get_nodes().is_empty());
  }

  #[test]
  fn test_random_walk_from_nodes() {
    let nodes = vec![1, 2, 3];
    let random_walk = RandomWalk::from_nodes(nodes.clone());
    assert_eq!(random_walk.get_nodes(), nodes.as_slice());
  }

  #[test]
  fn test_random_walk_add_node() {
    let mut random_walk = RandomWalk::new();
    random_walk._add_node(1);
    assert_eq!(random_walk.get_nodes(), &[1,]);
  }

  #[test]
  fn test_random_walk_get_nodes() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    assert_eq!(random_walk.get_nodes(), &[1, 2, 3,]);
  }

  #[test]
  fn test_random_walk_len() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    assert_eq!(random_walk.len(), 3);
  }

  #[test]
  fn test_random_walk_contains() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    assert!(random_walk.contains(&2));
    assert!(!random_walk.contains(&4));
  }

  #[test]
  fn test_random_walk_intersects_nodes() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    assert!(random_walk.intersects_nodes(&[2, 4,]));
    assert!(!random_walk.intersects_nodes(&[4, 5,]));
  }

  #[test]
  fn test_random_walk_get_nodes_mut() {
    let mut random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    random_walk._get_nodes_mut().push(4);
    assert_eq!(random_walk.get_nodes(), &[1, 2, 3, 4,]);
  }

  #[test]
  fn test_random_walk_first_node() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    assert_eq!(random_walk.first_node(), Some(1));

    let random_walk = RandomWalk::new();
    assert_eq!(random_walk.first_node(), None);
  }

  #[test]
  fn test_random_walk_last_node() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    assert_eq!(random_walk.last_node(), Some(3));

    let random_walk = RandomWalk::new();
    assert_eq!(random_walk.last_node(), None);
  }

  #[test]
  fn test_random_walk_iter() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let mut iter = random_walk.iter();
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next(), Some(&3));
    assert_eq!(iter.next(), None);
  }

  #[test]
  fn test_random_walk_push() {
    let mut random_walk = RandomWalk::new();
    random_walk.push(1, true);
    random_walk.push(2, false);
    random_walk.push(3, true);
    assert_eq!(random_walk.negative_segment_start.unwrap(), 1);
    assert_eq!(random_walk.get_nodes(), &[1, 2, 3]);
  }
  #[test]
  fn test_start_with_negative_step() {
    let mut random_walk = RandomWalk::new();
    random_walk.push(10, false); // Step 0: Negative
    assert_eq!(random_walk.negative_segment_start.unwrap(), 0);
    assert_eq!(random_walk.get_nodes(), &[10]);
  }

  #[test]
  fn test_multiple_positive_steps_after_negative() {
    let mut random_walk = RandomWalk::new();
    random_walk.push(1, false); // Step 0: Negative
    random_walk.push(2, true); // Step 1: Positive
    random_walk.push(3, true); // Step 2: Positive

    assert_eq!(random_walk.negative_segment_start.unwrap(), 0);
    assert_eq!(random_walk.get_nodes(), &[1, 2, 3]);
  }

  //  FIXME
  // #[test]
  // #[should_panic(expected = "Expected `negative_segment_start` to be `None`")]
  // fn test_no_overlapping_negative_segments() {
  //   let mut random_walk = RandomWalk::new();
  //   random_walk.push(1, false); // Step 0: Negative
  //   random_walk.push(2, false); // Step 1: Negative (should panic)
  // }

  #[test]
  fn test_random_walk_extend_pos_to_neg() {
    let mut random_walk = RandomWalk::from_nodes(vec![1]);
    let mut new_segment = RandomWalk::from_nodes(vec![2, 3]);
    new_segment.negative_segment_start = Some(0);
    random_walk.extend(&new_segment);
    assert_eq!(random_walk.get_nodes(), &[1, 2, 3,]);
    assert_eq!(random_walk.negative_segment_start.unwrap(), 1)
  }

  #[test]
  fn test_random_walk_extend_net_to_pos() {
    let mut random_walk = RandomWalk::from_nodes(vec![1, 2]);
    let new_segment = RandomWalk::from_nodes(vec![3, 4]);
    random_walk.negative_segment_start = Some(1);
    random_walk.extend(&new_segment);
    assert_eq!(random_walk.get_nodes(), &[1, 2, 3, 4]);
    assert_eq!(random_walk.negative_segment_start.unwrap(), 1)
  }

  #[test]
  fn test_random_walk_split_from() {
    let mut random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let split_segment = random_walk.split_from(1);
    assert_eq!(random_walk.get_nodes(), &[1]);
    assert_eq!(split_segment.get_nodes(), &[2, 3,]);
  }

  #[test]
  fn test_random_walk_into_iterator() {
    let random_walk = RandomWalk::from_nodes(vec![1, 2, 3]);
    let mut iter = random_walk.into_iter();
    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.next(), Some(3));
    assert_eq!(iter.next(), None);
  }
}
