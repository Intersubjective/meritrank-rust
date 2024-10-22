#[allow(unused_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::{indexmap, IndexMap};
    use integer_hasher::IntMap;
    use meritrank_core::graph::{EdgeId, NodeId};
    use meritrank_core::random_walk::RandomWalk;
    use meritrank_core::walk_storage::{
        decide_skip_invalidation, decide_skip_invalidation_on_edge_addition,
        decide_skip_invalidation_on_edge_deletion, WalkStorage,
    };
    use std::collections::HashMap;

    #[test]
    fn test_walk_storage_drop_walks_from_node() {
        let mut walk_storage = WalkStorage::new();

        let walk1 = RandomWalk::from_nodes(vec![1, 2, 3]);
        let walk2 = RandomWalk::from_nodes(vec![1, 4, 5]);
        let walk3 = RandomWalk::from_nodes(vec![2, 3, 4]);

        let walkid1 = walk_storage.get_next_free_walkid();
        let walkid2 = walk_storage.get_next_free_walkid();
        let walkid3 = walk_storage.get_next_free_walkid();

        walk_storage.get_walk_mut(walkid1).unwrap().extend(&walk1);
        walk_storage.get_walk_mut(walkid2).unwrap().extend(&walk2);
        walk_storage.get_walk_mut(walkid3).unwrap().extend(&walk3);

        walk_storage.update_walk_bookkeeping(walkid1, 0);
        walk_storage.update_walk_bookkeeping(walkid2, 0);
        walk_storage.update_walk_bookkeeping(walkid3, 0);

        walk_storage.drop_walks_from_node(1);

        let walk_storage_str = format!("{:?}", walk_storage);
        let expected_visits_str = format!(
            "WalkStorage {{ walks: {:?} }}",
            vec![
                IndexMap::default(),
                IndexMap::default(),
                indexmap! {
                    walkid3 => 0,
                },
                indexmap! {
                    walkid3 => 1,
                },
                indexmap! {
                    walkid3 => 2,
                },
                IndexMap::default(),
            ]
        );

        assert_eq!(walk_storage_str, expected_visits_str);

        assert_eq!(walk_storage.get_walks().len(), 6);
        assert_eq!(walk_storage.get_walks()[2].len(), 1);
        assert_eq!(walk_storage.get_walks()[3].len(), 1);
        assert_eq!(walk_storage.get_walks()[4].len(), 1);

        // Make sure that the walks are reused
        assert_eq!(walk_storage.get_next_free_walkid(), 0);
    }
}
