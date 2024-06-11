/// Errors that can occur in the MeritRank implementation.
#[derive(Debug, Clone)]
pub enum MeritRankError {
  NodeDoesNotExist,
  SelfReferenceNotAllowed,
  RandomChoiceError,
  NoPathExists,
  NodeIdParseError,
  NodeIsNotCalculated,
  InvalidWalkLength,
  InvalidNode,
}

use std::error::Error;
use std::fmt::{Display, Formatter, Result};

impl Display for MeritRankError {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      MeritRankError::NodeDoesNotExist        => write!(f, "Node does not exist"),
      MeritRankError::SelfReferenceNotAllowed => write!(f, "Self-reference is not allowed"),
      MeritRankError::RandomChoiceError       => write!(f, "Random choice error"),
      MeritRankError::NoPathExists            => write!(f, "No path exists"),
      MeritRankError::NodeIdParseError        => write!(f, "Node ID parse error"),
      MeritRankError::NodeIsNotCalculated     => write!(f, "Node is not calculated"),
      MeritRankError::InvalidWalkLength       => write!(f, "Invalid walk length"),
      MeritRankError::InvalidNode             => write!(f, "Invalid node"),
    }
  }
}

impl Error for MeritRankError {
}
