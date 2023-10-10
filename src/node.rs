use crate::MeritRankError;

// use uuid::Uuid;

// use crate::error::MeritRankError;
// use crate::graph::{MyGraph, MyDiGraph};
// use crate::node::{NodeId, Weight, Node};
// use crate::edge::EdgeId;
// use crate::random_walk::RandomWalk;
// use crate::counter::{Counter, CounterIterator};
// use crate::ranking::{PosWalk};
// use crate::walk::WalkIdGenerator;
// use crate::walk_storage::WalkStorage;
// use crate::constants::{ASSERT, OPTIMIZE_INVALIDATION};
// use crate::common::{sign};

#[derive(Hash, PartialOrd, Ord, Clone, Copy, PartialEq, Eq)]
pub enum NodeId {
    Int(i32),
    UInt(usize),
    None,
}

impl Default for NodeId {
    fn default() -> Self {
        NodeId::UInt(0)
    }
}

impl NodeId {
    /// Checks if the NodeId is equal to None.
    pub fn is_none(&self) -> bool {
        // matches!(self, NodeId::None)
        match self {
            NodeId::None => true,
            _ => false,
        }
    }

    /// Checks if the NodeId is not equal to None and not equal to the default value (0).
    pub fn is_some(&self) -> bool {
        // check if NodeId is not None and not default (== 0)
        // !self.is_none() && *self != NodeId::default()
        match self {
            NodeId::None => false,
            NodeId::Int(id) => *id != 0,
            NodeId::UInt(id) => *id != 0,
        }
    }
}

/// Represents a node in the MeritRank graph.
#[derive(Debug, Hash, Default, PartialOrd, Ord, Clone, PartialEq, Eq)]
pub struct Node {
    id: NodeId,
}

impl Node {
    /// Creates a new Node with the specified id.
    pub fn new(id: NodeId) -> Self {
        Node { id }
    }

    /// Returns the id of the node.
    pub fn get_id(&self) -> NodeId {
        self.id
    }
}

impl<T> From<T> for Node
where
    T: Into<NodeId>,
{
    /// Converts the value into a Node using the provided id.
    fn from(id: T) -> Self {
        Node { id: id.into() }
    }
}

impl From<i32> for NodeId {
    /// Converts the i32 value into a NodeId::Int variant.
    fn from(id: i32) -> Self {
        NodeId::Int(id)
    }
}

impl From<usize> for NodeId {
    /// Converts the usize value into a NodeId::UInt variant.
    fn from(id: usize) -> Self {
        NodeId::UInt(id)
    }
}

/// The weight type used in the MeritRank graph.
pub type Weight = f64;

impl From<NodeId> for Weight {
    /// Converts the NodeId into a Weight.
    fn from(id: NodeId) -> Self {
        match id {
            NodeId::Int(id) => id as f64,
            NodeId::UInt(id) => id as f64,
            _ => 0.0,
        }
    }
}

impl From<Weight> for NodeId {
    /// Converts the Weight into a NodeId.
    fn from(id: Weight) -> Self {
        match id {
            id if id < 0.0 => NodeId::Int(id as i32),
            id if id >= 0.0 => NodeId::UInt(id as usize),
            _ => NodeId::default(),
        }
    }
}

use std::str::FromStr;

impl FromStr for NodeId {
    type Err = MeritRankError;

    /// Parses a NodeId from a string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<i32>() {
            Ok(id) => Ok(NodeId::Int(id)),
            Err(_) => match s.parse::<usize>() {
                Ok(id) => Ok(NodeId::UInt(id)),
                Err(_) => Err(MeritRankError::NodeIdParseError),
            },
        }
    }
}
