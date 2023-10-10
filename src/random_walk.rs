use std::collections::HashMap;

use crate::node::{NodeId, Weight};
use crate::walk::{WalkId, WalkIdGenerator};

/// Represents a random walk through a graph.
#[derive(Clone)]
pub struct RandomWalk {
    nodes: Vec<NodeId>,
    walk_id: WalkId,
}

impl RandomWalk {
    /// Creates a new empty `RandomWalk` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk};
    ///
    /// let random_walk = RandomWalk::new();
    /// ```
    pub fn new() -> Self {
        let walk_id = WalkIdGenerator::new().get_id();
        RandomWalk {
            nodes: Vec::new(),
            walk_id,
        }
    }

    /// Creates a `RandomWalk` instance from a vector of node IDs.
    ///
    /// # Arguments
    ///
    /// * `nodes` - A vector of node IDs representing the nodes in the walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let nodes = vec![NodeId::UInt( 1), NodeId::UInt(2), NodeId::UInt(3)];
    /// let random_walk = RandomWalk::from_nodes(nodes);
    /// ```
    pub fn from_nodes(nodes: Vec<NodeId>) -> Self {
        let walk_id = WalkIdGenerator::new().get_id();
        RandomWalk { nodes, walk_id }
    }

    /// Adds a node to the random walk.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to add to the walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let mut random_walk = RandomWalk::new();
    /// random_walk._add_node(NodeId::UInt(1));
    /// ```
    pub fn _add_node(&mut self, node_id: NodeId) {
        self.nodes.push(node_id);
    }

    /// Returns a reference to the vector of node IDs in the random walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk};
    ///
    /// let random_walk = RandomWalk::new();
    /// let nodes = random_walk.get_nodes();
    /// ```
    pub fn get_nodes(&self) -> &[NodeId] {
        &self.nodes
    }

    /// Returns the number of nodes in the random walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let random_walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3)]);
    /// let len = random_walk.len();
    /// ```
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Checks if the random walk contains the given node ID.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to check for.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let random_walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3)]);
    /// let contains = random_walk.contains(&NodeId::UInt(2));
    /// ```
    pub fn contains(&self, node_id: &NodeId) -> bool {
        self.nodes.contains(node_id)
    }

    /// Checks if the random walk intersects with the given nodes.
    ///
    /// # Arguments
    ///
    /// * `nodes` - A slice of nodes to check for intersection.
    ///
    /// # Returns
    ///
    /// `true` if the random walk intersects with any of the given nodes, `false` otherwise.
    pub fn intersects_nodes(&self, nodes: &[NodeId]) -> bool {
        nodes.iter().any(|&node| self.contains(&node))
    }

    /// Returns a mutable reference to the vector of node IDs in the random walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk};
    ///
    /// let mut random_walk = RandomWalk::new();
    /// let nodes = random_walk._get_nodes_mut();
    /// ```
    pub fn _get_nodes_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.nodes
    }

    /// Returns an option containing the ID of the first node in the random walk, if it exists.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let random_walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3)]);
    /// let first_node = random_walk.first_node();
    /// ```
    pub fn first_node(&self) -> Option<NodeId> {
        self.nodes.first().copied()
    }

    /// Returns an option containing the ID of the last node in the random walk, if it exists.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let random_walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3)]);
    /// let last_node = random_walk.last_node();
    /// ```
    pub fn last_node(&self) -> Option<NodeId> {
        self.nodes.last().copied()
    }

    /// Returns the ID of the random walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk};
    ///
    /// let random_walk = RandomWalk::new();
    /// let walk_id = random_walk.get_walk_id();
    /// ```
    pub fn get_walk_id(&self) -> WalkId {
        self.walk_id
    }

    /// Returns an iterator over the node IDs in the random walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let random_walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3)]);
    /// for node_id in random_walk.iter() {
    ///     println!("Node ID: {:?}", node_id);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes.iter()
    }

    /// Pushes a node ID to the end of the random walk.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to push to the walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let mut random_walk = RandomWalk::new();
    /// random_walk.push(NodeId::UInt(1));
    /// ```
    pub fn push(&mut self, node_id: NodeId) {
        self.nodes.push(node_id);
    }

    /// Extends the random walk with a new segment of node IDs.
    ///
    /// # Arguments
    ///
    /// * `new_segment` - A slice of node IDs representing the new segment to add to the walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let mut random_walk = RandomWalk::new();
    /// let new_segment = vec![NodeId::UInt(2), NodeId::UInt(3)];
    /// random_walk.extend(&new_segment);
    /// ```
    pub fn extend(&mut self, new_segment: &[NodeId]) {
        self.nodes.extend_from_slice(new_segment);
    }

    /// Splits the random walk from the specified position and returns the split-off segment.
    ///
    /// # Arguments
    ///
    /// * `pos` - The position from which to split the walk.
    ///
    /// # Returns
    ///
    /// * `RandomWalk` - The split-off segment of the walk.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let mut random_walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3), NodeId::UInt(4), NodeId::UInt(5)]);
    /// let split_segment = random_walk.split_from(2);
    /// ```
    pub fn split_from(&mut self, pos: usize) -> RandomWalk {
        let split_segment = self.nodes.split_off(pos);
        RandomWalk::from_nodes(split_segment)
    }
}

impl IntoIterator for RandomWalk {
    type Item = NodeId;
    type IntoIter = std::vec::IntoIter<NodeId>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

/// Penalty calculation logic:
/// 1. The penalty is accumulated by walking backwards from the last node in the segment.
/// 2. If a node is encountered in the walk more than once, its penalty is updated to the highest current accumulated penalty.
/// 3. If a penalty-inducing node (called a "neg" for short) is encountered more than once, its effect is not accumulated.
///
/// In a sense, every neg in the walk produces a "tag", so each node in the walk leading up to a given neg is "tagged" by it,
/// and then each "tagged" node is penalized according to the weight of the "tags" associated with the negs.
///
/// Example:
/// nodes D and F both are negs of weight 1
/// node B is repeated twice in positions 2-3
///         ◄─+tag F────────┐
///         ◄─+tag D──┐     │
///        ┌──────────┴─────┴────┐
///        │ A  B  B  D  E  F  G │
///        └─────────────────────┘
/// Resulting penalties for the nodes:
/// node     A  -  B  D  E  F  G
/// "tags"   DF -  DF DF F  F
/// penalty  2  -  2  2  1  1  0
impl RandomWalk {
    /// Calculates penalties for nodes based on negative weights.
    ///
    /// Parameters:
    /// - `neg_weights`: A reference to the map of negative weights for nodes.
    ///
    /// Returns:
    /// - A map containing the penalties for nodes.
    pub fn calculate_penalties(
        &self,
        neg_weights: &HashMap<NodeId, Weight>,
    ) -> HashMap<NodeId, Weight> {
        let mut penalties: HashMap<NodeId, Weight> = HashMap::new();
        let mut negs = neg_weights.clone();
        let mut accumulated_penalty = 0.0;

        for &step in self.nodes.iter().rev() {
            if let Some(penalty) = negs.remove(&step) {
                accumulated_penalty += penalty;
            }

            if accumulated_penalty != 0.0 {
                penalties.insert(step, accumulated_penalty);
            }
        }

        penalties
    }
}
