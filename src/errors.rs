/// Errors that can occur in the MeritRank implementation.
#[derive(Debug, Clone)]
pub enum MeritRankError {
    NodeDoesNotExist,
    SelfReferenceNotAllowed,
    RandomChoiceError,
    NoPathExists,
    NodeIdParseError,
}

use std::fmt::{Display, Formatter, Result};
use std::error::Error;

impl Display for MeritRankError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            MeritRankError::NodeDoesNotExist => write!(f, "NodeDoesNotExist"),
            MeritRankError::SelfReferenceNotAllowed => write!(f, "SelfReferenceNotAllowed"),
            MeritRankError::RandomChoiceError => write!(f, "RandomChoiceError"),
            MeritRankError::NoPathExists => write!(f, "NoPathExists"),
            MeritRankError::NodeIdParseError => write!(f, "NodeIdParseError"),
        }
    }
}


impl Error for MeritRankError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MeritRankError::NodeDoesNotExist => None,
            MeritRankError::SelfReferenceNotAllowed => None,
            MeritRankError::RandomChoiceError => None,
            MeritRankError::NoPathExists => None,
            MeritRankError::NodeIdParseError => None,
        }
    }
}
