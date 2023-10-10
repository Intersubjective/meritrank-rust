#[allow(unused_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use meritrank::{NodeId, RandomWalk, WalkId, Weight};

    use std::collections::HashMap;

    #[test]
    fn test_random_walk_new() {
        let random_walk = RandomWalk::new();
        assert!(random_walk.get_nodes().is_empty());
        assert_ne!(random_walk.get_walk_id(), WalkId::nil());
    }

    #[test]
    fn test_random_walk_from_nodes() {
        let nodes = vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)];
        let random_walk = RandomWalk::from_nodes(nodes.clone());
        assert_eq!(random_walk.get_nodes(), nodes.as_slice());
        assert_ne!(random_walk.get_walk_id(), WalkId::nil());
    }

    #[test]
    fn test_random_walk_add_node() {
        let mut random_walk = RandomWalk::new();
        random_walk._add_node(NodeId::Int(1));
        assert_eq!(random_walk.get_nodes(), &[NodeId::Int(1)]);
    }

    #[test]
    fn test_random_walk_get_nodes() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        assert_eq!(
            random_walk.get_nodes(),
            &[NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]
        );
    }

    #[test]
    fn test_random_walk_len() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        assert_eq!(random_walk.len(), 3);
    }

    #[test]
    fn test_random_walk_contains() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        assert!(random_walk.contains(&NodeId::Int(2)));
        assert!(!random_walk.contains(&NodeId::Int(4)));
    }

    #[test]
    fn test_random_walk_intersects_nodes() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        assert!(random_walk.intersects_nodes(&[NodeId::Int(2), NodeId::Int(4)]));
        assert!(!random_walk.intersects_nodes(&[NodeId::Int(4), NodeId::Int(5)]));
    }

    #[test]
    fn test_random_walk_get_nodes_mut() {
        let mut random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        random_walk._get_nodes_mut().push(NodeId::Int(4));
        assert_eq!(
            random_walk.get_nodes(),
            &[
                NodeId::Int(1),
                NodeId::Int(2),
                NodeId::Int(3),
                NodeId::Int(4)
            ]
        );
    }

    #[test]
    fn test_random_walk_first_node() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        assert_eq!(random_walk.first_node(), Some(NodeId::Int(1)));

        let random_walk = RandomWalk::new();
        assert_eq!(random_walk.first_node(), None);
    }

    #[test]
    fn test_random_walk_last_node() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        assert_eq!(random_walk.last_node(), Some(NodeId::Int(3)));

        let random_walk = RandomWalk::new();
        assert_eq!(random_walk.last_node(), None);
    }

    #[test]
    fn test_random_walk_get_walk_id() {
        let random_walk = RandomWalk::new();
        assert_ne!(random_walk.get_walk_id(), WalkId::nil());
    }

    #[test]
    fn test_random_walk_iter() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        let mut iter = random_walk.iter();
        assert_eq!(iter.next(), Some(&NodeId::Int(1)));
        assert_eq!(iter.next(), Some(&NodeId::Int(2)));
        assert_eq!(iter.next(), Some(&NodeId::Int(3)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_random_walk_push() {
        let mut random_walk = RandomWalk::new();
        random_walk.push(NodeId::Int(1));
        random_walk.push(NodeId::Int(2));
        assert_eq!(random_walk.get_nodes(), &[NodeId::Int(1), NodeId::Int(2)]);
    }

    #[test]
    fn test_random_walk_extend() {
        let mut random_walk = RandomWalk::from_nodes(vec![NodeId::Int(1)]);
        let new_segment = vec![NodeId::Int(2), NodeId::Int(3)];
        random_walk.extend(&new_segment);
        assert_eq!(
            random_walk.get_nodes(),
            &[NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]
        );
    }

    #[test]
    fn test_random_walk_split_from() {
        let mut random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        let split_segment = random_walk.split_from(1);
        assert_eq!(random_walk.get_nodes(), &[NodeId::Int(1)]);
        assert_eq!(split_segment.get_nodes(), &[NodeId::Int(2), NodeId::Int(3)]);
    }

    #[test]
    fn test_random_walk_into_iterator() {
        let random_walk =
            RandomWalk::from_nodes(vec![NodeId::Int(1), NodeId::Int(2), NodeId::Int(3)]);
        let mut iter = random_walk.into_iter();
        assert_eq!(iter.next(), Some(NodeId::Int(1)));
        assert_eq!(iter.next(), Some(NodeId::Int(2)));
        assert_eq!(iter.next(), Some(NodeId::Int(3)));
        assert_eq!(iter.next(), None);
    }

    // -- *** Random Walk: Calculate_penalties
    // TODO: Check if this test is correct

    #[test]
    fn test_random_walk_calculate_penalties() {
        let random_walk = RandomWalk::from_nodes(vec![
            NodeId::from(1),
            NodeId::from(2),
            NodeId::from(2),
            NodeId::from(4),
            NodeId::from(5),
            NodeId::from(6),
            NodeId::from(7),
        ]);

        let mut neg_weights: HashMap<NodeId, Weight> = HashMap::new();
        neg_weights.insert(NodeId::from(4), 1.0);
        neg_weights.insert(NodeId::from(6), 1.0);

        let penalties = random_walk.calculate_penalties(&neg_weights);

        assert_eq!(penalties.get(&NodeId::from(1)), Some(&2.0));
        assert_eq!(penalties.get(&NodeId::from(2)), Some(&2.0));
        assert_eq!(penalties.get(&NodeId::from(4)), Some(&2.0));
        assert_eq!(penalties.get(&NodeId::from(5)), Some(&1.0));
        assert_eq!(penalties.get(&NodeId::from(6)), Some(&1.0));
        assert_eq!(penalties.get(&NodeId::from(7)), None);
    }

    // -- *** Random Walk: Calculate_penalties ^^^
}
