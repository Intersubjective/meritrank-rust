use crate::data::*;
use crate::utils::log::*;

use meritrank_core::NodeId;

use super::AugGraph;

impl AugGraph {
  /// Apply a single operation to this graph (immutable ref to op; used by double-buffered processor).
  /// Prefer `apply_seq_op`, which also reseeds the operation's stream and records its number.
  pub fn apply_op(
    &mut self,
    op: &AugGraphOp,
  ) {
    log_command!("{:?}", op);
    // TV is needed only by the staleness heuristic, and only when there is a snapshot to age.
    let track = self.settings.snapshot_staleness > 0.0
      && !self.snapshots.is_empty()
      && !matches!(op, AugGraphOp::BulkLoadEdges(_));
    self.mr.set_tv_tracking(track);
    if let AugGraphOp::ClearEgo(ego) = op {
      // Eviction keeps the frame's content as a snapshot.
      self.capture_frame(*ego);
    }

    // Deleting a node is a hard event for its snapshot (and for samples of it in flight).
    let deleted = match op {
      AugGraphOp::DeleteNode(name) => self.nodes.get_by_name(name).map(|i| i.id),
      _ => None,
    };

    // A panicking operation may have changed the graph partly: its mutations are still absorbed
    // before the panic goes on (strict snapshots must never outlive a change).
    let outcome =
      std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.apply_op_inner(op)));

    let mut mutations = self.mr.take_mutations();
    if let Some(d) = deleted {
      if let Err(i) = mutations.wall_owners.binary_search(&d) {
        mutations.wall_owners.insert(i, d);
      }
    }
    let dirty = self.mr.take_dirty_egos();
    match op {
      AugGraphOp::BulkLoadEdges(_) => self.clear_snapshots(),
      _ => self.absorb_mutations(&mutations),
    }
    for ego in dirty {
      // Eviction is not a new estimate: the snapshot holds the same content.
      if matches!(op, AugGraphOp::ClearEgo(e) if *e == ego) {
        continue;
      }
      self.bump_revision(ego);
    }
    if let Err(panic) = outcome {
      std::panic::resume_unwind(panic);
    }
  }

  fn apply_op_inner(
    &mut self,
    op: &AugGraphOp,
  ) {
    match op {
      AugGraphOp::WriteReset => {
        // A new incarnation: both copies derive the same epoch from the operation.
        let (stream, seq) = (self.stream, self.applied_seq);
        let epoch = super::mix64(self.epoch ^ super::mix64(seq));
        let quota = self.snapshots.quota();
        *self = AugGraph::build_reset(self.settings.clone(), stream, epoch);
        self.applied_seq = seq;
        self.set_snapshot_quota(quota);
        self.clear_snapshots();
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
        self.zero_revision += 1;
      },
      AugGraphOp::WriteRecalculateClustering => {
        log_warning!("Recalculate clustering is ignored!")
      },
      AugGraphOp::ClearEgo(ego_id) => {
        if let Err(e) = self.mr.clear_ego(*ego_id) {
          log_error!("ClearEgo failed: {}", e);
        }
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
      AugGraphOp::Barrier => {},
      AugGraphOp::EnsureCalculated(egos) => {
        for ego in egos {
          self.ensure_calculated(*ego);
        }
      },
      AugGraphOp::AdmitSnapshots(batch) => self.admit(batch),
      AugGraphOp::SetSnapshotQuota(quota) => self.set_snapshot_quota(*quota),
    }
  }
}
