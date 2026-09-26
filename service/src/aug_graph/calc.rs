use crate::data::*;
use crate::node_registry::*;
use crate::utils::log::*;

use meritrank_core::NodeId;

use super::AugGraph;

impl AugGraph {
  pub fn calculate(
    &mut self,
    ego: NodeName,
  ) {
    log_trace!("{:?}", ego);

    let kind = match node_kind_from_prefix(&ego) {
      Some(x) => x,
      None => {
        log_error!("Failed to get node kind for {:?}", ego);
        return;
      },
    };

    if kind != NodeKind::User {
      log_error!("Non-user node used as ego for calculation (rejected): {:?}", ego);
      return;
    }

    let ego_id = self.nodes.register(&mut self.mr, ego, kind);

    match self.mr.calculate(ego_id) {
      Ok(_) => {},
      Err(e) => log_error!("{}", e),
    };
  }

  /// Calculates the ego unless it already is. Unknown or non-user egos are ignored.
  pub fn ensure_calculated(
    &mut self,
    ego_id: NodeId,
  ) {
    match self.nodes.get_by_id(ego_id) {
      Some(info) if info.kind == NodeKind::User => {},
      _ => return,
    }
    if self.mr.is_calculated(ego_id) {
      return;
    }
    if let Err(e) = self.mr.calculate(ego_id) {
      log_error!("{}", e);
    }
  }
}
