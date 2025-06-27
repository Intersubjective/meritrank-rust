use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode, Eq, PartialEq)]
pub enum ServiceRequestOpcode {
  ReadScores,
  WriteEdge,
}
impl ServiceRequestOpcode {
  pub fn is_read(&self) -> bool {
    match self {
      ServiceRequestOpcode::ReadScores => true,
      ServiceRequestOpcode::WriteEdge => false,
    }
  }
}
#[derive(Debug, Encode, Decode)]
pub struct Response {
  pub response: u64,
}

pub type SubgraphName = String;
#[derive(Debug, Encode, Decode)]
pub struct Request {
  pub subgraph_name: SubgraphName,
  pub opcode:        ServiceRequestOpcode,
  pub ego:           NodeName,
  pub score_options: FilteringOptions,
}
