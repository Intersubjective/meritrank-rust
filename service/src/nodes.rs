use crate::constants::*;
use crate::log::*;
use std::ops::{Index, IndexMut};

pub use meritrank_core::{NodeId, Weight};

// #[derive(Default)] removed
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NodeKind {
  // Unknown variant removed, #[default] removed
  User,
  Beacon,
  Comment,
  Opinion,
  PollVariant, // alt name is "Vote"
  Poll, 
}

// New function
pub fn node_kind_from_prefix(name: &str) -> Option<NodeKind> {
    if name.is_empty() {
        return None;
    }
    match name.chars().next() {
        Some('U') => Some(NodeKind::User),
        Some('B') => Some(NodeKind::Beacon),
        Some('C') => Some(NodeKind::Comment),
        Some('O') => Some(NodeKind::Opinion),
        Some('V') => Some(NodeKind::PollVariant),
        Some('P') => Some(NodeKind::Poll),
        _ => None,
    }
}

pub const ALL_NODE_KINDS: [NodeKind; 6] = [
  NodeKind::User,
  NodeKind::Beacon,
  NodeKind::Comment,
  NodeKind::Opinion,
  NodeKind::PollVariant,
  NodeKind::Poll,
];

// NeighborDirection enum REMOVED from here

#[derive(PartialEq, Clone, Default)]
pub struct NodeInfo {
  pub kind: Option<NodeKind>, // Changed to Option<NodeKind>
  pub name: String,
  // Bloom filter of nodes marked as seen by this node in the null context
  pub seen_nodes: Vec<u64>,
}

#[derive(PartialEq, Clone)]
pub struct ClusterGroupBounds {
  pub updated_sec: u64,
  pub bounds:      Vec<Weight>,
}

impl Default for ClusterGroupBounds {
  fn default() -> ClusterGroupBounds {
    ClusterGroupBounds {
      updated_sec: 0,
      bounds:      vec![0.0; NUM_SCORE_QUANTILES - 1],
    }
  }
}

#[derive(PartialEq, Clone, Default)]
pub struct ScoreClustersByKind {
  // pub unknown:  ClusterGroupBounds, // Field removed
  pub users:    ClusterGroupBounds,
  pub beacons:  ClusterGroupBounds,
  pub comments: ClusterGroupBounds,
  pub opinions: ClusterGroupBounds,
  pub poll_options:    ClusterGroupBounds,
  pub polls:    ClusterGroupBounds,
}

impl Index<NodeKind> for ScoreClustersByKind {
  type Output = ClusterGroupBounds;

  fn index(
    &self,
    index: NodeKind,
  ) -> &ClusterGroupBounds {
    match index {
      // NodeKind::Unknown arm removed
      NodeKind::User => &self.users,
      NodeKind::Beacon => &self.beacons,
      NodeKind::Comment => &self.comments,
      NodeKind::Opinion => &self.opinions,
      NodeKind::PollVariant => &self.poll_options,
      NodeKind::Poll => &self.polls,
    }
  }
}

impl IndexMut<NodeKind> for ScoreClustersByKind {
  fn index_mut(
    &mut self,
    index: NodeKind,
  ) -> &mut ClusterGroupBounds {
    match index {
      // NodeKind::Unknown arm removed
      NodeKind::User => &mut self.users,
      NodeKind::Beacon => &mut self.beacons,
      NodeKind::Comment => &mut self.comments,
      NodeKind::Opinion => &mut self.opinions,
      NodeKind::PollVariant => &mut self.poll_options,
      NodeKind::Poll => &mut self.polls,
    }
  }
}

// Removed: pub fn kind_from_name(name: &str) -> NodeKind { ... }
// Removed: pub fn kind_from_prefix(prefix: &str) -> Result<NodeKind, ()> { ... }

pub fn node_name_from_id(
  infos: &[NodeInfo],
  id: NodeId,
) -> String {
  match infos.get(id) {
    Some(x) => x.name.clone(),
    _ => {
      log_error!("Node does not exist: {}", id);
      "".to_string()
    },
  }
}

pub fn node_kind_from_id(
  infos: &[NodeInfo],
  id: NodeId,
) -> Option<NodeKind> { // Return type changed
  match infos.get(id) {
    Some(x) => x.kind, // This is already Option<NodeKind>
    _ => {
      log_error!("Node does not exist: {}", id);
      None // Fallback to None
    },
  }
}

pub fn nodes_by_kind(
  kind: NodeKind, // Parameter is a concrete NodeKind
  node_infos: &[NodeInfo],
) -> Vec<NodeId> {
  node_infos
    .iter()
    .enumerate()
  .filter(|(_, info)| info.kind == Some(kind)) // Compare with Some(kind)
    .map(|(id, _)| id)
    .collect()
}
