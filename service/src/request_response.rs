use crate::read::FilterOptions;
use crate::read::ScoreResult;
use crate::vsids::Magnitude;
use crate::aug_graph::NodeName;

use meritrank_core::Weight;

use bincode::{Decode, Encode};

pub type SubgraphName = String;

#[derive(Debug, Clone, Encode, Decode)]
pub struct ReqReadScores {
  pub ego:           NodeName,
  pub score_options: FilterOptions,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ReqWriteEdge {
  pub src:       NodeName,
  pub dst:       NodeName,
  pub amount:    Weight,
  pub magnitude: Magnitude,
}

#[derive(Debug, Encode, Decode)]
pub struct ResScores {
  pub data: Vec<ScoreResult>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum ReqData {
  ReadScores(ReqReadScores),
  WriteEdge(ReqWriteEdge),
}

#[derive(Debug, Encode, Decode)]
pub struct Request {
  pub subgraph: SubgraphName,
  pub data:     ReqData,
}

#[derive(Debug, Encode, Decode)]
pub enum Response {
  Ok,
  Fail,
  Scores(ResScores),
}
