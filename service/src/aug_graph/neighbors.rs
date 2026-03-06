use crate::data::*;
use crate::node_registry::*;
use crate::utils::log::*;

use meritrank_core::NodeId;

use super::AugGraph;

impl AugGraph {
  pub fn fetch_neighbors(
    &self,
    ego_id: NodeId,
    focus_id: NodeId,
    dir: i64,
  ) -> Vec<(NodeInfo, Weight, NodeCluster)> {
    log_trace!("{} {} {:?}", ego_id, focus_id, dir);

    let node_data = match self.mr.graph.get_node_data(focus_id) {
      Some(data) => data,
      None => {
        log_warning!("Node not found: {}", focus_id);
        return vec![];
      },
    };

    let outgoing: Vec<(NodeId, Weight)> =
      node_data.get_outgoing_edges().collect();
    let inbound: Vec<(NodeId, Weight)> =
      node_data.get_inbound_edges().collect();

    let items: Vec<(NodeId, Weight)> = match dir {
      NEIGHBORS_OUTBOUND => outgoing,
      NEIGHBORS_INBOUND => inbound,
      NEIGHBORS_ALL => {
        let mut all = outgoing;
        all.extend(inbound);
        all
      },
      _ => {
        log_error!("Invalid direction: {}", dir);
        return vec![];
      },
    };

    items
      .into_iter()
      .filter_map(|(dst_id, weight)| {
        let (_score, cluster) = self.fetch_score_cached(ego_id, dst_id);
        self.nodes.get_by_id(dst_id).map(|info| {
          (info.clone(), weight, cluster)
        })
      })
      .collect()
  }

  pub fn read_neighbors(
    &self,
    data: OpReadNeighbors,
  ) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let kind_opt = data.kind;

    let dir = data.direction;

    if dir != NEIGHBORS_INBOUND
      && dir != NEIGHBORS_OUTBOUND
      && dir != NEIGHBORS_ALL
    {
      log_error!("Invalid direction: {}", dir);
      return vec![];
    }

    let ego = &data.ego;
    let focus = &data.focus;

    let ego_info = match self.nodes.get_by_name(ego) {
      Some(x) => x,
      _ => {
        log_error!("Node not found: {:?}", ego);
        return vec![];
      },
    };

    if !self.ensure_ego_is_user(ego, ego_info) {
      return vec![];
    }

    let ego_id = ego_info.id;

    let focus_id = match self.nodes.get_by_name(focus) {
      Some(x) => x.id,
      _ => {
        log_error!("Node not found: {:?}", focus);
        return vec![];
      },
    };

    // Handling the special case - dirty hack - of returning
    // poll results through the neighbors method.

    if kind_opt == Some(NodeKind::PollVariant)
      && node_kind_from_prefix(ego) == Some(NodeKind::User)
      && node_kind_from_prefix(focus) == Some(NodeKind::Poll)
      && dir == NEIGHBORS_INBOUND
    {
      log_error!("Poll variant not implemented.");
      return vec![];
    }

    let mut scores = self.fetch_neighbors(ego_id, focus_id, dir);

    if kind_opt == Some(NodeKind::Opinion) && dir == NEIGHBORS_INBOUND {
      scores.retain(|(node_info, _, _)| {
        self.get_object_owner(node_info.id) != Some(focus_id)
      });
    }

    self.apply_filters_and_pagination(
      scores,
      ego_info,
      &FilterOptions {
        node_kind:     None,
        hide_personal: data.hide_personal,
        score_lt:      data.lt,
        score_lte:     data.lte,
        score_gt:      data.gt,
        score_gte:     data.gte,
        index:         data.index,
        count:         data.count,
      },
      true,
    )
  }

  pub fn read_mutual_scores(
    &self,
    data: OpReadMutualScores,
  ) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let ego_info = match self.nodes.get_by_name(&data.ego) {
      Some(x) => x,
      None => {
        log_error!("Node not found: {:?}", data.ego);
        return vec![];
      },
    };

    if !self.ensure_ego_is_user(&data.ego, ego_info) {
      return vec![];
    }

    let ego_id = ego_info.id;

    let ranks = self.fetch_all_scores(ego_info);
    let mut v = Vec::<ScoreResult>::new();
    v.reserve_exact(ranks.len());

    for (node, score_value_of_dst, score_cluster_of_dst) in ranks {
      if score_value_of_dst > 0.0 && node.kind == NodeKind::User {
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(node.id) {
            Some(dst_owner_id) => self.fetch_score_cached(dst_owner_id, ego_id),
            None => (0.0, 0),
          };
        v.push(ScoreResult {
          ego:             data.ego.clone(),
          target:          node.name,
          score:           score_value_of_dst,
          reverse_score:   score_value_of_ego,
          cluster:         score_cluster_of_dst,
          reverse_cluster: score_cluster_of_ego,
        });
      }
    }
    v
  }
}
