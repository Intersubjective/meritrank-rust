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

    // lets write test for get_personal_hits(&self) -> &HashMap<NodeId, Counter>
    #[test]
    fn test_get_personal_hits() {
        let graph = MyGraph::new();
        let merit_rank = MeritRank::new(graph).unwrap();
        let result = merit_rank.get_personal_hits();
        assert!(result.is_empty());
    }
}
