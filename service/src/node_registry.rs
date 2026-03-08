use crate::data::*;
use crate::utils::log::*;

use meritrank_core::{MeritRank, NodeId};

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
  pub id:    NodeId,
  pub name:  NodeName,
  pub kind:  NodeKind,
  pub owner: Option<NodeId>,
}

#[derive(Clone)]
pub struct NodeRegistry {
  pub name_to_id:   HashMap<NodeName, NodeId>,
  pub id_to_info:   Vec<NodeInfo>,
  pub kind_to_ids:  HashMap<NodeKind, Vec<NodeId>>,
  pub next_id:      NodeId,
}

impl NodeRegistry {
  pub fn new() -> Self {
    Self {
      name_to_id:  HashMap::new(),
      id_to_info:  Vec::new(),
      kind_to_ids: HashMap::new(),
      next_id:     0,
    }
  }

  pub fn register(
    &mut self,
    mr: &mut MeritRank,
    name: NodeName,
    kind: NodeKind,
  ) -> NodeId {
    if let Some(&id) = self.name_to_id.get(&name) {
      return id;
    }

    let id = self.next_id;
    self.next_id += 1;

    if id != mr.get_new_nodeid() {
      log_error!("Got unexpected node id.");
    }

    let info = NodeInfo {
      id,
      name: name.clone(),
      kind,
      owner: None,
    };
    self.name_to_id.insert(name, id);
    self.id_to_info.push(info);
    self.kind_to_ids.entry(kind).or_default().push(id);

    id
  }

  pub fn register_with_owner(
    &mut self,
    mr: &mut MeritRank,
    name: NodeName,
    kind: NodeKind,
    owner: NodeId,
  ) -> NodeId {
    if let Some(&id) = self.name_to_id.get(&name) {
      return id;
    }

    let id = self.next_id;
    self.next_id += 1;

    if id != mr.get_new_nodeid() {
      log_error!("Got unexpected node id.");
    }

    let info = NodeInfo {
      id,
      name: name.clone(),
      kind,
      owner: Some(owner),
    };
    self.name_to_id.insert(name, id);
    self.id_to_info.push(info);
    self.kind_to_ids.entry(kind).or_default().push(id);

    id
  }

  pub fn get_by_id(
    &self,
    id: NodeId,
  ) -> Option<&NodeInfo> {
    self.id_to_info.get(id)
  }

  pub fn get_by_name(
    &self,
    name: &str,
  ) -> Option<&NodeInfo> {
    self
      .name_to_id
      .get(name)
      .and_then(|&id| self.id_to_info.get(id))
  }

  pub fn nodes_by_kind(
    &self,
    kind: NodeKind,
  ) -> &[NodeId] {
    self.kind_to_ids.get(&kind).map(Vec::as_slice).unwrap_or(&[])
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
