use crate::constants::*;
use crate::log::*;
use std::ops::{Index, IndexMut};

pub use meritrank_core::{NodeId, Weight};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum NodeKind {
  #[default]
  Unknown,  // TODO: remove this completely and instead propagate errors
  User,
  Beacon,
  Comment,
  Opinion,
  PollOption, // alt name is "Vote"
  Poll, 
}

pub const ALL_NODE_KINDS: [NodeKind; 6] = [
  NodeKind::User,
  NodeKind::Beacon,
  NodeKind::Comment,
  NodeKind::Opinion,
  NodeKind::PollOption,
  NodeKind::Poll,
];

// NeighborDirection enum REMOVED from here

#[derive(PartialEq, Clone, Default)]
pub struct NodeInfo {
  pub kind: NodeKind,
  pub name: String,

  // Bloom filter of nodes marked as seen by this node in the null context
  pub seen_nodes: Vec<u64>,
}

#[derive(PartialEq, Clone)]
pub struct ClusterGroupBounds {
  pub updated_sec: u64,
  pub bounds:      [Weight; NUM_SCORE_QUANTILES - 1],
}

impl Default for ClusterGroupBounds {
  fn default() -> ClusterGroupBounds {
    ClusterGroupBounds {
      updated_sec: 0,
      bounds:      [0.0; NUM_SCORE_QUANTILES - 1],
    }
  }
}

#[derive(PartialEq, Clone, Default)]
pub struct ScoreClustersByKind {
  pub unknown:  ClusterGroupBounds,
  pub users:    ClusterGroupBounds,
  pub beacons:  ClusterGroupBounds,
  pub comments: ClusterGroupBounds,
  pub opinions: ClusterGroupBounds,
  pub poll_options:    ClusterGroupBounds, // TODO: remove
  pub polls:    ClusterGroupBounds, // TODO: remove
}

impl Index<NodeKind> for ScoreClustersByKind {
  type Output = ClusterGroupBounds;

  fn index(
    &self,
    index: NodeKind,
  ) -> &ClusterGroupBounds {
    match index {
      NodeKind::Unknown => &self.unknown,
      NodeKind::User => &self.users,
      NodeKind::Beacon => &self.beacons,
      NodeKind::Comment => &self.comments,
      NodeKind::Opinion => &self.opinions,
      NodeKind::PollOption => &self.poll_options, // TODO: remove
      NodeKind::Poll => &self.polls, // TODO: remove
    }
  }
}

impl IndexMut<NodeKind> for ScoreClustersByKind {
  fn index_mut(
    &mut self,
    index: NodeKind,
  ) -> &mut ClusterGroupBounds {
    match index {
      NodeKind::Unknown => &mut self.unknown,
      NodeKind::User => &mut self.users,
      NodeKind::Beacon => &mut self.beacons,
      NodeKind::Comment => &mut self.comments,
      NodeKind::Opinion => &mut self.opinions,
      NodeKind::PollOption => &mut self.poll_options, // New case
      NodeKind::Poll => &mut self.polls, // New case
    }
  }
}

pub fn kind_from_name(name: &str) -> NodeKind {
  log_trace!("{:?}", name);

  match name.chars().next() {
    Some('U') => NodeKind::User,
    Some('B') => NodeKind::Beacon,
    Some('C') => NodeKind::Comment,
    Some('O') => NodeKind::Opinion,
    Some('V') => NodeKind::PollOption,
    Some('P') => NodeKind::Poll,
    _ => NodeKind::Unknown,
  }
}

pub fn kind_from_prefix(prefix: &str) -> Result<NodeKind, ()> {
  match prefix {
    "" => Ok(NodeKind::Unknown),
    "U" => Ok(NodeKind::User),
    "B" => Ok(NodeKind::Beacon),
    "C" => Ok(NodeKind::Comment),
    "O" => Ok(NodeKind::Opinion),
    "V" => Ok(NodeKind::PollOption), // "V" stands for "Vote"
    "P" => Ok(NodeKind::Poll),
    _ => Err(()),
  }
}

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
) -> NodeKind {
  match infos.get(id) {
    Some(x) => x.kind,
    _ => {
      log_error!("Node does not exist: {}", id);
      NodeKind::Unknown
    },
  }
}

pub fn nodes_by_kind(
  kind: NodeKind,
  node_infos: &[NodeInfo],
) -> Vec<NodeId> {
  node_infos
    .iter()
    .enumerate()
    .filter(|(_, info)| info.kind == kind)
    .map(|(id, _)| id)
    .collect()
}
