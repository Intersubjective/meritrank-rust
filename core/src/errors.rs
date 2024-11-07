/// Errors that can occur in the MeritRank implementation.
#[derive(Debug, Clone)]
pub enum MeritRankError {
  ZeroWeightEncountered,
  NodeDoesNotExist,
  SelfReferenceNotAllowed,
  RandomChoiceError,
  NoPathExists,
  NodeIdParseError,
  NodeIsNotCalculated,
  InvalidWalkLength,
  NodeNotFound,
  WalkNotFound,
  EdgeNotFound,
}

use std::error::Error;
use std::fmt::{Display, Formatter, Result};

impl Display for MeritRankError {
  fn fmt(
    &self,
    f: &mut Formatter<'_>,
  ) -> Result {
    match self {
      MeritRankError::ZeroWeightEncountered => {
        write!(f, "Edge with zero weights are not allowed")
      },
      MeritRankError::NodeDoesNotExist => {
        write!(f, "Node does not exist")
      },
      MeritRankError::SelfReferenceNotAllowed => {
        write!(f, "Self-reference is not allowed")
      },
      MeritRankError::RandomChoiceError => {
        write!(f, "Random choice error")
      },
      MeritRankError::NoPathExists => write!(f, "No path exists"),
      MeritRankError::NodeIdParseError => {
        write!(f, "Node ID parse error")
      },
      MeritRankError::NodeIsNotCalculated => {
        write!(f, "Node is not calculated")
      },
      MeritRankError::InvalidWalkLength => {
        write!(f, "Invalid walk length")
      },
      MeritRankError::NodeNotFound => {
        write!(f, "Can't find the node with the given ID")
      },
      MeritRankError::WalkNotFound => {
        write!(f, "Can't find the walk with the given ID")
      },
      MeritRankError::EdgeNotFound => {
        write!(f, "Can't find the edge between given nodes")
      },
    }
  }
}

impl Error for MeritRankError {}
