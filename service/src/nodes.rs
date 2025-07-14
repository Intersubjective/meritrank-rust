use bincode::{Decode, Encode};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, Hash)]
pub enum NodeKind {
  User,
  Beacon,
  Comment,
  Opinion,
  PollVariant,
  Poll,
}

impl fmt::Display for NodeKind {
  fn fmt(
    &self,
    f: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    match self {
      NodeKind::User => write!(f, "User"),
      NodeKind::Beacon => write!(f, "Beacon"),
      NodeKind::Comment => write!(f, "Comment"),
      NodeKind::Opinion => write!(f, "Opinion"),
      NodeKind::PollVariant => write!(f, "PollVariant"),
      NodeKind::Poll => write!(f, "Poll"),
    }
  }
}

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

// pub const ALL_NODE_KINDS: [NodeKind; 6] = [
//   NodeKind::User,
//   NodeKind::Beacon,
//   NodeKind::Comment,
//   NodeKind::Opinion,
//   NodeKind::PollVariant,
//   NodeKind::Poll,
// ];
