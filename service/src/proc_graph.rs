use crate::aug_graph::{AugGraph, NodeName};
use crate::nonblocking_loop::ConcurrentDataProcessor;
use crate::utils::log::*;
use crate::vsids::Magnitude;

use bincode::{Decode, Encode};
use left_right::Absorb;
use meritrank_core::Weight;

#[derive(Debug, Encode, Decode, Clone)]
pub enum AugGraphOp {
  WriteEdgeOp {
    src:       NodeName,
    dst:       NodeName,
    amount:    Weight,
    magnitude: Magnitude,
  },
  AddPollVariantOp {
    poll_id:    NodeName,
    variant_id: NodeName,
  },
  SetUserVoteOp {
    user_id:    NodeName,
    variant_id: NodeName,
    amount:     Weight,
  },
}

impl Absorb<AugGraphOp> for AugGraph {
  fn absorb_first(
    &mut self,
    op: &mut AugGraphOp,
    _: &Self,
  ) {
    log_trace!();

    match op {
      AugGraphOp::WriteEdgeOp {
        src,
        dst,
        amount,
        magnitude,
      } => {
        self.set_edge(src.clone(), dst.clone(), *amount, *magnitude);
      },
      _ => {
        log_error!("Not implemented.");
      },
    }
  }

  fn sync_with(
    &mut self,
    first: &Self,
  ) {
    *self = first.clone()
  }
}

pub type GraphProcessor = ConcurrentDataProcessor<AugGraph, AugGraphOp>;
