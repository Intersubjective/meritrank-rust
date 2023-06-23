/// Errors that can occur in the MeritRank implementation.
#[derive(Debug, Clone)]
pub enum MeritRankError {
    NodeDoesNotExist,
    SelfReferenceNotAllowed,
    RandomChoiceError,
    NoPathExists,
}

impl std::fmt::Display for MeritRankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeritRankError::NodeDoesNotExist => write!(f, "NodeDoesNotExist"),
            MeritRankError::SelfReferenceNotAllowed => write!(f, "SelfReferenceNotAllowed"),
            MeritRankError::RandomChoiceError => write!(f, "RandomChoiceError"),
            MeritRankError::NoPathExists => write!(f, "NoPathExists"),
        }
    }
}
