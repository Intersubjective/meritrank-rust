use indexmap::IndexMap;
use integer_hasher::BuildIntHasher;

use crate::errors::MeritRankError;
use crate::RandomWalk;
use log::error;
use rand::distributions::{Distribution, WeightedIndex};
use rand::{thread_rng, Rng};

type IntIndexMap<K, V> = IndexMap<K, V, BuildIntHasher<K>>;

pub type NodeId = usize;
pub type Weight = f64;
pub type EdgeId = (NodeId, NodeId);

#[derive(Debug, Clone, Default)]
pub struct NodeData {
    // Negative weights are stored as abs values, to simplify calculations
    pub pos_edges: IntIndexMap<NodeId, Weight>,
    pub neg_edges: IntIndexMap<NodeId, Weight>,

    // The sum of positive edges is often used for normalization,
    // so it is efficient to cache it.
    pub pos_sum: Weight,
    pub neg_sum: Weight,
    abs_distr_cache: Option<WeightedIndex<Weight>>,
    pos_distr_cache: Option<WeightedIndex<Weight>>,
}

impl NodeData {
    // Return a random neighbor and whether it's from positive or negative edges
    pub fn random_neighbor(&mut self, positive_only: bool) -> Option<(NodeId, bool)> {
        if positive_only {
            if self.pos_edges.is_empty() {
                return None;
            }

            if self.pos_distr_cache.is_none() {
                // Build and cache the distribution for positive edges
                let weights: Vec<Weight> = self.pos_edges.values().copied().collect();
                let wi = WeightedIndex::new(weights).unwrap();
                self.pos_distr_cache = Some(wi);
            }

            // Use the cached distribution
            let cache = self.pos_distr_cache.as_ref().unwrap();
            let index = cache.sample(&mut thread_rng());
            let node_id = *self.pos_edges.keys().nth(index).unwrap();
            Some((node_id, true))
        } else {
            if self.pos_edges.is_empty() && self.neg_edges.is_empty() {
                return None;
            }

            if self.abs_distr_cache.is_none() {
                // Build and cache the combined distribution of positive and negative edges
                let combined_weights: Vec<Weight> = self
                    .pos_edges
                    .values()
                    .chain(self.neg_edges.values())
                    .map(|&w| w)
                    .collect();

                let wi = WeightedIndex::new(combined_weights).unwrap();
                self.abs_distr_cache = Some(wi);
            }

            // Use the cached distribution
            let cache = self.abs_distr_cache.as_ref().unwrap();
            let index = cache.sample(&mut thread_rng());
            self.get_node_at_index(index)
        }
    }

    // Helper method to get the node at a given index from combined edges
    fn get_node_at_index(&self, index: usize) -> Option<(NodeId, bool)> {
        let pos_len = self.pos_edges.len();

        if index < pos_len {
            let node_id = *self.pos_edges.keys().nth(index).unwrap();
            Some((node_id, true))
        } else {
            let neg_index = index - pos_len;
            let node_id = *self.neg_edges.keys().nth(neg_index).unwrap();
            Some((node_id, false))
        }
    }
    pub fn abs_sum(&self) -> Weight {
        self.pos_sum + self.neg_sum
    }
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<NodeData>,
}

impl Graph {
    pub fn new() -> Self {
        Graph { nodes: Vec::new() }
    }
    pub fn get_new_nodeid(&mut self) -> NodeId {
        self.nodes.push(NodeData::default());
        self.nodes.len() - 1
    }

    /// Checks if a node with the given `NodeId` exists in the graph.
    pub fn contains_node(&self, node_id: NodeId) -> bool {
        // Check if the given NodeId exists in the nodes mapping
        self.nodes.get(node_id).is_some()
    }

