use crate::aug_graph::{AugGraph, AugGraphOpcode};
use crate::nonblocking_loop::ConcurrentDataProcessor;
use left_right::Absorb;

pub struct AugGraphOp {
  pub opcode:  AugGraphOpcode,
  pub ego_str: String,
}

impl AugGraphOp {
  pub fn new(
    opcode: AugGraphOpcode,
    ego_str: String,
  ) -> Self {
    AugGraphOp {
      opcode,
      ego_str,
    }
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

pub type GraphProcessor = ConcurrentDataProcessor<AugGraph, AugGraphOp>;
