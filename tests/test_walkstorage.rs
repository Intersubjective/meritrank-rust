#[allow(unused_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;
    use meritrank::edge::EdgeId;
    use meritrank::node::{Node, NodeId};
    use meritrank::poswalk::PosWalk;
    use meritrank::random_walk::RandomWalk;
    use meritrank::walk_storage::WalkStorage;

    #[test]
    fn test_walk_storage_add_walk() {
        let mut walk_storage = WalkStorage::new();

        let walk = RandomWalk::from_nodes(vec![
            NodeId::from(1),
            NodeId::from(2),
            NodeId::from(3),
            NodeId::from(4),
        ]);
        let start_pos = 1;

        walk_storage.add_walk(walk.clone(), start_pos);

        let walk_storage_str = format!("{:?}", walk_storage);
        let expected_walks_str = format!(
            "WalkStorage {{ walks: {:?} }}",
            indexmap! {
                NodeId::from(2) => indexmap! {
                    walk.get_walk_id() => PosWalk::new(walk.clone(), 1),
                },
                NodeId::from(3) => indexmap! {
                    walk.get_walk_id() => PosWalk::new(walk.clone(), 2),
                },
                NodeId::from(4) => indexmap! {
                    walk.get_walk_id() => PosWalk::new(walk.clone(), 3),
                },
            }
        );

        assert_eq!(walk_storage_str, expected_walks_str);

        // TODO: Finish this test!

        // // assert_eq!(walk_storage.get_walks(), expected_walks);
        // assert_eq!(walk_storage.len(), expected_walks.len());
        // for (node, walks) in expected_walks {
        //     assert_eq!(walk_storage.get(&node), Some(&walks));
        // }
    }

    #[test]
    fn test_walk_storage_update_walk() {
        let mut walk_storage = WalkStorage::new();

        let old_walk =
            RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)]);
        let new_walk =
            RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(4), NodeId::from(5)]);

        walk_storage.add_walk(old_walk.clone(), 0);
        walk_storage.update_walk(new_walk.clone(), Some(old_walk.clone()));

        // Check that the old walk is removed

        let walk_storage_str = format!("{:?}", walk_storage);
        let expected_walks_str = format!(
            "WalkStorage {{ walks: {:?} }}",
            indexmap! {
                NodeId::from(1) => indexmap! {
                    new_walk.get_walk_id() => PosWalk::new(new_walk.clone(), 0),
                },
                NodeId::from(4) => indexmap! {
                    new_walk.get_walk_id() => PosWalk::new(new_walk.clone(), 1),
                },
                NodeId::from(5) => indexmap! {
                    new_walk.get_walk_id() => PosWalk::new(new_walk.clone(), 2),
                },
            }
        );

        println!("walk_storage_str: \n{}\n\n", walk_storage_str);
        println!("expected_walks_str: \n{}", expected_walks_str);

        assert_eq!(walk_storage_str, expected_walks_str);
    }

    #[test]
    fn test_walk_storage_implement_changes() {
        let mut walk_storage = WalkStorage::new();

        let old_walk =
            RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)]);
        let new_walk =
            RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(4), NodeId::from(5)]);

        let update_walk_vec = vec![(new_walk.clone(), old_walk.clone())];

        walk_storage.add_walk(old_walk.clone(), 0);
        walk_storage.implement_changes(update_walk_vec);

        // Check that the old walk is removed

        let walk_storage_str = format!("{:?}", walk_storage);
        let expected_walks_str = format!(
            "WalkStorage {{ walks: {:?} }}",
            indexmap! {
                NodeId::from(1) => indexmap! {
                    new_walk.get_walk_id() => PosWalk::new(new_walk.clone(), 0),
                },
                NodeId::from(4) => indexmap! {
                    new_walk.get_walk_id() => PosWalk::new(new_walk.clone(), 1),
                },
                NodeId::from(5) => indexmap! {
                    new_walk.get_walk_id() => PosWalk::new(new_walk.clone(), 2),
                },
            }
        );

        println!("walk_storage_str: \n{}\n", walk_storage_str);
        println!("expected_walks_str: \n{}", expected_walks_str);

        assert_eq!(walk_storage_str, expected_walks_str);
    }

    // #[test]
    // fn test_walk_storage_update_walk() {
    //     let mut walk_storage = WalkStorage::new();
    //
    //     let old_walk = RandomWalk::from_nodes(vec![
    //         NodeId::from(1),
    //         NodeId::from(2),
    //         NodeId::from(3),
    //     ]);
    //     let new_walk = RandomWalk::from_nodes(vec![
    //         NodeId::from(1),
    //         NodeId::from(4),
    //         NodeId::from(5),
    //     ]);
    //
    //     walk_storage.add_walk(old_walk.clone(), 0);
    //     walk_storage.update_walk(new_walk.clone(), Some(old_walk.clone()));
    //
    //     let mut expected_walks = WalkStorage::new();
    //     expected_walks.add_walk(new_walk.clone(), 0);
    //     expected_walks.add_walk(new_walk.clone(), 1);
    //     expected_walks.add_walk(new_walk.clone(), 2);
    //
    //     assert_eq!(format!("{:?}", walk_storage), format!("{:?}", expected_walks));
    // }

    #[test]
    fn test_walk_storage_get_walks_starting_from_node() {
        let mut walk_storage = WalkStorage::new();

        let walk1 = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)]);
        let walk2 = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(4), NodeId::from(5)]);

        walk_storage.add_walk(walk1.clone(), 0);
        walk_storage.add_walk(walk2.clone(), 0);

        let walks = walk_storage._get_walks_starting_from_node(NodeId::from(1));

        assert_eq!(walks.len(), 2);
        assert_eq!(walks[0].get_nodes(), walk1.get_nodes());
        assert_eq!(walks[1].get_nodes(), walk2.get_nodes());
    }

    #[test]
    fn test_walk_storage_drop_walks_from_node() {
        let mut walk_storage = WalkStorage::new();

        let walk1 = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)]);
        let walk2 = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(4), NodeId::from(5)]);
        let walk3 = RandomWalk::from_nodes(vec![NodeId::from(2), NodeId::from(3), NodeId::from(4)]);

        walk_storage.add_walk(walk1.clone(), 0);
        walk_storage.add_walk(walk2.clone(), 0);
        walk_storage.add_walk(walk3.clone(), 0);

        walk_storage.drop_walks_from_node(NodeId::from(1));

        let walk_storage_str = format!("{:?}", walk_storage);
        let expected_walks_str = format!(
            "WalkStorage {{ walks: {:?} }}",
            indexmap! {
                NodeId::from(2) => indexmap! {
                    walk3.get_walk_id() => PosWalk::new(walk3.clone(), 0),
                },
                NodeId::from(3) => indexmap! {
                    walk3.get_walk_id() => PosWalk::new(walk3.clone(), 1),
                },
                NodeId::from(4) => indexmap! {
                    walk3.get_walk_id() => PosWalk::new(walk3.clone(), 2),
                },
            }
        );

        assert_eq!(walk_storage_str, expected_walks_str);

        assert_eq!(walk_storage.get_walks().len(), 3);
        assert_eq!(
            walk_storage
                .get_walks()
                .get(&NodeId::from(1))
                .map(|m| format!("{:?}", m)),
            None.map(|()| "".to_string())
        );
        assert_eq!(walk_storage.get_walks()[&NodeId::from(2)].len(), 1);
        assert_eq!(walk_storage.get_walks()[&NodeId::from(3)].len(), 1);
        assert_eq!(walk_storage.get_walks()[&NodeId::from(4)].len(), 1);
    }

    #[test]
    fn test_walk_storage_get_walks_through_node() {
        let mut walk_storage = WalkStorage::new();

        let walk1 = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)]);
        let walk2 = RandomWalk::from_nodes(vec![NodeId::from(2), NodeId::from(4), NodeId::from(5)]);
        let walk3 = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(3), NodeId::from(4)]);

        walk_storage.add_walk(walk1.clone(), 0);
        walk_storage.add_walk(walk2.clone(), 0);
        walk_storage.add_walk(walk3.clone(), 0);

        let walks_filter_len = walk_storage
            .get_walks_through_node(NodeId::from(1), |pos_walk| pos_walk.get_walk().len() > 2);
        assert_eq!(walks_filter_len.len(), 2);
        assert_eq!(walks_filter_len[0].get_walk_id(), walk1.get_walk_id());
        assert_eq!(walks_filter_len[1].get_walk_id(), walk3.get_walk_id());

        let walks_filter_node = walk_storage.get_walks_through_node(NodeId::from(2), |pos_walk| {
            pos_walk.get_current_node() == NodeId::from(2)
                && pos_walk.get_walk().contains(&NodeId::from(2))
        });
        assert_eq!(walks_filter_node.len(), 2);
        assert_eq!(walks_filter_node[0].get_walk_id(), walk1.get_walk_id());
        assert_eq!(walks_filter_node[1].get_walk_id(), walk2.get_walk_id());

        let walks_no_match = walk_storage.get_walks_through_node(NodeId::from(3), |pos_walk| {
            pos_walk.get_current_node() == NodeId::from(5)
        });
        assert_eq!(walks_no_match.len(), 0);
    }

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_walk_storage_decide_skip_invalidation() {
        let walk = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)]);
        let edge: EdgeId = (NodeId::from(2), NodeId::from(3));
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
        let walk = RandomWalk::from_nodes(vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)]);
        let edge: EdgeId = (NodeId::from(2), NodeId::from(3));

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
            NodeId::from(1),
            NodeId::from(2),
            NodeId::from(1),
            NodeId::from(3),
        ]);
        let edge: EdgeId = (NodeId::from(1), NodeId::from(2));
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
