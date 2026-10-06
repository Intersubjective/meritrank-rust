use crate::data::*;
use crate::utils::log::*;

use meritrank_core::NodeId;

use super::AugGraph;

impl AugGraph {
  /// An explicit (re)calculation of the ego's frame, registering it if it is new. Fresh frames
  /// are seeded per ego (D14): on an unchanged graph the frame is reproduced exactly.
  pub fn calculate(
    &mut self,
    ego: NodeName,
  ) {
    log_trace!("{:?}", ego);
    if ego.is_empty() {
      log_error!("Empty node name used as ego (rejected)");
      return;
    }
    let ego_id = self.nodes.register(&mut self.mr, ego);
    self.calculate_fresh(ego_id);
  }

  /// Calculates the ego unless it already is. Unknown egos are ignored.
  pub fn ensure_calculated(
    &mut self,
    ego_id: NodeId,
  ) {
    if self.nodes.get_by_id(ego_id).is_none() || self.mr.is_calculated(ego_id) {
      return;
    }
    self.calculate_fresh(ego_id);
  }

  fn calculate_fresh(
    &mut self,
    ego_id: NodeId,
  ) {
    let seed = self.fresh_seed(ego_id);
    match self.mr.calculate_seeded(ego_id, seed) {
      Ok(_) => {
        self.counters.frames_calculated += 1;
        // The resident frame is authoritative now.
        self.drop_snapshot(ego_id);
      },
      Err(e) => log_error!("{}", e),
    }
  }
}
