/// Errors that can occur in the MeritRank implementation.
#[derive(Debug, Clone)]
pub enum MeritRankError {
    NodeDoesNotExist,
    SelfReferenceNotAllowed,
    RandomChoiceError,
    NoPathExists,
    NodeIdParseError,
    NodeDoesNotCalculated,
    InvalidWalkLength,
    // Experimental
    InvalidNode
}

use std::error::Error;
use std::fmt::{Display, Formatter, Result};

impl Display for MeritRankError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            MeritRankError::NodeDoesNotExist => write!(f, "Node does not exist"),
            MeritRankError::SelfReferenceNotAllowed => write!(f, "Self-reference is not allowed"),
            MeritRankError::RandomChoiceError => write!(f, "Random choice error"),
            MeritRankError::NoPathExists => write!(f, "No path exists"),
            MeritRankError::NodeIdParseError => write!(f, "Node ID parse error"),
            MeritRankError::NodeDoesNotCalculated => write!(f, "Node does not calculated"),
            MeritRankError::InvalidWalkLength => write!(f, "Invalid walk length"),
            MeritRankError::InvalidNode => write!(f, "Invalid node"),
        }
    }
}

impl Error for MeritRankError {
    // fn source(&self) -> Option<&(dyn Error + 'static)> {
    //     match self {
    //         MeritRankError::NodeDoesNotExist => None,
    //         MeritRankError::SelfReferenceNotAllowed => None,
    //         MeritRankError::RandomChoiceError => None,
    //         MeritRankError::NoPathExists => None,
    //         MeritRankError::NodeIdParseError => None,
    //     }
    // }
}
