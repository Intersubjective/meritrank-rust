// use crate::error::MeritRankError;
// use crate::graph::{MyGraph, MyDiGraph};
use crate::node::NodeId;
// use crate::edge::EdgeId;
use crate::random_walk::RandomWalk;
// use crate::counter::{Counter, CounterIterator};
// use crate::ranking::{PosWalk};
// use crate::walk::WalkIdGenerator;
// use crate::walk_storage::WalkStorage;
// use crate::constants::{ASSERT, OPTIMIZE_INVALIDATION};
// use crate::common::{sign};

/// Represents a positional random walk.
pub struct PosWalk {
    walk: RandomWalk,
    pos: usize,
}

#[allow(dead_code)]
impl PosWalk {
    /// Creates a new `PosWalk` with the given `RandomWalk` and position.
    ///
    /// Parameters:
    /// - `walk`: The `RandomWalk` associated with the `PosWalk`.
    /// - `pos`: The current position within the random walk.
    pub fn new(walk: RandomWalk, pos: usize) -> Self {
        PosWalk { walk, pos }
    }

    /// Returns a reference to the `RandomWalk` associated with the `PosWalk`.
    pub fn get_walk(&self) -> &RandomWalk {
        &self.walk
    }

    /// Returns a mutable reference to the `RandomWalk` associated with the `PosWalk`.
    pub fn get_walk_mut(&mut self) -> &mut RandomWalk {
        &mut self.walk
    }

    /// Returns the current position within the random walk.
    pub fn get_pos(&self) -> usize {
        self.pos
    }

    /// Sets the current position within the random walk.
    ///
    /// Parameters:
    /// - `pos`: The new position to set.
    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn set_walk(&mut self, walk: RandomWalk) {
        self.walk = walk;
    }

    /// Returns the ID of the current node in the random walk.
    ///
    /// Panics:
    /// - If the current position is out of bounds of the random walk.
    pub fn get_current_node(&self) -> NodeId {
        assert!(
            self.pos < self.walk.len(),
            "Current position is out of bounds."
        );
        *self.walk.get_nodes().get(self.pos).unwrap()
    }
}
