use indexmap::IndexMap;
use integer_hasher::BuildIntHasher;

use crate::errors::internal_fatal;
use crate::errors::MeritRankError;
use crate::RandomWalk;
use log::error;
use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use rand::Rng;
use std::sync::OnceLock;

type IntIndexMap<K, V> = IndexMap<K, V, BuildIntHasher<K>>;

pub type NodeId = usize;
pub type Weight = f64;
pub type EdgeId = (NodeId, NodeId);

/// A weighted sampling distribution over a node's out-edges, together with the exact sum of their
/// weights. Built in one pass from the current weights, never updated incrementally, so the sum
/// cannot drift or cancel.
#[derive(Debug, Clone)]
struct EdgeDistr {
  index: WeightedIndex<Weight>,
  sum:   Weight,
}

impl EdgeDistr {
  /// `None` when there are no edges or the weights do not form a distribution.
  fn build<I: Iterator<Item = Weight>>(weights: I) -> Option<EdgeDistr> {
    let weights: Vec<Weight> = weights.collect();
    if weights.is_empty() {
      return None;
    }
    let sum = weights.iter().sum();
    match WeightedIndex::new(weights) {
      Ok(index) => Some(EdgeDistr { index, sum }),
      Err(e) => {
        error!("Unable to build an edge distribution: {}", e);
        None
      },
    }
  }
}

#[derive(Debug, Clone, Default)]
pub struct NodeData {
  // Negative weights are stored as abs values, to simplify calculations
  pub pos_edges:     IntIndexMap<NodeId, Weight>,
  pub neg_edges:     IntIndexMap<NodeId, Weight>,
  pub inbound_edges: IntIndexMap<NodeId, Weight>, // Cache for inbound edges

  // Distributions and weight sums are built lazily by the first consumer after a change and
  // reset in O(1) by every change. Eager building would cost O(degree) per inserted edge, i.e.
  // O(Σ degree²) during a bulk load. `OnceLock` lets readers build them through `&self`.
  pos_distr: OnceLock<Option<EdgeDistr>>,
  abs_distr: OnceLock<Option<EdgeDistr>>,
}

impl NodeData {
  pub fn get_outgoing_edges(
    &self
  ) -> impl Iterator<Item = (NodeId, Weight)> + '_ {
    self
      .pos_edges
      .iter()
      .map(|(&node_id, &weight)| (node_id, weight))
      .chain(
        self
          .neg_edges
          .iter()
          .map(|(&node_id, &weight)| (node_id, -weight)),
      )
  }

  // This method is not used in the MeritRank algorithm, as the algorithm does not
  // consider the inbound edges. It is added for the sake of convenience of
  // higher-level users of the library that may require access to the inbound edges.
  // TODO: make the caching of the inbound edges optional
  pub fn get_inbound_edges(
    &self
  ) -> impl Iterator<Item = (NodeId, Weight)> + '_ {
    self
      .inbound_edges
      .iter()
      .map(|(&node_id, &weight)| (node_id, weight))
  }

  fn pos_distr(&self) -> Option<&EdgeDistr> {
    self
      .pos_distr
      .get_or_init(|| EdgeDistr::build(self.pos_edges.values().copied()))
      .as_ref()
  }

  fn abs_distr(&self) -> Option<&EdgeDistr> {
    self
      .abs_distr
      .get_or_init(|| {
        EdgeDistr::build(
          self.pos_edges.values().chain(self.neg_edges.values()).copied(),
        )
      })
      .as_ref()
  }

  /// Exact sum of the positive out-edge weights.
  pub fn pos_sum(&self) -> Weight {
    self.pos_distr().map_or(0.0, |d| d.sum)
  }

  /// Exact sum of the absolute out-edge weights, positive and negative.
  pub fn abs_sum(&self) -> Weight {
    self.abs_distr().map_or(0.0, |d| d.sum)
  }

  /// Resets the cached distributions after a change of the out-edges.
  fn invalidate_distributions(&mut self) {
    self.pos_distr = OnceLock::new();
    self.abs_distr = OnceLock::new();
  }

  // Return a random neighbor and whether it's from positive or negative edges
  pub fn random_neighbor<R: Rng + ?Sized>(
    &self,
    positive_only: bool,
    rng: &mut R,
  ) -> Result<Option<(NodeId, bool)>, MeritRankError> {
    if positive_only {
      if self.pos_edges.is_empty() {
        return Ok(None);
      }
      let distr = match self.pos_distr() {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::GRAPH_NODEDATA_POS_WEIGHTED_INDEX,
        ))),
      };
      let index = distr.index.sample(rng);
      let node_id = match self.pos_edges.get_index(index) {
        Some((x, _)) => *x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::GRAPH_NODEDATA_POS_KEYS_NTH,
        ))),
      };
      Ok(Some((node_id, true)))
    } else {
      if self.pos_edges.is_empty() && self.neg_edges.is_empty() {
        return Ok(None);
      }
      let distr = match self.abs_distr() {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::GRAPH_NODEDATA_ABS_WEIGHTED_INDEX,
        ))),
      };
      let index = distr.index.sample(rng);
      self.get_node_at_index(index)
    }
  }

  // Helper method to get the node at a given index from combined edges
  fn get_node_at_index(
    &self,
    index: usize,
  ) -> Result<Option<(NodeId, bool)>, MeritRankError> {
    let pos_len = self.pos_edges.len();

    if index < pos_len {
      let node_id = match self.pos_edges.get_index(index) {
        Some((x, _)) => *x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::GRAPH_GET_NODE_AT_INDEX_POS,
        ))),
      };
      Ok(Some((node_id, true)))
    } else {
      let neg_index = index - pos_len;
      let node_id = match self.neg_edges.get_index(neg_index) {
        Some((x, _)) => *x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::GRAPH_GET_NODE_AT_INDEX_NEG,
        ))),
      };
      Ok(Some((node_id, false)))
    }
  }
}

