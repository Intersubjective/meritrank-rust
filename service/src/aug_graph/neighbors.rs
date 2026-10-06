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
        let (_score, cluster) = self.fetch_score_clustered(ego_id, dst_id);
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

    // `data.kind` and `data.hide_personal` are ignored: one node class, no owners (D14).
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

    let ego_id = ego_info.id;

    let focus_id = match self.nodes.get_by_name(focus) {
      Some(x) => x.id,
      _ => {
        log_error!("Node not found: {:?}", focus);
        return vec![];
      },
    };

    let scores = self.fetch_neighbors(ego_id, focus_id, dir);

    self.apply_filters_and_pagination(
      scores,
      ego_info,
      &FilterOptions {
        node_kind:     None,
        hide_personal: false,
        score_lt:      data.lt,
        score_lte:     data.lte,
        score_gt:      data.gt,
        score_gte:     data.gte,
        index:         data.index,
        count:         data.count,
      },
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

    let ego_id = ego_info.id;

    let ranks = self.fetch_all_scores(ego_info);
    let mut v = Vec::<ScoreResult>::new();
    v.reserve_exact(ranks.len());

    for (node, score_value_of_dst, score_cluster_of_dst) in ranks {
      if score_value_of_dst > 0.0 {
        let (score_value_of_ego, score_cluster_of_ego) =
          self.fetch_score_clustered(node.id, ego_id);
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
