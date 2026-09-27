use crate::errors::internal_fatal;
use crate::graph::NodeId;
use crate::MeritRankError;
use tinyset::SetUsize;

/// A random walk through a graph, starting at its ego (the first node).
///
/// An absorbed walk ended because it stepped into one of its ego's walls: the wall is its last
/// node. It credits nobody and, with discredit, blames its prefix.
#[derive(Clone, Default)]
pub struct RandomWalk {
  pub nodes:    Vec<NodeId>,
  pub absorbed: bool,
}

impl RandomWalk {
  pub fn new() -> Self {
    RandomWalk::default()
  }

  pub fn from_nodes(nodes: Vec<NodeId>) -> Self {
    RandomWalk {
      nodes,
      absorbed: false,
    }
  }

  pub fn _add_node(
    &mut self,
    node_id: NodeId,
  ) {
    self.nodes.push(node_id);
  }

  pub fn get_nodes(&self) -> &[NodeId] {
    &self.nodes
  }

  pub fn len(&self) -> usize {
    self.nodes.len()
  }

  pub fn contains(
    &self,
    node_id: &NodeId,
  ) -> bool {
    self.nodes.contains(node_id)
  }

  pub fn intersects_nodes<'a, I>(
    &self,
    nodes: I,
  ) -> bool
  where
    I: IntoIterator<Item = &'a NodeId>,
  {
    let set: SetUsize = SetUsize::from_iter(self.nodes.iter().copied());
    nodes.into_iter().any(|&node| set.contains(node))
  }

  pub fn _get_nodes_mut(&mut self) -> &mut Vec<NodeId> {
    &mut self.nodes
  }

  pub fn is_empty(&self) -> bool {
    self.nodes.is_empty()
  }

  pub fn first_node(&self) -> Option<NodeId> {
    self.nodes.first().copied()
  }

  pub fn last_node(&self) -> Option<NodeId> {
    self.nodes.last().copied()
  }

  /// Empties the walk and resets all of its metadata.
  pub fn clear(&mut self) {
    self.nodes.clear();
    self.absorbed = false;
  }

  pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
    self.nodes.iter()
  }

  pub fn push(
    &mut self,
    node_id: NodeId,
  ) -> Result<(), MeritRankError> {
    if self.absorbed {
      return Err(MeritRankError::InternalFatalError(Some(
        internal_fatal::RANDOM_WALK_PUSH_ABSORBED,
      )));
    }
    // Direct self-loops are forbidden and should never happen.
    if let Some(prev) = self.nodes.last() {
      if *prev == node_id {
        return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::RANDOM_WALK_PUSH_SELF_LOOP,
        )));
      }
    }
    self.nodes.push(node_id);
    Ok(())
  }

  pub fn insert_first(
    &mut self,
    node_id: NodeId,
  ) {
    self.nodes.insert(0, node_id);
  }

  pub fn extend(
    &mut self,
    new_segment: &RandomWalk,
  ) -> Result<(), MeritRankError> {
    if self.absorbed {
      return Err(MeritRankError::InternalFatalError(Some(
        internal_fatal::RANDOM_WALK_EXTEND_ABSORBED,
      )));
    }
    self.nodes.extend(new_segment.get_nodes());
    self.absorbed = new_segment.absorbed;
    Ok(())
  }

  /// Cuts the walk at `at` and returns the tail. The absorbing arrival, if any, is the last node,
  /// so it moves with a non-empty tail.
  pub fn split_from(
    &mut self,
    at: usize,
  ) -> RandomWalk {
    let tail_absorbed = self.absorbed && at < self.nodes.len();
    if tail_absorbed {
      self.absorbed = false;
    }
    let tail = self.nodes.split_off(at.min(self.nodes.len()));
    RandomWalk {
      nodes:    tail,
      absorbed: tail_absorbed,
    }
  }
}

impl IntoIterator for RandomWalk {
  type Item = NodeId;
  type IntoIter = std::vec::IntoIter<NodeId>;

  fn into_iter(self) -> Self::IntoIter {
    self.nodes.into_iter()
  }
}
