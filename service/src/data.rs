//  FIXME: Clean up type names consistency.

use bincode::{Decode, Encode};

pub const NEIGHBORS_ALL: i64 = 0;
pub const NEIGHBORS_OUTBOUND: i64 = 1;
pub const NEIGHBORS_INBOUND: i64 = 2;
use serde::{Deserialize, Serialize};

pub type NodeName = String;
pub type NodeScore = f64;
pub use meritrank_core::{NodeId, Weight};
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
    Self {
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
  pub magnitude: u32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct BulkEdge {
  pub src:       NodeName,
  pub dst:       NodeName,
  pub amount:    Weight,
  pub magnitude: u32,
  pub context:   SubgraphName,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteBulkEdges {
  pub edges: Vec<BulkEdge>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct OpWriteCalculate {
  pub ego: NodeName,
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
pub struct OpWriteZeroOpinion {
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
  BulkLoadEdges(Vec<OpWriteEdge>),
  WriteCalculate(OpWriteCalculate),
  WriteZeroOpinion(OpWriteZeroOpinion),
  WriteReset,
  WriteRecalculateClustering,
  ClearEgo(NodeId),
  DeleteNode(NodeName),
  Stamp(u64),
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
  pub nodes: Vec<(NodeName,)>,
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

/// Stats snapshot returned by GetStats (same shape as ProcessorStats snapshot).
#[derive(Debug, Clone, Default, Encode, Decode, Serialize, Deserialize)]
pub struct ResStats {
  pub pending:   usize,
  pub median_us: u64,
  pub p95_us:    u64,
  pub p99_us:    u64,
  pub min_us:    u64,
  pub max_us:    u64,
  pub count:     usize,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum ReqData {
  ReadScores(OpReadScores),
  WriteEdge(OpWriteEdge),
  WriteBulkEdges(OpWriteBulkEdges),
  WriteCalculate(OpWriteCalculate),
  Stamp(u64),
  Sync(u64),

  ResetStats,
  GetStats,

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
  WriteZeroOpinion(OpWriteZeroOpinion),
  WriteRecalculateClustering,
  WriteDeleteEdge(OpWriteDeleteEdge),
  WriteDeleteNode(OpWriteDeleteNode),
  WriteCreateContext,
  WriteNewEdgesFilter(OpWriteNewEdgesFilter),
  WriteFetchNewEdges(OpWriteFetchNewEdges),
}

impl ReqData {
  /// Returns the ego for read operations that require walks (scores, graph, neighbors, mutual).
  pub fn read_ego(&self) -> Option<&NodeName> {
    use ReqData::*;
    match self {
      ReadScores(data) => Some(&data.ego),
      ReadNodeScore(data) => Some(&data.ego),
      ReadGraph(data) => Some(&data.ego),
      ReadNeighbors(data) => Some(&data.ego),
      ReadMutualScores(data) => Some(&data.ego),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Request {
  //  NOTE: Subgraph name is ignored for some requests.
  pub subgraph: SubgraphName,

  pub data: ReqData,
}

#[derive(Debug, Encode, Decode)]
pub enum Response {
  Ok,
  Fail,
  NotImplemented,
  Stamp(u64),
  Scores(ResScores),
  NodeList(ResNodeList),
  NewEdgesFilter(ResNewEdgesFilter),
  Graph(ResGraph),
  Connections(ResConnections),
  Edges(ResEdges),
  NewEdges(ResNewEdges),
  Stats(ResStats),
}
