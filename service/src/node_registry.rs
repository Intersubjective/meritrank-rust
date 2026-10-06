use crate::data::*;
use crate::utils::log::*;

use meritrank_core::{MeritRank, NodeId};

use std::collections::HashMap;

/// A node of the graph. There is one class of nodes (D14): no kinds, no owners.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
  pub id:   NodeId,
  pub name: NodeName,
}

#[derive(Clone)]
pub struct NodeRegistry {
  pub name_to_id: HashMap<NodeName, NodeId>,
  pub id_to_info: Vec<NodeInfo>,
  pub next_id:    NodeId,
}

impl NodeRegistry {
  pub fn new() -> Self {
    Self {
      name_to_id: HashMap::new(),
      id_to_info: Vec::new(),
      next_id:    0,
    }
  }

  /// The id of `name`, registering it (and its graph node) if it is new.
  pub fn register(
    &mut self,
    mr: &mut MeritRank,
    name: NodeName,
  ) -> NodeId {
    if let Some(&id) = self.name_to_id.get(&name) {
      return id;
    }

    let id = self.next_id;
    self.next_id += 1;

    if id != mr.get_new_nodeid() {
      log_error!("Got unexpected node id.");
    }

    self.name_to_id.insert(name.clone(), id);
    self.id_to_info.push(NodeInfo { id, name });

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

  /// Every registered node, by id.
  pub fn len(&self) -> usize {
    self.id_to_info.len()
  }

  pub fn is_empty(&self) -> bool {
    self.id_to_info.is_empty()
  }
}
