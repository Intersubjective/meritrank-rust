#[allow(unused_imports)]
#[cfg(test)]
mod tests {
    use super::*;
    use meritrank::common::sign;
    use meritrank::edge::EdgeId;
    use meritrank::errors::MeritRankError;
    use meritrank::node::{Node, NodeId, Weight};
    use meritrank::random_walk::RandomWalk;
    use meritrank::walk::{WalkId, WalkIdGenerator};
    use meritrank::walk_storage::WalkStorage;

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_is_none() {
        let node_id = NodeId::None;
        assert!(node_id.is_none());
        assert!(!node_id.is_some());

        let node_id = NodeId::Int(0);
        assert!(!node_id.is_none());
        assert!(!node_id.is_some());
    }

    #[test]
    fn test_is_some() {
        let node_id = NodeId::None;
        assert!(!node_id.is_some());

        let node_id = NodeId::Int(0);
        assert!(!node_id.is_some());

        let node_id = NodeId::Int(1);
        assert!(node_id.is_some());

        let node_id = NodeId::UInt(0);
        assert!(!node_id.is_some());

        let node_id = NodeId::UInt(1);
        assert!(node_id.is_some());
    }

    #[test]
    fn test_node_id_is_none() {
        let none_id = NodeId::None;
        let int_id = NodeId::Int(0);
        let uint_id = NodeId::UInt(0);

        assert!(none_id.is_none());
        assert!(!int_id.is_none());
        assert!(!uint_id.is_none());
    }

    #[test]
    fn test_node_id_is_some() {
        let none_id = NodeId::None;
        let default_id = NodeId::default();
        let int_id = NodeId::Int(10);
        let uint_id = NodeId::UInt(5);

        assert!(!none_id.is_some());
        assert!(!default_id.is_some());
        assert!(int_id.is_some());
        assert!(uint_id.is_some());
    }

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
    fn test_merit_rank_error_display() {
        let error = MeritRankError::NodeDoesNotExist;
        assert_eq!(error.to_string(), "NodeDoesNotExist");

        let error = MeritRankError::SelfReferenceNotAllowed;
        assert_eq!(error.to_string(), "SelfReferenceNotAllowed");

        let error = MeritRankError::RandomChoiceError;
        assert_eq!(error.to_string(), "RandomChoiceError");
    }

    #[test]
    fn test_node_new() {
        let node = Node::new(NodeId::Int(42));
        assert_eq!(node.get_id(), NodeId::Int(42));

        let node = Node::new(NodeId::UInt(123));
        assert_eq!(node.get_id(), NodeId::UInt(123));
    }

    #[test]
    fn test_node_from() {
        let node: Node = NodeId::Int(42).into();
        assert_eq!(node.get_id(), NodeId::Int(42));

        let node: Node = NodeId::UInt(123).into();
        assert_eq!(node.get_id(), NodeId::UInt(123));
    }

    #[test]
    fn test_node_id_from() {
        let node_id: NodeId = 42.into();
        assert_eq!(node_id, NodeId::Int(42));

        let node_id: NodeId = 123usize.into();
        assert_eq!(node_id, NodeId::UInt(123));
    }

    #[test]
    fn test_node_id_from_weight() {
        let node_id: NodeId = Weight::from(42.5).into();
        assert_eq!(node_id, NodeId::UInt(42));

        let node_id: NodeId = Weight::from(123.0).into();
        assert_eq!(node_id, NodeId::UInt(123));
    }

    #[test]
    fn test_weight_from_node_id() {
        let weight: Weight = NodeId::Int(42).into();
        assert_eq!(weight, 42.0);

        let weight: Weight = NodeId::UInt(123).into();
        assert_eq!(weight, 123.0);
    }

    #[test]
    fn test_decide_skip_invalidation() {
        let storage = WalkStorage::new();
        let walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2)]);
        let pos = 0;
        let edge: EdgeId = (NodeId::UInt(1), NodeId::UInt(2));
        let step_recalc_probability = 0.5;

        let rng_seed = 1234;

        // Create a deterministic random number generator
        let mut rng = StdRng::seed_from_u64(rng_seed);

        let (_may_skip, _new_pos) = storage.decide_skip_invalidation_on_edge_addition(
            &walk,                   // walk: This is the walk that is being invalidated
            pos,  // pos: This is the position in the walk that is being invalidated
            edge, // edge: This is the edge that is being added
            step_recalc_probability, // The probability that the step will be recalculated
            Some(&mut rng), // rng: This is the random number generator
        );
    }
}
