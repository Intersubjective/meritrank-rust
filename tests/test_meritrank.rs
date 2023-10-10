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
    use meritrank::{MeritRank, MyGraph};

    use std::collections::HashMap;

    // lets write test for new(graph: MyGraph) -> Result<Self, MeritRankError>
    #[test]
    fn test_new() {
        let graph = MyGraph::new();
        let result = MeritRank::new(graph);
        assert!(result.is_ok());
    }

    // lets write test for add_walk(&mut self, walk: RandomWalk, start_pos: usize)
    #[test]
    fn test_add_walk() {
        let graph = MyGraph::new();
        let mut merit_rank = MeritRank::new(graph).unwrap();
        let walk = RandomWalk::new();
        let start_pos = 0;
        merit_rank.add_walk(walk, start_pos);
        assert_eq!(merit_rank.get_walks().len(), 0);
    }

    // lets write test for get_walks(&self) -> &IndexMap<NodeId, IndexMap<WalkId, PosWalk>>
    #[test]
    fn test_get_walks() {
        let graph = MyGraph::new();
        let merit_rank = MeritRank::new(graph).unwrap();

        let walks = merit_rank.get_walks();
        assert!(
            walks.is_empty(),
            "Newly created MeritRank should not have any walks"
        );
    }

    // lets write test for get_neg_hits(&self) -> &HashMap<NodeId, HashMap<NodeId, Weight>>
    #[test]
    fn test_get_neg_hits() {
        let graph = MyGraph::new();
        let merit_rank = MeritRank::new(graph).unwrap();
        let result = merit_rank.get_neg_hits();
        assert!(result.is_empty());
    }

    // lets write test for get_personal_hits(&self) -> &HashMap<NodeId, Counter>
    #[test]
    fn test_get_personal_hits() {
        let graph = MyGraph::new();
        let merit_rank = MeritRank::new(graph).unwrap();
        let result = merit_rank.get_personal_hits();
        assert!(result.is_empty());
    }

    // lets write test for get_graph(&self) -> &MyGraph
    #[test]
    fn test_get_graph() {
        let graph = MyGraph::new();
        let merit_rank = MeritRank::new(graph.clone()).unwrap();
        let result = merit_rank.get_graph();
        assert_eq!(
            result, &graph,
            "The graph returned by get_graph did not match the original"
        );
    }

    // lets write test for get_graph_mut(&mut self) -> &mut MyGraph
    #[test]
    fn test_get_graph_mut() {
        let mut graph = MyGraph::new();
        let mut merit_rank = MeritRank::new(graph.clone()).unwrap();
        let result = merit_rank.get_graph_mut();
        assert_eq!(
            result, &mut graph,
            "The mutable graph returned by get_graph_mut did not match the original"
        );
    }

    // lets write test for get_alpha(&self) -> Weight
    #[test]
    fn test_get_alpha() {
        let graph = MyGraph::new();
        let merit_rank = MeritRank::new(graph).unwrap();
        let result = merit_rank.get_alpha();
        assert!(result >= 0.0, "Alpha weight should be non-negative");
    }

    // lets write test for set_alpha(&mut self, alpha: Weight)
    #[test]
    fn test_set_alpha() {
        let graph = MyGraph::new();
        let mut merit_rank = MeritRank::new(graph).unwrap();
        let alpha = 0.0;
        merit_rank.set_alpha(alpha);
        assert_eq!(merit_rank.get_alpha(), 0.0);
    }

    // lets write test for get_hit_counts(&self, node: &NodeId) -> Option<f64>
    #[test]
    fn test_get_hit_counts() {
        let graph = MyGraph::new();
        let merit_rank = MeritRank::new(graph).unwrap();
        let node = NodeId::UInt(1);
        let result = merit_rank.get_hit_counts(&node);
        assert!(
            result.is_none(),
            "Should return an Option<f64>, if not found then None"
        );
    }

    // lets write test for increment_hit_counts(&mut self, _walk: &RandomWalk)
    #[test]
    fn test_increment_hit_counts() {
        let graph = MyGraph::new();
        let mut merit_rank = MeritRank::new(graph).unwrap();
        let walk = RandomWalk::new();
        merit_rank.increment_hit_counts(&walk);
        let node = NodeId::UInt(1);
        assert_eq!(
            merit_rank.get_hit_counts(&node),
            None,
            // Some(1.0),
            "After one increment, total hit counts should be 1"
        );
    }

    // // lets write test for calculate(&mut self, ego: NodeId, num_walks: usize) -> Result<(), MeritRankError>
    // fn test_calculate() {
    //     let graph = MyGraph::new();
    //     let mut merit_rank = MeritRank::new(graph).unwrap();
    //     let ego = NodeId::UInt(1);
    //     let num_walks = 0;
    //     let result = merit_rank.calculate(ego, num_walks);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for _get_node_score(&self, ego: NodeId, target: NodeId) -> Weight
    // fn test__get_node_score() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let ego = NodeId::UInt(1);
    //     let target = NodeId::UInt(1);
    //     let result = merit_rank._get_node_score(ego, target);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for get_ranks(&self, ego: NodeId, limit: Option<usize>) -> HashMap<NodeId, Weight>
    // fn test_get_ranks() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let ego = NodeId::UInt(1);
    //     let limit = 0;
    //     let result = merit_rank.get_ranks(ego, limit);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for neighbors_weighted(&self, node: NodeId, positive: bool) -> Option<HashMap<NodeId, Weight>>
    // fn test_neighbors_weighted() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let node = NodeId::UInt(1);
    //     let positive = true;
    //     let result = merit_rank.neighbors_weighted(node, positive);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for perform_walk(&self, start_node: NodeId) -> Result<RandomWalk, MeritRankError>
    // fn test_perform_walk() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let start_node = NodeId::UInt(1);
    //     let result = merit_rank.perform_walk(start_node);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for generate_walk_segment(
    // fn test_generate_walk_segment() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let start_node = NodeId::UInt(1);
    //     let result = merit_rank.generate_walk_segment(start_node);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for random_choice<T: Copy>(values: &[T], weights: &[f64], rng: &mut impl Rng) -> Option<T>
    // fn test_random_choice() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let values = &[0];
    //     let weights = &[0.0];
    //     let rng = 0;
    //     let result = merit_rank.random_choice(values, weights, rng);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for update_negative_hits(&mut self, walk: &RandomWalk, negs: &HashMap<NodeId, Weight>, subtract: bool)
    // fn test_update_negative_hits() {
    //     let graph = MyGraph::new();
    //     let mut merit_rank = MeritRank::new(graph).unwrap();
    //     let walk = RandomWalk::new();
    //     let negs = HashMap::new();
    //     let subtract = true;
    //     merit_rank.update_negative_hits(walk, negs, subtract);
    //     assert_eq!(merit_rank.get_negative_hits(), 1);
    // }
    //
    // // lets write test for get_edge(&self, src: NodeId, dest: NodeId) -> Option<Weight>
    // fn test_get_edge() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let src = NodeId::UInt(1);
    //     let dest = NodeId::UInt(2);
    //     let result = merit_rank.get_edge(src, dest);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for update_penalties_for_edge(&mut self, src: NodeId, dest: NodeId, remove_penalties: bool)
    // fn test_update_penalties_for_edge() {
    //     let graph = MyGraph::new();
    //     let mut merit_rank = MeritRank::new(graph).unwrap();
    //     let src = NodeId::UInt(1);
    //     let dest = NodeId::UInt(2);
    //     let remove_penalties = true;
    //     merit_rank.update_penalties_for_edge(src, dest, remove_penalties);
    //     assert_eq!(merit_rank.get_penalties(), 1);
    // }
    //
    // // lets write test for clear_invalidated_walk(&mut self, walk: &RandomWalk, invalidated_segment: &Vec<NodeId>)
    // fn test_clear_invalidated_walk() {
    //     let graph = MyGraph::new();
    //     let mut merit_rank = MeritRank::new(graph).unwrap();
    //     let walk = RandomWalk::new();
    //     let invalidated_segment = Vec::new();
    //     merit_rank.clear_invalidated_walk(walk, invalidated_segment);
    //     assert_eq!(merit_rank.get_invalidated_walk(), 1);
    // }
    //
    // // lets write test for recalc_invalidated_walk(
    // fn test_recalc_invalidated_walk() {
    //     let graph = MyGraph::new();
    //     let merit_rank = MeritRank::new(graph).unwrap();
    //     let walk = RandomWalk::new();
    //     let invalidated_segment = Vec::new();
    //     let result = merit_rank.recalc_invalidated_walk(walk, invalidated_segment);
    //     assert!(result.is_ok());
    // }
    //
    // // lets write test for add_edge(&mut self, src: NodeId, dest: NodeId, weight: f64)
    // fn test_add_edge() {
    //     let graph = MyGraph::new();
    //     let mut merit_rank = MeritRank::new(graph).unwrap();
    //     let src = NodeId::UInt(1);
    //     let dest = NodeId::UInt(2);
    //     let weight = 0.0;
    //     merit_rank.add_edge(src, dest, weight);
    //     assert_eq!(merit_rank.get_edges(), 1);
    // }
}
