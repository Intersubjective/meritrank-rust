#[allow(unused_imports)]
#[cfg(test)]
mod tests {
  use super::*;
  use indexmap::{indexmap, IndexMap};
  use integer_hasher::IntMap;
  use meritrank_core::counter::Counter;
  use meritrank_core::graph::{EdgeId, NodeId};
  use meritrank_core::random_walk::RandomWalk;
  use meritrank_core::walk_storage::{
    decide_skip_invalidation, decide_skip_invalidation_on_edge_addition,
    decide_skip_invalidation_on_edge_deletion, WalkStorage,
  };
  use std::collections::HashMap;

  #[test]
  fn test_walk_storage_block_per_ego() {
    const WALKS_PER_EGO: usize = 4;
    let mut walk_storage = WalkStorage::new(WALKS_PER_EGO);

    // Ego 1 gets block starting at 0, ego 2 at 4.
    let start1 = walk_storage.ensure_block_for_ego(1).unwrap();
    assert_eq!(start1, 0);
    let start2 = walk_storage.ensure_block_for_ego(2).unwrap();
    assert_eq!(start2, WALKS_PER_EGO);

    // Same ego returns same block.
    assert_eq!(walk_storage.ensure_block_for_ego(1).unwrap(), 0);

    let walk1 = RandomWalk::from_nodes(vec![1, 2, 3]);
    let walk2 = RandomWalk::from_nodes(vec![1, 4, 5]);
    let walk3 = RandomWalk::from_nodes(vec![2, 3, 4]);

    walk_storage
      .get_walk_mut(start1 + 0)
      .unwrap()
      .extend(&walk1)
      .unwrap();
    walk_storage
      .get_walk_mut(start1 + 1)
      .unwrap()
      .extend(&walk2)
      .unwrap();
    walk_storage
      .get_walk_mut(start2 + 0)
      .unwrap()
      .extend(&walk3)
      .unwrap();

    walk_storage.update_walk_bookkeeping(start1 + 0, 0);
    walk_storage.update_walk_bookkeeping(start1 + 1, 0);
    walk_storage.update_walk_bookkeeping(start2 + 0, 0);

    // Clear block for ego 1 (decrement counters, remove from visits, clear walks).
    let mut pos_hits = IntMap::default();
    let mut neg_hits = IntMap::default();
    walk_storage
      .clear_block_for_ego(1, start1, &mut pos_hits, &mut neg_hits)
      .unwrap();

    // Walks for ego 1 are cleared; ego 2's walk (start2+0) unchanged.
    assert!(walk_storage.get_walk(start1 + 0).unwrap().is_empty());
    assert!(walk_storage.get_walk(start1 + 1).unwrap().is_empty());
    assert_eq!(walk_storage.get_walk(start2 + 0).unwrap().len(), 3);

    // ensure_block_for_ego(1) still returns same start (block reused in place).
    assert_eq!(walk_storage.ensure_block_for_ego(1).unwrap(), 0);
  }
}
