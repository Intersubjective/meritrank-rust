use crate::data::*;
use crate::utils::log::*;

use meritrank_core::NodeId;

use super::AugGraph;

impl AugGraph {
  /// Apply a single operation to this graph (immutable ref to op; used by double-buffered processor).
  pub fn apply_op(
    &mut self,
    op: &AugGraphOp,
  ) {
    log_command!("{:?}", op);

    match op {
      AugGraphOp::WriteReset => {
        *self = AugGraph::new(self.settings.clone());
      },
      AugGraphOp::WriteEdge(OpWriteEdge {
        src,
        dst,
        amount,
        magnitude,
      }) => {
        self.set_edge(src.clone(), dst.clone(), *amount, *magnitude);
      },
      AugGraphOp::BulkLoadEdges(edges) => {
        self.bulk_load_edges(edges.clone());
      },
      AugGraphOp::WriteCalculate(OpWriteCalculate {
        ego,
      }) => {
        self.calculate(ego.clone());
      },
      AugGraphOp::WriteZeroOpinion(OpWriteZeroOpinion {
        node,
        score,
      }) => {
        let id = match self.nodes.get_by_name(node) {
          Some(x) => x.id,
          None => {
            log_error!("Node not found: {:?}", node);
            return;
          },
        };

        if id >= self.zero_opinion.len() {
          self.zero_opinion.resize(id + 1, 0.0);
        }
        self.zero_opinion[id] = *score;
      },
      AugGraphOp::WriteRecalculateZero => self.recalculate_zero(),
      AugGraphOp::WriteRecalculateClustering => {
        log_warning!("Recalculate clustering is ignored!")
      },
      AugGraphOp::DeleteNode(node) => {
        if let Some(src_info) = self.nodes.get_by_name(node) {
          let src_id = src_info.id;
          let dst_ids: Vec<NodeId> = self
            .mr
            .graph
            .get_node_data(src_id)
            .map(|data| {
              data
                .get_outgoing_edges()
                .map(|(dst_id, _)| dst_id)
                .collect()
            })
            .unwrap_or_default();
          for dst_id in dst_ids {
            match self.mr.set_edge(src_id, dst_id, 0.0) {
              Ok(_) => {},
              Err(e) => log_error!("{}", e),
            }
          }
        } else {
          log_warning!("DeleteNode: node not found: {:?}", node);
        }
      },
      AugGraphOp::Stamp(value) => self.stamp = *value,
    }
  }
}
