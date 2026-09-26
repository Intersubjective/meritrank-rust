use crate::data::*;
use crate::node_registry::*;
use crate::utils::log::*;

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
}
