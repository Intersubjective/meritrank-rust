use bincode::{Decode, Encode};
use left_right::Absorb;
use meritrank_core::MeritRank;

#[derive(Clone)]
pub struct AugGraph {
  _mr: MeritRank,
}

#[derive(Debug, Encode, Decode, Eq, PartialEq)]
pub enum AugGraphOpcode {
  WriteEdge,
}


pub struct AugGraphOp {
  pub opcode: AugGraphOpcode,
  pub ego_str: String,
}

impl AugGraphOp {
    pub fn new(opcode: AugGraphOpcode, ego_str: String) -> Self {
        AugGraphOp {
            opcode,
            ego_str,
        }
    }
}

impl AugGraph {
  pub fn new() -> AugGraph {
    todo!()
  }
}
impl Absorb<AugGraphOp> for AugGraph {
  fn absorb_first(
    &mut self,
    _operation: &mut AugGraphOp,
    _: &Self,
  ) {
    todo!()
  }

  fn sync_with(
    &mut self,
    first: &Self,
  ) {
    *self = first.clone()
  }
}
