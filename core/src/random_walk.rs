use crate::graph::NodeId;
use crate::MeritRankError;
use tinyset::SetUsize;

/// Represents a random walk through a graph.
#[derive(Clone)]
pub struct RandomWalk {
  pub nodes:                  Vec<NodeId>,
  pub negative_segment_start: Option<usize>,
}

impl RandomWalk {
  pub fn new() -> Self {
    RandomWalk {
      nodes:                  Vec::new(),
      negative_segment_start: None,
    }
  }

  pub fn from_nodes(nodes: Vec<NodeId>) -> Self {
    RandomWalk {
      nodes,
      negative_segment_start: None,
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

  pub fn clear(&mut self) {
    self.nodes.clear();
  }

  pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
    self.nodes.iter()
  }

  pub fn push(
    &mut self,
    node_id: NodeId,
    step_is_positive: bool,
  ) -> Result<(), MeritRankError> {
    let index = self.nodes.len();
    // Ensure the node_id is not the same as the last node in the walk:
    // direct self-loops are forbidden and should never happen.
    if let Some(prev) = self.nodes.last() {
      if *prev == node_id {
        return Err(MeritRankError::InternalFatalError);
      }
    }
    self.nodes.push(node_id);

    // Update `negative_segment_start` based on `step_is_positive`
    if !step_is_positive {
      if !self.negative_segment_start.is_none() {
        return Err(MeritRankError::InternalFatalError);
      }
      self.negative_segment_start = Some(index);
    }
    Ok(())
  }

  pub fn insert_first(
    &mut self,
    node_id: NodeId,
  ) {
    self.nodes.insert(0, node_id);
  }

  pub fn positive_subsegment(&self) -> impl Iterator<Item = &NodeId> {
    self
      .nodes
      .iter()
      .take(self.negative_segment_start.unwrap_or(self.nodes.len()))
  }
  pub fn negative_subsegment(&self) -> impl Iterator<Item = &NodeId> {
    self
      .nodes
      .iter()
      .skip(self.negative_segment_start.unwrap_or(self.nodes.len()))
  }

  pub fn extend(
    &mut self,
    new_segment: &RandomWalk,
  ) -> Result<(), MeritRankError> {
    if self.negative_segment_start.is_some()
      && new_segment.negative_segment_start.is_some()
    {
      return Err(MeritRankError::InternalFatalError);
    }
    if let Some(new_neg_start) = new_segment.negative_segment_start {
      self.negative_segment_start = Some(self.nodes.len() + new_neg_start);
    }
    self.nodes.extend(new_segment.get_nodes());
    Ok(())
  }

  pub fn split_from(
    &mut self,
    at: usize,
  ) -> RandomWalk {
    let new_segment_neg_start = self
      .negative_segment_start
      .filter(|&neg_start| at <= neg_start)
      .map(|neg_start| {
        self.negative_segment_start = None;
        neg_start - at
      });

    let split_segment = self.nodes.split_off(at);
    RandomWalk {
      nodes:                  split_segment,
      negative_segment_start: new_segment_neg_start,
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
