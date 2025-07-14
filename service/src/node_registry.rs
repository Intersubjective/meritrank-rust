use crate::aug_graph::NodeName;
use crate::nodes::NodeKind;
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
  name_to_id: HashMap<NodeName, NodeId>,
  id_to_info: Vec<NodeInfo>,
  next_id:    NodeId,
}

impl NodeRegistry {
  pub fn new() -> Self {
    NodeRegistry {
      name_to_id: HashMap::new(),
      id_to_info: Vec::new(),
      next_id:    0,
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

  pub fn update_owner(
    &mut self,
    id: NodeId,
    new_owner: Option<NodeId>,
  ) -> bool {
    if let Some(info) = self.id_to_info.get_mut(id) {
      info.owner = new_owner;
      true
    } else {
      false
    }
  }

  pub fn len(&self) -> usize {
    self.id_to_info.len()
  }

  pub fn is_empty(&self) -> bool {
    self.id_to_info.is_empty()
  }

  pub fn nodes_by_kind(
    &self,
    kind: NodeKind,
  ) -> Vec<NodeId> {
    self
      .id_to_info
      .iter()
      .enumerate()
      .filter(|(_, info)| info.kind == kind)
      .map(|(id, _)| id)
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use meritrank_core::Graph;

  #[test]
  fn test_node_registry() {
    let mut mr = MeritRank::new(Graph::new());

    let mut registry = NodeRegistry::new();

    let user_id =
      registry.register(&mut mr, "Alice".to_string(), NodeKind::User);
    assert_eq!(user_id, 0);

    let comment_id = registry.register_with_owner(
      &mut mr,
      "Comment1".to_string(),
      NodeKind::Comment,
      user_id,
    );
    assert_eq!(comment_id, 1);

    // Test get_by_id
    let info = registry.get_by_id(0).unwrap();
    assert_eq!(info.name, "Alice");
    assert_eq!(info.kind, NodeKind::User);
    assert_eq!(info.owner, None);

    // Test get_by_name
    let info = registry.get_by_name("Comment1").unwrap();
    assert_eq!(info.id, 1);
    assert_eq!(info.kind, NodeKind::Comment);
    assert_eq!(info.owner, Some(user_id));

    // Test registering an existing name
    let existing_id =
      registry.register(&mut mr, "Alice".to_string(), NodeKind::User);
    assert_eq!(existing_id, 0);

    // Test update_owner
    assert!(registry.update_owner(1, Some(0)));
    let updated_info = registry.get_by_id(1).unwrap();
    assert_eq!(updated_info.owner, Some(0));

    // Test update_owner for non-existent id
    assert!(!registry.update_owner(2, Some(0)));

    assert_eq!(registry.len(), 2);
    assert!(!registry.is_empty());

    // Test non-existent entries
    assert_eq!(registry.get_by_id(2), None);
    assert_eq!(registry.get_by_name("Bob"), None);
  }
}
