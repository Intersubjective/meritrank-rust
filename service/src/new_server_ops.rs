use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode, Eq, PartialEq)]
pub enum ServiceRequestOpcode  {
  ReadRank,
  WriteEdge,
}
impl ServiceRequestOpcode {
    pub fn is_read(&self) -> bool {
        match self {
            ServiceRequestOpcode::ReadRank => true,
            ServiceRequestOpcode::WriteEdge => false,
        }
    }
}


pub type SubgraphName = String;
pub type NodeName = String;
#[derive(Debug, Encode, Decode)]
pub struct Request {
  pub subgraph_name: SubgraphName,
  pub opcode: ServiceRequestOpcode,
  pub ego: NodeName,
}


#[derive(Debug, Encode, Decode)]
pub struct Response { pub response: u64 }