#[derive(Debug, Clone)]
pub struct Graph {
  pub nodes: Vec<NodeData>,
}

impl Graph {
  pub fn new() -> Self {
    Graph {
      nodes: Vec::new(),
    }
  }
  pub fn get_new_nodeid(&mut self) -> NodeId {
    self.nodes.push(NodeData::default());
    self.nodes.len() - 1
  }

  /// Checks if a node with the given `NodeId` exists in the graph.
  pub fn contains_node(
    &self,
    node_id: NodeId,
  ) -> bool {
    // Check if the given NodeId exists in the nodes mapping
    self.nodes.get(node_id).is_some()
  }

  pub fn set_edge(
    &mut self,
    from: NodeId,
    to: NodeId,
    weight: Weight,
  ) -> Result<(), MeritRankError> {
    if !self.contains_node(from) || !self.contains_node(to) {
      return Err(MeritRankError::NodeNotFound);
    }
    if from == to {
      error!("Trying to add self-reference edge to node {}", from);
      return Err(MeritRankError::SelfReferenceNotAllowed);
    }
    if self.edge_weight(from, to)?.is_some() {
      if self.remove_edge(from, to).is_err() {
        return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::GRAPH_SET_EDGE_REMOVE_FAILED,
        )));
      }
    }
    if weight.is_nan() {
      error!("Trying to set NaN weight for edge from {} to {}", from, to);
      return Err(MeritRankError::NaNWeightEncountered);
    }
    if weight.is_infinite() {
      error!(
        "Trying to set infinite weight for edge from {} to {}",
        from, to
      );
      return Err(MeritRankError::InfWeightEncountered);
    }

    let node = self
      .nodes
      .get_mut(from)
      .ok_or(MeritRankError::NodeNotFound)?;
    match weight {
      0.0 => {
        return Err(MeritRankError::ZeroWeightEncountered);
      },
      w if w > 0.0 => {
        node.pos_edges.insert(to, weight);
      },
      _ => {
        node.neg_edges.insert(to, weight.abs());
      },
    }
    node.invalidate_distributions();

    // Update inbound edge cache for the target node
    self.nodes[to].inbound_edges.insert(from, weight);

    Ok(())
  }

  pub fn get_node_data(
    &self,
    node_id: NodeId,
  ) -> Option<&NodeData> {
    self.nodes.get(node_id)
  }
  pub fn get_node_data_mut(
    &mut self,
    node_id: NodeId,
  ) -> Option<&mut NodeData> {
    self.nodes.get_mut(node_id)
  }

  /// Removes the edge between the two given nodes from the graph.
  pub fn remove_edge(
    &mut self,
    from: NodeId,
    to: NodeId,
  ) -> Result<Weight, MeritRankError> {
    // Remove from inbound edge cache of the target node
    let dst_node = self
      .nodes
      .get_mut(to)
      .ok_or(MeritRankError::NodeNotFound)?;
    dst_node.inbound_edges.swap_remove(&from);

    let node = self
      .nodes
      .get_mut(from)
      .ok_or(MeritRankError::NodeNotFound)?;
    // This is slightly inefficient. More efficient would be to only try removing pos,
    // and get to neg only if pos_weight is None. We keep it to check the invariant of
    // not having both pos and neg weights for an edge simultaneously.
    let pos_weight = node.pos_edges.swap_remove(&to);
    let neg_weight = node.neg_edges.swap_remove(&to);

    // Both pos and neg weights should never be present at the same time.
    assert!(!(pos_weight.is_some() && neg_weight.is_some()));
    node.invalidate_distributions();

    Ok(if let Some(weight) = pos_weight {
      weight
    } else if let Some(weight) = neg_weight {
      -weight
    } else {
      panic!("Edge not found")
    })
  }

  pub fn edge_weight(
    &self,
    from: NodeId,
    to: NodeId,
  ) -> Result<Option<Weight>, MeritRankError> {
    let node = self.nodes.get(from).ok_or(MeritRankError::NodeNotFound)?;
    if !self.contains_node(to) {
      return Err(MeritRankError::NodeNotFound);
    }
    Ok(if let Some(weight) = node.pos_edges.get(&to) {
      Some(*weight)
    } else if let Some(weight) = node.neg_edges.get(&to) {
      Some(-*weight)
    } else {
      None
    })
  }

  pub fn generate_walk_segment<R: Rng + ?Sized>(
    &self,
    start_node: NodeId,
    alpha: f64,
    positive_only: bool,
    rng: &mut R,
  ) -> Result<RandomWalk, MeritRankError> {
    let mut node = start_node;
    let mut segment = RandomWalk::new();

    let mut negative_continuation_mode = false;
    // When this variable becomes true, it means that a walk has encountered a negative edge,
    // followed it, and now the walk is in the "negative continuation mode", meaning we
    // will only follow positive edges, from now on, storing the index of its
    // start in "negative_segment_start" variable. Later, we will u "punish" the nodes that
    // were encountered in the negative continuation mode:
    //  +  +  -     +  +  +
    // A->B->C->(-D)->E->F->G
    // P  P  P    N   N  N  N

    loop {
      let node_data = match self.get_node_data(node) {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::GRAPH_GENERATE_WALK_GET_NODE_DATA,
        ))),
      };
      if rng.random::<f64>() > alpha {
        break;
      }
      if let Some((next_step, step_is_positive)) = node_data
        .random_neighbor(negative_continuation_mode || positive_only, rng)?
      {
        segment.push(next_step, step_is_positive)?;
        if !step_is_positive {
          assert!(!negative_continuation_mode);
          negative_continuation_mode = true;
        }
        node = next_step;
      } else {
        // Dead-end encountered
        break;
      }
    }
    Ok(segment)
  }

  pub fn continue_walk<R: Rng + ?Sized>(
    &self,
    walk: &mut RandomWalk,
    alpha: f64,
    rng: &mut R,
  ) -> Result<(), MeritRankError> {
    // If the original walk is already in "negative mode",
    // we should restrict segment generation to positive edges
    let positive_only = walk.negative_segment_start.is_some();
    let start_node = match walk.last_node() {
      Some(x) => x,
      None => return Err(MeritRankError::InternalFatalError(Some(
        internal_fatal::GRAPH_CONTINUE_WALK_LAST_NODE,
      ))),
    };
    let new_segment =
      self.generate_walk_segment(start_node, alpha, positive_only, rng)?;

    walk.extend(&new_segment)
  }

  pub fn extend_walk_in_case_of_edge_deletion<R: Rng + ?Sized>(
    &self,
    walk: &mut RandomWalk,
    rng: &mut R,
  ) -> Result<(), MeritRankError> {
    // No force_first_step, so this is "edge deletion mode"
    //
    // Force addition of the first step by extending the original walk with it.
    // Make sure that positive/negative subsegment marking is taken into account.
    // Forcing the step is neccessary in case of edge deletion in optimized mode:
    // we simulate the situation when the actual edge that was taken in the first case
    // was an edge different from the deleted one. Therefore, we should not apply
    // alpha-based stop to it, as this would lead to bias.
    let src_node = walk.last_node().unwrap();
    let node_data = self.get_node_data(src_node).unwrap();
    let adding_to_negative_subsegment = walk.negative_segment_start.is_some();
    if let Some((forced_step, step_is_positive)) =
      node_data.random_neighbor(adding_to_negative_subsegment, rng)?
    {
      walk.push(forced_step, step_is_positive)?;
    }
    Ok(())
  }

  pub fn get_inbound_edges(
    &self,
    node_id: NodeId,
  ) -> Result<impl Iterator<Item = (NodeId, Weight)> + '_, MeritRankError> {
    self
      .nodes
      .get(node_id)
      .map(|node| node.get_inbound_edges())
      .ok_or(MeritRankError::NodeNotFound)
  }
}
