use integer_hasher::IntMap;

use log::error;
use crate::errors::MeritRankError;

pub type NodeId = usize;
pub type Weight = f64;
pub type EdgeId = (NodeId, NodeId);

#[derive(PartialEq, Eq)]
pub enum Neighbors {
  //All,
  Positive,
  Negative,
}

#[derive(Debug, Clone, Default)]
pub struct NodeData{
  pos_edges: IntMap<NodeId, Weight>,
  neg_edges: IntMap<NodeId, Weight>,

  // The sum of positive edges is often used for normalization,
  // so it is efficient to cache it.
  pos_sum: Weight,
}




impl NodeData {
  pub fn get_pos_edges_sum(&self) -> Weight {
    self.pos_sum
  }

  pub fn neighbors(&self, mode: Neighbors) -> &IntMap<NodeId, Weight> {
    match mode {
      //Neighbors::All => Some((nbr, weight)),
      Neighbors::Positive => &self.pos_edges,
      Neighbors::Negative => &self.neg_edges,
      //_ => None,
    }
  }
}

#[derive(Debug, Clone)]
pub struct Graph {
  nodes: Vec<NodeData>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
        }
    }
  pub fn get_new_nodeid(&mut self) -> NodeId {
    self.nodes.push(NodeData::default());
    self.nodes.len()-1
  }

  /// Checks if a node with the given `NodeId` exists in the graph.
  pub fn contains_node(&self, node_id: NodeId) -> bool {
    // Check if the given NodeId exists in the nodes mapping
    self.nodes.get(node_id).is_some()
  }

  pub fn add_edge(&mut self, from: NodeId, to: NodeId, weight: Weight)->Result<(), MeritRankError> {
    if !self.contains_node(to){
      return Err(MeritRankError::NodeNotFound);
    }
    let node = self.nodes.get_mut(from).ok_or(MeritRankError::NodeNotFound)?;
    if from == to {
      error!("Trying to add self-reference edge to node {}", from);
      return Err(MeritRankError::SelfReferenceNotAllowed);
    }
    match weight {
      0.0 => {
        return Err(MeritRankError::ZeroWeightEncountered);
      },
      w if w > 0.0 => {
        node.pos_edges.insert(to, weight);
        node.pos_sum += weight;
      },
      _ => {
        node.neg_edges.insert(to, weight);
      }
    }
    Ok(())
  }

  pub fn get_node_data(&self, node_id: NodeId) -> Option<&NodeData>{
    self.nodes.get(node_id)
  }

  /// Removes the edge between the two given nodes from the graph.
  pub fn remove_edge(&mut self, from: NodeId, to: NodeId)->Result<Weight, MeritRankError>  {
    let node = self.nodes.get_mut(from).ok_or(MeritRankError::NodeNotFound)?;
    // This is slightly inefficient. More efficient would be to only try removing pos,
    // and get to neg only if pos_weight is None.
    let pos_weight = node.pos_edges.remove(&to);
    let neg_weight = node.neg_edges.remove(&to);

    node.pos_sum -= pos_weight.unwrap_or(0.0);

    assert!(!(pos_weight.is_some() && neg_weight.is_some()));
    Ok(pos_weight.or(neg_weight).expect("Edge not found"))
  }

  pub fn edge_weight(&self, from: NodeId, to: NodeId) -> Result<Option<&Weight>, MeritRankError> {
    let node = self.nodes.get(from).ok_or(MeritRankError::NodeNotFound)?;
    if !self.contains_node(to){
      return Err(MeritRankError::NodeNotFound);
    }
    Ok(node.pos_edges.get(&to).or(node.neg_edges.get(&to)))
  }

}


