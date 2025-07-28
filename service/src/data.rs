//  FIXME: Clean up type names consistency.

use crate::vsids::Magnitude;

use bincode::{Decode, Encode};
use meritrank_core::Weight;
use serde::{Deserialize, Serialize};

pub type NodeName = String;
pub type NodeScore = f64;
pub type NodeCluster = usize;
pub type SubgraphName = String;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, Hash)]
pub enum NodeKind {
  User,
  Beacon,
  Comment,
  Opinion,
  PollVariant,
  Poll,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct FilterOptions {
  pub node_kind:     Option<NodeKind>,
  pub hide_personal: bool,
  pub score_lt:      f64,
  pub score_lte:     bool,
  pub score_gt:      f64,
  pub score_gte:     bool,
  pub index:         u32,
  pub count:         u32,
}

impl Default for FilterOptions {
  fn default() -> Self {
    FilterOptions {
      node_kind:     None,
      hide_personal: false,
      score_lt:      f64::MAX,
      score_lte:     true,
      score_gt:      f64::MIN,
      score_gte:     true,
      index:         0,
      count:         u32::MAX,
    }
  }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpReadScores {
  pub ego:           NodeName,
  pub score_options: FilterOptions,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpReadNeighbors {
  pub ego:           NodeName,
  pub focus:         NodeName,
  pub direction:     i64,
  pub kind:          Option<NodeKind>,
  pub hide_personal: bool,
  pub lt:            Weight,
  pub lte:           bool,
  pub gt:            Weight,
  pub gte:           bool,
  pub index:         u32,
  pub count:         u32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteEdge {
  pub src:       NodeName,
  pub dst:       NodeName,
  pub amount:    Weight,
  pub magnitude: Magnitude,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteCalculate {
  pub ego: NodeName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpAddPollVariant {
  pub poll_id:    NodeName,
  pub variant_id: NodeName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpSetUserVote {
  pub user_id:    NodeName,
  pub variant_id: NodeName,
  pub amount:     Weight,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpReadNodeScore {
  pub ego:    NodeName,
  pub target: NodeName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpReadGraph {
  pub ego:           NodeName,
  pub focus:         NodeName,
  pub positive_only: bool,
  pub index:         u64,
  pub count:         u64,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpReadConnected {
  pub node: NodeName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpReadMutualScores {
  pub ego: NodeName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpReadNewEdgesFilter {
  pub src: NodeName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteSetZeroOpinion {
  pub node:  NodeName,
  pub score: Weight,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteDeleteEdge {
  pub src:   NodeName,
  pub dst:   NodeName,
  pub index: i64,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteDeleteNode {
  pub node:  NodeName,
  pub index: i64,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteNewEdgesFilter {
  pub src:    NodeName,
  pub filter: Vec<u8>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteFetchNewEdges {
  pub src:    NodeName,
  pub prefix: NodeName,
}

#[derive(Debug, Encode, Decode, Clone)]
pub enum AugGraphOp {
  WriteEdge(OpWriteEdge),
  WriteCalculate(OpWriteCalculate),
  AddPollVariant(OpAddPollVariant),
  SetUserVote(OpSetUserVote),
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ScoreResult {
  pub ego:             NodeName,
  pub target:          NodeName,
  pub score:           NodeScore,
  pub reverse_score:   NodeScore,
  pub cluster:         NodeCluster,
  pub reverse_cluster: NodeCluster,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct GraphResult {
  pub src:             NodeName,
  pub dst:             NodeName,
  pub weight:          Weight,
  pub score:           NodeScore,
  pub reverse_score:   NodeScore,
  pub cluster:         NodeCluster,
  pub reverse_cluster: NodeCluster,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ConnectionResult {
  pub src: NodeName,
  pub dst: NodeName,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct EdgeResult {
  pub src:    NodeName,
  pub dst:    NodeName,
  pub weight: Weight,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct NewEdgeResult {
  pub node:             NodeName,
  pub score:            NodeScore,
  pub score_reversed:   NodeScore,
  pub cluster:          NodeCluster,
  pub cluster_reversed: NodeCluster,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ResScores {
  pub scores: Vec<ScoreResult>,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ResNodeList {
  pub nodes: Vec<NodeName>,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ResNewEdgesFilter {
  pub bytes: Vec<u8>,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ResGraph {
  pub graph: Vec<GraphResult>,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ResConnections {
  pub connections: Vec<ConnectionResult>,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ResEdges {
  pub edges: Vec<EdgeResult>,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct ResNewEdges {
  pub new_edges: Vec<NewEdgeResult>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum ReqData {
  ReadScores(OpReadScores),
  WriteEdge(OpWriteEdge),
  WriteCalculate(OpWriteCalculate),

  //  Legacy requests
  ReadNodeList,
  ReadNodeScore(OpReadNodeScore),
  ReadGraph(OpReadGraph),
  ReadConnected(OpReadConnected),
  ReadEdges,
  ReadMutualScores(OpReadMutualScores),
  ReadNewEdgesFilter(OpReadNewEdgesFilter),
  ReadNeighbors(OpReadNeighbors),
  WriteReset,
  WriteRecalculateZero,
  WriteSetZeroOpinion(OpWriteSetZeroOpinion),
  WriteRecalculateClustering,
  WriteDeleteEdge(OpWriteDeleteEdge),
  WriteDeleteNode(OpWriteDeleteNode),
  WriteCreateContext,
  WriteNewEdgesFilter(OpWriteNewEdgesFilter),
  WriteFetchNewEdges(OpWriteFetchNewEdges),
}

#[derive(Debug, Encode, Decode)]
pub struct Request {
  //  NOTE: Subgraph name is ignored for some requests.
  pub subgraph: SubgraphName,

  pub data: ReqData,
}

#[derive(Debug, Encode, Decode)]
pub enum Response {
  Ok,
  Fail,
  Scores(ResScores),
  NodeList(ResNodeList),
  NewEdgesFilter(ResNewEdgesFilter),
  Graph(ResGraph),
  Connections(ResConnections),
  Edges(ResEdges),
  NewEdges(ResNewEdges),
}