    pub fn set_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        weight: Weight,
    ) -> Result<(), MeritRankError> {
        if !self.contains_node(from) || !self.contains_node(to) {
            return Err(MeritRankError::NodeNotFound);
        }
        if from == to {
            error!("Trying to add self-reference edge to node {}", from);
            return Err(MeritRankError::SelfReferenceNotAllowed);
        }
        if self.edge_weight(from, to)?.is_some() {
            self.remove_edge(from, to).unwrap();
        }

        let node = self
            .nodes
            .get_mut(from)
            .ok_or(MeritRankError::NodeNotFound)?;
        match weight {
            0.0 => {
                return Err(MeritRankError::ZeroWeightEncountered);
            }
            w if w > 0.0 => {
                node.pos_edges.insert(to, weight);
                node.pos_sum += weight;
                node.abs_distr_cache = None;
                node.pos_distr_cache = None;
            }
            _ => {
                node.neg_edges.insert(to, weight.abs());
                node.neg_sum += weight.abs();
                node.abs_distr_cache = None;
            }
        }
        Ok(())
    }

    pub fn get_node_data(&self, node_id: NodeId) -> Option<&NodeData> {
        self.nodes.get(node_id)
    }
    pub fn get_node_data_mut(&mut self, node_id: NodeId) -> Option<&mut NodeData> {
        self.nodes.get_mut(node_id)
    }

    /// Removes the edge between the two given nodes from the graph.
    pub fn remove_edge(&mut self, from: NodeId, to: NodeId) -> Result<Weight, MeritRankError> {
        let node = self
            .nodes
            .get_mut(from)
            .ok_or(MeritRankError::NodeNotFound)?;
        // This is slightly inefficient. More efficient would be to only try removing pos,
        // and get to neg only if pos_weight is None. We keep it to check the invariant of
        // not having both pos and neg weights for an edge simultaneously.
        let pos_weight = node.pos_edges.swap_remove(&to);
        let neg_weight = node.neg_edges.swap_remove(&to);

        // Both pos and neg weights should never be present at the same time.
        assert!(!(pos_weight.is_some() && neg_weight.is_some()));
        node.abs_distr_cache = None;
        if pos_weight.is_some() {
            node.pos_distr_cache = None;
        }
        // We have to clamp the sum to zero to avoid negative sums,
        // because floating-point arithmetic is not perfectly associative.
        if let Some(weight) = pos_weight {
            node.pos_sum -= weight;
            if node.pos_sum < 0.0 {
                node.pos_sum = 0.0;
            }
        } else if let Some(weight) = neg_weight {
            node.neg_sum -= weight;
            if node.neg_sum < 0.0 {
                node.neg_sum = 0.0;
            }
        }

        Ok(if let Some(weight) = pos_weight {
            weight
        } else if let Some(weight) = neg_weight {
            -weight
        } else {
            panic!("Edge not found")
        })
    }

    pub fn edge_weight(&self, from: NodeId, to: NodeId) -> Result<Option<Weight>, MeritRankError> {
        let node = self.nodes.get(from).ok_or(MeritRankError::NodeNotFound)?;
        if !self.contains_node(to) {
            return Err(MeritRankError::NodeNotFound);
        }
        Ok(if let Some(weight) = node.pos_edges.get(&to) {
            Some(*weight)
        } else if let Some(weight) = node.neg_edges.get(&to) {
            Some(-*weight)
        } else {
            None
        })
    }

    pub fn generate_walk_segment(
        &mut self,
        start_node: NodeId,
        alpha: f64,
        positive_only: bool,
    ) -> RandomWalk {
        let mut node = start_node;
        let mut segment = RandomWalk::new();
        let mut rng = thread_rng();

        let mut negative_continuation_mode = false;
        // When this variable becomes true, it means that a walk has encountered a negative edge,
        // followed it, and now the walk is in the "negative continuation mode", meaning we
        // will only follow positive edges, from now on, storing the index of its
        // start in "negative_segment_start" variable. Later, we will u "punish" the nodes that
        // were encountered in the negative continuation mode:
        //  +  +  -     +  +  +
        // A->B->C->(-D)->E->F->G
        // P  P  P    N   N  N  N

        loop {
            let node_data = self.get_node_data_mut(node).unwrap();
            if rng.gen::<f64>() > alpha {
                break;
            }
            if let Some((next_step, step_is_positive)) =
                node_data.random_neighbor(negative_continuation_mode || positive_only)
            {
                segment.push(next_step, step_is_positive);
                if !step_is_positive {
                    assert!(!negative_continuation_mode);
                    negative_continuation_mode = true;
                }
                node = next_step;
            } else {
                // Dead-end encountered
                break;
            }
        }
        segment
    }

    pub fn continue_walk(&mut self, walk: &mut RandomWalk, alpha: f64) {
        // If the original walk is already in "negative mode",
        // we should restrict segment generation to positive edges
        let positive_only = walk.negative_segment_start.is_some();
        let start_node = walk.last_node().unwrap();
        let new_segment = self.generate_walk_segment(start_node, alpha, positive_only);

        // Borrow mutable `walk` again for `extend`
        walk.extend(&new_segment);
    }
    pub fn extend_walk_in_case_of_edge_deletion(&mut self, walk: &mut RandomWalk) {
        // Borrow mutable `walk` from `self.walks`
        // No force_first_step, so this is "edge deletion mode"
        //
        // Force addition of the first step by extending the original walk with it.
        // Make sure that positive/negative subsegment marking is taken into account.
        // Forcing the step is neccessary in case of edge deletion in optimized mode:
        // we simulate the situation when the actual edge that was taken in the first case
        // was an edge different from the deleted one. Therefore, we should not apply
        // alpha-based stop to it, as this would lead to bias.
        let src_node = walk.last_node().unwrap();
        let node_data = self.get_node_data_mut(src_node).unwrap();
        let adding_to_negative_subsegment = walk.negative_segment_start.is_some();
        if let Some((forced_step, step_is_positive)) =
            node_data.random_neighbor(adding_to_negative_subsegment)
        {
            walk.push(forced_step, step_is_positive);
        }
    }
}
