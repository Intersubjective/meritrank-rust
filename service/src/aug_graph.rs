use crate::data::*;
use crate::helpers::*;
use crate::node_registry::*;
use crate::settings::*;
use crate::utils::{log::*, quantiles::*};
use crate::vsids::{Magnitude, VSIDSManager};

use left_right::Absorb;
use meritrank_core::{constants::EPSILON, Graph, MeritRank, NodeId, Weight};
use moka::sync::Cache;
use petgraph::graph::{DiGraph, NodeIndex};
use simple_pagerank::Pagerank;

use std::{collections::HashMap, time::Duration};

pub type ClusterGroupBounds = Vec<NodeScore>;

#[derive(Clone)]
pub struct AugGraph {
  pub mr:                    MeritRank,
  pub nodes:                 NodeRegistry,
  pub settings:              Settings,
  pub zero_opinion:          Vec<NodeScore>, // FIXME: change to map because of sparseness
  pub cached_scores:         Cache<(NodeId, NodeId), NodeScore>,
  pub cached_score_clusters: Cache<(NodeId, NodeKind), ClusterGroupBounds>,
  pub vsids:                 VSIDSManager,
  pub stamp:                 u64,
}

#[derive(Debug)]
enum AugGraphError {
  SelfReference,
  IncorrectNodeKinds(NodeName, NodeName),
}

impl AugGraph {
  pub fn new(settings: Settings) -> AugGraph {
    let cached_scores: Cache<(NodeId, NodeId), NodeScore> = Cache::builder()
      .max_capacity(settings.scores_cache_size as u64)
      .time_to_live(Duration::from_secs(settings.scores_cache_timeout))
      .build();

    let cached_score_clusters: Cache<(NodeId, NodeKind), ClusterGroupBounds> =
      Cache::builder()
        .max_capacity(settings.score_clusters_cache_size as u64)
        .time_to_live(Duration::from_secs(settings.score_clusters_timeout))
        .build();

    AugGraph {
      mr: MeritRank::new(Graph::new()),
      nodes: NodeRegistry::new(),
      settings: settings.clone(),
      zero_opinion: Vec::new(),
      cached_scores,
      cached_score_clusters,
      vsids: VSIDSManager::new(),
      stamp: 0,
    }
  }

  pub fn update_node_score_clustering(
    &self,
    ego: NodeId,
    kind: NodeKind,
  ) -> ClusterGroupBounds {
    log_trace!("{} {:?}", ego, kind);
    let node_ids = self.nodes.nodes_by_kind(kind);
    let bounds = self.calculate_score_clusters_bounds(ego, kind, &*node_ids);
    self
      .cached_score_clusters
      .insert((ego, kind), bounds.clone());
    bounds
  }

  fn calculate_score_clusters_bounds(
    &self,
    ego: NodeId,
    kind: NodeKind,
    node_ids: &[NodeId],
  ) -> Vec<NodeScore> {
    log_trace!("{} {:?}", ego, kind);

    let scores: Vec<NodeScore> = node_ids
      .iter()
      .map(|dst| self.fetch_raw_score(ego, *dst))
      .filter(|score| *score >= f64::EPSILON)
      .collect();

    if scores.is_empty() {
      return vec![0.0; self.settings.num_score_quantiles - 1];
    }

    calculate_quantiles_bounds(scores, self.settings.num_score_quantiles)
  }

  pub fn apply_score_clustering(
    &self,
    ego_id: NodeId,
    score: NodeScore,
    kind: NodeKind,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{} {} {:?}", ego_id, score, kind);

    if score < f64::EPSILON {
      //  Clusterize only positive scores.
      return (score, 0);
    }

    let bounds: &Vec<Weight> = &self
      .cached_score_clusters
      .get(&(ego_id, kind))
      .unwrap_or_else(|| self.update_node_score_clustering(ego_id, kind));

    if bounds_are_empty(bounds) {
      return (score, 1); // Return 1 instead of 0 for empty bounds
    }
    let mut cluster = 1; // Start with cluster 1

    for bound in bounds {
      if score <= *bound {
        break;
      }
      cluster += 1;
    }
    (score, cluster)
  }

  pub fn read_scores(
    &self,
    data: OpReadScores,
  ) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let ego = data.ego;
    let filter_options = data.score_options;

    if let Some(ego_info) = self.nodes.get_by_name(&ego) {
      if ego_info.kind != NodeKind::User {
        log_warning!("Trying to use non-user as ego {}", ego);
        return vec![];
      }
      let scores = self.fetch_all_scores(ego_info);
      self.apply_filters_and_pagination(
        scores,
        ego_info,
        &filter_options,
        false,
      )
    } else {
      log_error!("Ego not found: {:?}", ego);
      vec![]
    }
  }

  pub fn read_node_score(
    &self,
    data: OpReadNodeScore,
  ) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let ego = data.ego;
    let dst = data.target;

    let ego_info = match self.nodes.get_by_name(&ego) {
      Some(x) => x,
      None => {
        log_error!("Node not found: {:?}", ego);
        return vec![];
      },
    };

    let dst_id = match self.nodes.get_by_name(&dst) {
      Some(x) => x.id,
      None => {
        log_error!("Node not found: {:?}", dst);
        return vec![];
      },
    };

    let (score, cluster) = self.apply_score_clustering(
      ego_info.id,
      self.fetch_raw_score(ego_info.id, dst_id),
      ego_info.kind,
    );
    let (reverse_score, reverse_cluster) =
      match self.get_object_owner(dst_id) {
        Some(dst_owner_id) => self.fetch_score_cached(dst_owner_id, ego_info.id),
        None => (0.0, 0),
      };

    vec![ScoreResult {
      ego: ego.into(),
      target: dst.into(),
      score,
      reverse_score,
      cluster,
      reverse_cluster,
    }]
  }

  fn validate_read_graph_params_and_setup(
    &self,
    ego_str: &str,
    focus_str: &str,
  ) -> Result<
    (
      NodeId,
      NodeId,
      DiGraph<NodeId, Weight>,
      HashMap<NodeId, NodeIndex>,
      HashMap<NodeIndex, NodeId>,
    ),
    String,
  > {
    let ego_id = match self.nodes.get_by_name(ego_str) {
      Some(x) => x.id,
      None => {
        return Err(format!("Node not found: {:?}", ego_str));
      },
    };

    let focus_id = match self.nodes.get_by_name(focus_str) {
      Some(x) => x.id,
      None => {
        return Err(format!("Node not found: {:?}", focus_str));
      },
    };

    let mut indices = HashMap::<NodeId, NodeIndex>::new();
    let mut ids = HashMap::<NodeIndex, NodeId>::new();
    let mut im_graph = DiGraph::<NodeId, Weight>::new();

    // Add the focus node to the graph as the starting point
    let focus_node_index = im_graph.add_node(focus_id);
    indices.insert(focus_id, focus_node_index);
    ids.insert(focus_node_index, focus_id);

    Ok((ego_id, focus_id, im_graph, indices, ids))
  }

  pub fn edge_weight_normalized(
    &self,
    src: NodeId,
    dst: NodeId,
  ) -> Weight {
    log_trace!("{} {}", src, dst);

    let pos_sum = match self.mr.graph.get_node_data(src) {
      Some(x) => {
        if x.pos_sum < EPSILON {
          log_warning!(
            "Unable to normalize node weight, positive sum is zero."
          );
          1.0
        } else {
          x.pos_sum
        }
      },

      None => 1.0,
    };

    self
      .mr
      .graph
      .edge_weight(src, dst)
      .unwrap_or(None)
      .unwrap_or(0.0)
      / pos_sum
  }

  fn add_shortest_path_to_graph(
    &self,
    node_infos: &Vec<NodeInfo>,
    ego_id: NodeId,
    focus_id: NodeId,
    indices: &mut HashMap<NodeId, NodeIndex>,
    ids: &mut HashMap<NodeIndex, NodeId>,
    im_graph: &mut DiGraph<NodeId, Weight>,
  ) {
    log_trace!();

    let ego_to_focus =
      match perform_astar_search(&self.mr.graph, ego_id, focus_id) {
        Ok(path) => path,
        Err(_) => return,
      };

    let mut edges = Vec::<(NodeId, NodeId, Weight)>::new();
    edges.reserve_exact(ego_to_focus.len().saturating_sub(1));

    log_verbose!("Process shortest path.");

    for k in 0..ego_to_focus.len().saturating_sub(1) {
      let a = ego_to_focus[k];
      let b = ego_to_focus[k + 1];
      let a_info_opt = node_infos.get(a);
      let b_info_opt = node_infos.get(b);
      let a_b_weight = self.edge_weight_normalized(a, b);

      let a_kind_opt = match a_info_opt {
        Some(info) => Some(info.kind),
        None => None,
      };

      let b_kind_opt = match b_info_opt {
        Some(info) => Some(info.kind),
        None => None,
      };

      if k + 2 == ego_to_focus.len() {
        if a_kind_opt == Some(NodeKind::User) {
          edges.push((a, b, a_b_weight));
        } else {
          log_verbose!("Ignore node: {:?}", node_infos[a].name);
        }
      } else if b_kind_opt != Some(NodeKind::User) {
        log_verbose!("Ignore node: {:?}", node_infos[b].name);
        if k + 2 < ego_to_focus.len() {
          let c = ego_to_focus[k + 2];
          let b_c_weight = self.edge_weight_normalized(b, c);
          let a_c_weight = a_b_weight
            * b_c_weight
            * if a_b_weight < 0.0 && b_c_weight < 0.0 {
              -1.0
            } else {
              1.0
            };
          edges.push((a, c, a_c_weight));
        }
      } else if a_kind_opt == Some(NodeKind::User) {
        edges.push((a, b, a_b_weight));
      } else {
        log_verbose!("Ignore node: {:?}", node_infos[a].name);
      }
    }

    log_verbose!("Add path to the graph.");

    for (src, dst, weight) in edges {
      if let std::collections::hash_map::Entry::Vacant(e) = indices.entry(src) {
        let index = im_graph.add_node(src);
        e.insert(index);
        ids.insert(index, src);
      }
      add_edge_if_valid(im_graph, indices, ids, src, dst, weight);
    }
  }

  pub fn all_outbound_neighbors_normalized(
    &self,
    node: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{}", node);

    let mut v = vec![];

    match self.mr.graph.get_node_data(node) {
      None => {},
      Some(data) => {
        v.reserve_exact(data.pos_edges.len() + data.neg_edges.len());

        let abs_sum = if data.pos_sum < EPSILON {
          log_warning!(
            "Unable to normalize node weight, positive sum is zero."
          );
          1.0
        } else {
          data.abs_sum()
        };

        for x in &data.pos_edges {
          v.push((*x.0, *x.1 / abs_sum));
        }

        for x in &data.neg_edges {
          v.push((*x.0, -*x.1 / abs_sum));
        }
      },
    }
    v
  }

  fn add_focus_neighbor_connections(
    &self,
    focus_id: NodeId,
    im_graph: &mut DiGraph<NodeId, Weight>,
    indices: &mut HashMap<NodeId, NodeIndex>,
    ids: &mut HashMap<NodeIndex, NodeId>,
    node_infos: &Vec<NodeInfo>,
    positive_only: bool,
    focus_neighbors: &[(NodeId, Weight)],
  ) {
    log_trace!();

    for (dst_id, focus_dst_weight) in focus_neighbors.iter() {
      let dst_kind_opt = match node_infos.get(*dst_id) {
        Some(x) => Some(x.kind),
        None => None,
      };

      if positive_only && *focus_dst_weight <= 0.0 {
        continue;
      }

      if dst_kind_opt == Some(NodeKind::User) {
        add_edge_if_valid(
          im_graph,
          indices,
          ids,
          focus_id,
          *dst_id,
          *focus_dst_weight,
        );
      } else if dst_kind_opt == Some(NodeKind::Comment)
        || dst_kind_opt == Some(NodeKind::Beacon)
        || dst_kind_opt == Some(NodeKind::Opinion)
      {
        let dst_neighbors = self.all_outbound_neighbors_normalized(*dst_id);
        for (ngh_id, dst_ngh_weight) in dst_neighbors {
          if (positive_only && dst_ngh_weight <= 0.0)
            || ngh_id == focus_id
            || match node_infos.get(ngh_id) {
              Some(x) => Some(x.kind),
              None => None,
            } != Some(NodeKind::User)
          {
            continue;
          }
          let focus_ngh_weight = (*focus_dst_weight)
            * dst_ngh_weight
            * if *focus_dst_weight < 0.0 && dst_ngh_weight < 0.0 {
              -1.0
            } else {
              1.0
            };
          add_edge_if_valid(
            im_graph,
            indices,
            ids,
            focus_id,
            ngh_id,
            focus_ngh_weight,
          );
        }
      }
    }
  }

  fn fetch_score(
    &self,
    ego: NodeId,
    dst: NodeId,
  ) -> (NodeScore, NodeCluster) {
    self.apply_score_clustering(
      ego,
      self.fetch_raw_score(ego, dst),
      self.nodes.id_to_info[ego].kind,
    )
  }

  pub(crate) fn get_object_owner(
    &self,
    node: NodeId,
  ) -> Option<NodeId> {
    match self.nodes.id_to_info.get(node) {
      Some(info) => match info.owner {
        Some(id) => Some(id),
        None => Some(node),
      },
      None => Some(node),
    }
  }

  fn sort_paginate_and_format_graph_edges(
    &self,
    mut edge_ids: Vec<(NodeId, NodeId, Weight)>,
    ego_id: NodeId,
    index: u32,
    count: u32,
  ) -> Vec<GraphResult> {
    edge_ids.sort_by(|(_, _, a), (_, _, b)| b.abs().total_cmp(&a.abs()));

    edge_ids
      .into_iter()
      .skip(index as usize)
      .take(count as usize)
      .map(|(src_id, dst_id, weight_of_dst)| {
        let (score_value_of_dst, score_cluster_of_dst) =
          self.fetch_score(ego_id, dst_id);
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(dst_id) {
            Some(dst_owner_id) => self.fetch_score_cached(dst_owner_id, ego_id),
            None => (0.0, 0),
          };

        GraphResult {
          src:             self.nodes.id_to_info[src_id].name.clone(),
          dst:             self.nodes.id_to_info[dst_id].name.clone(),
          weight:          weight_of_dst,
          score:           score_value_of_dst,
          reverse_score:   score_value_of_ego,
          cluster:         score_cluster_of_dst,
          reverse_cluster: score_cluster_of_ego,
        }
      })
      .collect()
  }

  fn collect_all_edges(
    &self,
    indices: &HashMap<NodeId, NodeIndex>,
    ids: &HashMap<NodeIndex, NodeId>,
    im_graph: &DiGraph<NodeId, Weight>,
    ego_id: NodeId,
    index: u32,
    count: u32,
  ) -> Vec<GraphResult> {
    let unique_edges =
      extract_unique_edges_from_graph_data(indices, ids, im_graph);
    self.sort_paginate_and_format_graph_edges(
      unique_edges,
      ego_id,
      index,
      count,
    )
  }

  pub fn read_graph(
    &self,
    data: OpReadGraph,
  ) -> Vec<GraphResult> {
    log_command!("{:?}", data);

    let ego_str = &data.ego;
    let focus_str = &data.focus;
    let positive_only = data.positive_only;
    let index = data.index;
    let count = data.count;

    let (ego_id, focus_id, mut im_graph, mut indices, mut ids) =
      match self.validate_read_graph_params_and_setup(ego_str, focus_str) {
        Ok(data) => data,
        Err(msg) => {
          log_error!("{}", msg);
          return vec![];
        },
      };

    let node_infos = self.nodes.id_to_info.clone();
    let force_read_graph_conn = self.settings.force_read_graph_conn;

    if ego_id == focus_id {
      log_verbose!("Ego is same as focus");
    } else {
      self.add_shortest_path_to_graph(
        &node_infos,
        ego_id,
        focus_id,
        &mut indices,
        &mut ids,
        &mut im_graph,
      );
    }

    if force_read_graph_conn && !indices.contains_key(&ego_id) {
      add_edge_if_valid(
        &mut im_graph,
        &mut indices,
        &mut ids,
        ego_id,
        focus_id,
        1.0,
      );
    }

    let focus_neighbors = self.all_outbound_neighbors_normalized(focus_id);

    self.add_focus_neighbor_connections(
      focus_id,
      &mut im_graph,
      &mut indices,
      &mut ids,
      &node_infos,
      positive_only,
      &focus_neighbors,
    );

    remove_self_references_from_im_graph(&mut im_graph, &indices);

    let edges = self.collect_all_edges(
      &indices,
      &ids,
      &im_graph,
      ego_id,
      index as u32,
      count as u32,
    );

    edges
  }

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

    let ego_id = ego_info.id;

    let ranks = self.fetch_all_scores(&ego_info);
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

  fn apply_filters_and_pagination(
    &self,
    scores: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    filter_options: &FilterOptions,
    prioritize_ego_owned_nodes: bool,
  ) -> Vec<ScoreResult> {
    let mut filtered_sorted_scores =
      filter_and_sort_scores(scores, ego_info, filter_options);

    if prioritize_ego_owned_nodes {
      prioritize_ego_owned_items(&mut filtered_sorted_scores, ego_info);
    }

    self.paginate_and_format_items(
      filtered_sorted_scores,
      ego_info,
      filter_options.index,
      filter_options.count,
    )
  }

  fn paginate_and_format_items(
    &self,
    items: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    index: u32,
    count: u32,
  ) -> Vec<ScoreResult> {
    let start = index as usize;
    let end = (index + count) as usize;

    items[start..end.min(items.len())]
      .iter()
      .map(|(target_info, score, cluster)| {
        let (reverse_score, reverse_cluster) =
          match self.get_object_owner(target_info.id) {
            Some(owner_id) => self.fetch_score_cached(owner_id, ego_info.id),
            None => (0.0, 0),
          };
        ScoreResult {
          ego: ego_info.name.clone(),
          target: target_info.name.clone(),
          score: *score,
          reverse_score,
          cluster: *cluster,
          reverse_cluster,
        }
      })
      .collect()
  }

  pub fn fetch_score_cached(
    &self,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> (NodeScore, NodeCluster) {
    log_trace!("{} {}", dst_id, ego_id);

    let score = match self.cached_scores.get(&(ego_id, dst_id)) {
      Some(score) => self.with_zero_opinion(dst_id, score),
      None => self.fetch_raw_score(ego_id, dst_id),
    };

    let kind_opt = self
      .nodes
      .get_by_id(dst_id)
      .and_then(|node_info| Some(node_info.kind));

    if let Some(kind) = kind_opt {
      self.apply_score_clustering(ego_id, score, kind)
    } else {
      (score, 0) // Default cluster if kind is None
    }
  }

  fn fetch_all_scores(
    &self,
    ego_info: &NodeInfo,
  ) -> Vec<(NodeInfo, NodeScore, NodeCluster)> {
    log_trace!("{}", ego_info.id);
    self
      .fetch_all_raw_scores(ego_info.id, self.settings.zero_opinion_factor)
      .iter()
      .filter_map(|(dst_id, score)| {
        self.nodes.get_by_id(*dst_id).map(|node_info| {
          let cluster = self
            .apply_score_clustering(ego_info.id, *score, node_info.kind)
            .1;
          (node_info.clone(), *score, cluster)
        })
      })
      .collect()
  }

  pub fn with_zero_opinion(
    &self,
    dst_id: NodeId,
    score: NodeScore,
  ) -> NodeScore {
    log_trace!("{} {}", dst_id, score);

    let zero_score = match self.zero_opinion.get(dst_id) {
      Some(x) => *x,
      _ => 0.0,
    };
    let k = self.settings.zero_opinion_factor;
    score * (1.0 - k) + k * zero_score
  }

  fn with_zero_opinions(
    &self,
    scores: Vec<(NodeId, NodeScore)>,
  ) -> Vec<(NodeId, NodeScore)> {
    let k = self.settings.zero_opinion_factor;

    let mut res: Vec<(NodeId, NodeScore)> = vec![];
    res.resize(self.zero_opinion.len(), (0, 0.0));

    for (id, zero_score) in self.zero_opinion.iter().enumerate() {
      res[id] = (id, zero_score * k);
    }

    for (id, score) in scores.iter() {
      if *id >= res.len() {
        let n = res.len();
        res.resize(id + 1, (0, 0.0));
        for id in n..res.len() {
          res[id].0 = id;
        }
      }
      res[*id].1 += (1.0 - k) * score;
    }

    res
      .into_iter()
      .filter(|(_id, score)| *score != 0.0)
      .collect::<Vec<_>>()
  }

  pub fn fetch_raw_score(
    &self,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> NodeScore {
    log_trace!("{} {} {}", ego_id, dst_id, self.settings.num_walks);

    match self.mr.get_node_score(ego_id, dst_id) {
      Ok(score) => {
        self.cached_scores.insert((ego_id, dst_id), score);
        self.with_zero_opinion(dst_id, score)
      },
      Err(e) => {
        log_error!("Failed to get node score: {}", e);
        0.0
      },
    }
  }

  fn fetch_all_raw_scores(
    &self,
    ego_id: NodeId,
    zero_opinion_factor: f64,
  ) -> Vec<(NodeId, NodeScore)> {
    log_trace!(
      "{} {} {}",
      ego_id,
      self.settings.num_walks,
      zero_opinion_factor
    );

    match self.mr.get_all_scores(ego_id, None) {
      Ok(scores) => {
        for (dst_id, score) in &scores {
          self.cached_scores.insert((ego_id, *dst_id), *score);
        }
        let scores = self.with_zero_opinions(scores);

        // Filter out nodes that have a direct negative edge from ego
        if self.settings.omit_neg_edges_scores {
          scores
            .into_iter()
            .filter(|(dst_id, _)| {
              // Check if there's a direct edge and if it's negative
              match self.mr.graph.edge_weight(ego_id, *dst_id) {
                Ok(Some(weight)) => weight > 0.0, // Keep only positive edges
                _ => true, // Keep if no direct edge exists
              }
            })
            .collect()
        } else {
          scores
        }
      },
      Err(e) => {
        log_error!("{}", e);
        vec![]
      },
    }
  }

  fn set_edge_by_id(
    &mut self,
    src_id: NodeId,
    dst_id: NodeId,
    amount: Weight,
    magnitude: Magnitude,
  ) {
    log_trace!();

    let (
      new_weight_scaled,
      mut new_min_weight, // This will be potentially updated by the helper
      new_max_weight,
      new_mag_scale,
      rescale_factor,
    ) = self.vsids.scale_weight(src_id, amount, magnitude);

    let edge_deletion_threshold = new_max_weight * self.vsids.deletion_ratio;
    // let can_delete_at_least_one_edge = new_min_weight <= edge_deletion_threshold;
    let must_rescale = rescale_factor > 1.0;

    //  FIXME: This condition doesn't allow to create new edges at all.
    // if can_delete_at_least_one_edge || must_rescale {
    new_min_weight = self.apply_edge_rescales_and_deletions(
      src_id,
      new_min_weight, // Pass current new_min_weight
      edge_deletion_threshold,
      rescale_factor,
      must_rescale,
    );

    match self.mr.set_edge(src_id, dst_id, new_weight_scaled) {
      Ok(_) => {},
      Err(e) => {
        log_error!("{}", e);
      },
    };

    if must_rescale {
      log_verbose!(
        "Rescale performed: src={}, dst={}, normalized_new_weight={}",
        src_id,
        dst_id,
        new_weight_scaled
      );
    } else {
      log_verbose!(
        "Edge updated without rescale: src={}, dst={}, new_weight_scaled={}",
        src_id,
        dst_id,
        new_weight_scaled
      );
    }
    self
      .vsids
      .min_max_weights
      .insert(src_id, (new_min_weight, new_max_weight, new_mag_scale));
    // }
  }

  fn apply_edge_rescales_and_deletions(
    &mut self,
    src_id: NodeId,
    current_min_weight: Weight,
    edge_deletion_threshold: Weight,
    rescale_factor: f64,
    must_rescale: bool,
  ) -> Weight {
    let node_data = match self.mr.graph.get_node_data(src_id) {
      Some(x) => x,
      None => {
        log_error!("Unable to get node data.");
        return 0.0;
      },
    };

    let (edges_to_modify, new_min_weight_from_scan) =
      node_data.get_outgoing_edges().fold(
        (Vec::new(), current_min_weight), // Use passed current_min_weight
        |(mut to_modify, min), (dest, weight)| {
          let abs_weight = if must_rescale {
            weight.abs() / rescale_factor
          } else {
            weight.abs()
          };

          if abs_weight <= edge_deletion_threshold {
            to_modify.push((dest, 0.0));
            (to_modify, min)
          } else {
            if must_rescale {
              to_modify.push((dest, weight / rescale_factor));
            }
            // If not must_rescale, but we are in this block, it implies can_delete_at_least_one_edge is true.
            // Edges that are not rescaled and not deleted are not added to `edges_to_modify`.
            // This preserves the original logic where only edges needing change (deletion or rescale) are processed.
            (to_modify, min.min(abs_weight))
          }
        },
      );

    for (dst_id_iter, weight_iter) in edges_to_modify {
      log_verbose!(
        "Rescale or delete edge: src={}, dst={}, new_weight={}",
        src_id,
        dst_id_iter,
        weight_iter
      );
      match self.mr.set_edge(src_id, dst_id_iter, weight_iter) {
        Ok(_) => {},
        Err(e) => {
          log_error!("{}", e);
        },
      };
    }
    new_min_weight_from_scan // Return the updated min_weight
  }

  pub fn set_edge(
    &mut self,
    src: NodeName,
    dst: NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) {
    log_trace!("{:?} {:?} {}", src, dst, amount);

    match self.reg_owner_and_get_ids(src.clone(), dst.clone()) {
      Ok((src_id, dst_id)) => {
        self.set_edge_by_id(src_id, dst_id, amount, magnitude);

        //  D1 (JOURNAL): auto-calculate for nodes that have never had walks
        //  initialised. This ensures read_scores works immediately after a
        //  put_edge + sync even when no explicit WriteCalculate was sent (new
        //  TCP path). For nodes that already have walks, meritrank_core's
        //  incremental set_edge_ update is sufficient.
        if !self.mr.get_personal_hits().contains_key(&src_id) {
          self.calculate(src);
        }
        if !self.mr.get_personal_hits().contains_key(&dst_id) {
          self.calculate(dst);
        }
      },
      Err(e) => match e {
        AugGraphError::SelfReference => {
          log_error!("Self-reference is not allowed.")
        },
        AugGraphError::IncorrectNodeKinds(s, d) => {
          log_error!("Incorrect node kinds combination {} -> {}.", s, d)
        },
      },
    }
  }

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

    let ego_id = self.nodes.register(&mut self.mr, ego, kind);

    match self.mr.calculate(ego_id, self.settings.num_walks) {
      Ok(_) => {},
      Err(e) => log_error!("{}", e),
    };
  }

  pub fn reduced_graph(
    &mut self,
    infos: &[NodeInfo],
    num_walks: usize,
  ) -> Vec<(NodeId, NodeId, Weight)> {
    log_trace!();

    let mut all_edges = vec![];

    for src in 0..infos.len() {
      if let Some(data) = self.mr.graph.get_node_data(src) {
        for (dst, _) in &data.pos_edges {
          all_edges.push((src, *dst));
        }

        for (dst, _) in &data.neg_edges {
          all_edges.push((src, *dst));
        }
      }
    }

    let users: Vec<NodeId> = infos
      .iter()
      .enumerate()
      .filter(|(_id, info)| info.kind == NodeKind::User)
      .filter(|(id, _info)| {
        for (src, dst) in &all_edges {
          if *id == *src || *id == *dst {
            return true;
          }
        }
        false
      })
      .map(|(id, _info)| id)
      .collect();

    if users.is_empty() {
      return vec![];
    }

    for id in users.iter() {
      match self.mr.calculate(*id, num_walks) {
        Ok(_) => {},
        Err(e) => log_error!("{}", e),
      };
    }

    let edges: Vec<(NodeId, NodeId, Weight)> = users
      .into_iter()
      .flat_map(|id| -> Vec<(NodeId, NodeId, Weight)> {
        self
          .fetch_all_raw_scores(id, 0.0)
          .into_iter()
          .map(|(node_id, score)| (id, node_id, score))
          .filter(|(ego_id, node_id, score)| {
            let kind = infos[*node_id].kind;

            (kind == NodeKind::User || kind == NodeKind::Beacon)
              && *score > EPSILON
              && ego_id != node_id
          })
          .collect()
      })
      .collect();

    let result: Vec<(NodeId, NodeId, Weight)> = edges
      .into_iter()
      .map(|(ego_id, dst_id, weight)| {
        let ego_kind = infos[ego_id].kind;
        let dst_kind = infos[dst_id].kind;
        (ego_id, ego_kind, dst_id, dst_kind, weight)
      })
      .filter(|(ego_id, ego_kind_opt, dst_id, dst_kind_opt, _)| {
        ego_id != dst_id
          && *ego_kind_opt == NodeKind::User
          && (*dst_kind_opt == NodeKind::User
            || *dst_kind_opt == NodeKind::Beacon)
      })
      .map(|(ego_id, _, dst_id, _, weight)| (ego_id, dst_id, weight))
      .collect();

    result
  }

  pub fn top_nodes(
    &mut self,
    infos: &[NodeInfo],
    top_nodes_limit: usize,
    num_walks: usize,
  ) -> Vec<(NodeId, f64)> {
    log_trace!();

    let reduced = self.reduced_graph(infos, num_walks);

    if reduced.is_empty() {
      log_error!("Reduced graph is empty");
      return vec![];
    }

    // TODO: remove PageRank in favor of direct sum of scores
    // Actually, instead of calculating page rank it is
    // possible to just sum all the scores by each user for each other.
    // The result should be the same after normalization.
    let mut pr = Pagerank::<NodeId>::new();

    reduced
      .iter()
      .filter(|(_src, _dst, weight)| !(*weight > -EPSILON && *weight < EPSILON))
      .for_each(|(src, dst, _weight)| {
        pr.add_edge(*src, *dst);
      });

    log_verbose!("Calculate page rank");
    pr.calculate();

    let (nodes, scores): (Vec<NodeId>, Vec<f64>) = pr
        .nodes()  // already sorted by score
        .into_iter()
        .take(top_nodes_limit)
        .unzip();

    let res = nodes.into_iter().zip(scores).collect::<Vec<_>>();

    if res.is_empty() {
      log_error!("No top nodes");
    }

    res
  }

  pub fn recalculate_all_users(
    &mut self,
    infos: &[NodeInfo],
    num_walk: usize,
  ) {
    log_trace!("{}", num_walk);

    for id in 0..infos.len() {
      if (id % 100) == 90 {
        log_verbose!("{}%", (id * 100) / infos.len());
      }
      if infos[id].kind == NodeKind::User {
        match self.mr.calculate(id, num_walk) {
          Ok(_) => {},
          Err(e) => log_error!("{}", e),
        };
      }
    }
  }

  fn recalculate_zero(&mut self) {
    log_command!();

    let infos = self.nodes.id_to_info.clone();

    //  Save the current state of the graph
    let data_bak = self.mr.clone();

    self.recalculate_all_users(&infos, 0);
    let nodes = self.top_nodes(
      &infos,
      self.settings.top_nodes_limit,
      self.settings.zero_opinion_num_walks,
    );

    //  Drop all walks and make sure to empty caches.
    self.recalculate_all_users(&infos, 0);
    self.cached_scores = Cache::new(self.settings.scores_cache_size as u64);
    // self.cached_walks = LruCache::new(self.settings.walks_cache_size);

    self.zero_opinion = vec![];
    self.zero_opinion.reserve(nodes.len());

    for (node_id, amount) in nodes.iter() {
      if *node_id >= self.zero_opinion.len() {
        self.zero_opinion.resize(*node_id + 1, 0.0);
      }
      self.zero_opinion[*node_id] = *amount;
    }

    //  Reset the graph
    self.mr = data_bak;
  }

  fn reg_owner_and_get_ids(
    &mut self,
    src: NodeName,
    dst: NodeName,
  ) -> Result<(NodeId, NodeId), AugGraphError> {
    if src == dst {
      return Err(AugGraphError::SelfReference);
    }

    let opt_src_kind = node_kind_from_prefix(&src);
    let opt_dst_kind = node_kind_from_prefix(&dst);

    match (opt_src_kind, opt_dst_kind) {
      (Some(NodeKind::User), Some(NodeKind::User)) => {
        let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
        let dst_id = self.nodes.register(&mut self.mr, dst, NodeKind::User);
        Ok((src_id, dst_id))
      },
      (Some(src_kind), Some(NodeKind::User)) => {
        let dst_id = self.nodes.register(&mut self.mr, dst, NodeKind::User);
        let src_id =
          self
            .nodes
            .register_with_owner(&mut self.mr, src, src_kind, dst_id);
        Ok((src_id, dst_id))
      },
      (Some(NodeKind::User), Some(dst_kind)) => {
        let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
        let dst_id = self.nodes.register(&mut self.mr, dst, dst_kind);
        Ok((src_id, dst_id))
      },
      (Some(_), Some(_)) => Err(AugGraphError::IncorrectNodeKinds(src, dst)),
      _ => Err(AugGraphError::IncorrectNodeKinds(src, dst)),
    }
  }
}

impl Absorb<AugGraphOp> for AugGraph {
  fn absorb_first(
    &mut self,
    op: &mut AugGraphOp,
    _: &Self,
  ) {
    log_command!("{:?}", op);

    //  FIXME: Pass strings by reference, no clones!

    match op {
      AugGraphOp::WriteReset => {
        // NOTE: This doesn't actually get called, because reset
        //       is implemented on the multi-graph level.
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
        //  D2 (JOURNAL): zero all outgoing edges from the node.
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

  fn sync_with(
    &mut self,
    first: &Self,
  ) {
    *self = first.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use meritrank_core::Graph;

  #[test]
  fn node_registry() {
    let mut mr = MeritRank::new(Graph::new());

    let mut registry = NodeRegistry::new();

    let user_id =
      registry.register(&mut mr, "Alice".to_string(), NodeKind::User);
    assert_eq!(user_id, 0);

    let comment_id = registry.register_with_owner(
      &mut mr,
      "Comment1".to_string(),
      NodeKind::Comment,
      user_id,
    );
    assert_eq!(comment_id, 1);

    // Test get_by_id
    let info = registry.get_by_id(0).unwrap();
    assert_eq!(info.name, "Alice");
    assert_eq!(info.kind, NodeKind::User);
    assert_eq!(info.owner, None);

    // Test get_by_name
    let info = registry.get_by_name("Comment1").unwrap();
    assert_eq!(info.id, 1);
    assert_eq!(info.kind, NodeKind::Comment);
    assert_eq!(info.owner, Some(user_id));

    // Test registering an existing name
    let existing_id =
      registry.register(&mut mr, "Alice".to_string(), NodeKind::User);
    assert_eq!(existing_id, 0);

    // Test non-existent entries
    assert_eq!(registry.get_by_id(2), None);
    assert_eq!(registry.get_by_name("Bob"), None);
  }

  #[test]
  fn ownership_assigned_on_nonuser_to_user_edge() {
    let mut aug = AugGraph::new(Settings::default());
    aug.set_edge("O1".into(), "U1".into(), 1.0, 0);

    let o1 = aug.nodes.get_by_name("O1").unwrap();
    let u1 = aug.nodes.get_by_name("U1").unwrap();
    assert_eq!(o1.kind, NodeKind::Opinion);
    assert_eq!(o1.owner, Some(u1.id));
    assert_eq!(u1.kind, NodeKind::User);
    assert_eq!(u1.owner, None);

    assert_eq!(aug.get_object_owner(o1.id), Some(u1.id));
    assert_eq!(aug.get_object_owner(u1.id), Some(u1.id));
  }

  #[test]
  fn ownership_stable_across_subsequent_edges() {
    let mut aug = AugGraph::new(Settings::default());
    aug.set_edge("O1".into(), "U1".into(), 1.0, 0);
    aug.set_edge("O1".into(), "U2".into(), 1.0, 0);

    let o1 = aug.nodes.get_by_name("O1").unwrap();
    let u1 = aug.nodes.get_by_name("U1").unwrap();
    let u2 = aug.nodes.get_by_name("U2").unwrap();
    assert_eq!(o1.owner, Some(u1.id));
    assert_eq!(u2.owner, None);
  }

  // ================================================================
  // Ported legacy tests from service/src/legacy/tests.rs
  // ================================================================

  fn default_graph() -> AugGraph {
    AugGraph::new(Settings {
      num_walks:              50,
      zero_opinion_num_walks: 100,
      zero_opinion_factor:    0.0,
      ..Settings::default()
    })
  }

  fn default_graph_zero() -> AugGraph {
    AugGraph::new(Settings {
      num_walks:              50,
      zero_opinion_num_walks: 50,
      ..Settings::default()
    })
  }

  fn read_scores(
    graph: &AugGraph,
    ego: &str,
    kind_prefix: &str,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<ScoreResult> {
    let node_kind = node_kind_from_prefix(kind_prefix);
    graph.read_scores(OpReadScores {
      ego:           ego.into(),
      score_options: FilterOptions {
        node_kind,
        hide_personal,
        score_lt,
        score_lte,
        score_gt,
        score_gte,
        index,
        count,
      },
    })
  }

  fn read_node_score_helper(
    graph: &AugGraph,
    ego: &str,
    target: &str,
  ) -> Vec<ScoreResult> {
    graph.read_node_score(OpReadNodeScore {
      ego:    ego.into(),
      target: target.into(),
    })
  }

  fn read_graph_helper(
    graph: &AugGraph,
    ego: &str,
    focus: &str,
    positive_only: bool,
    index: u64,
    count: u64,
  ) -> Vec<GraphResult> {
    graph.read_graph(OpReadGraph {
      ego: ego.into(),
      focus: focus.into(),
      positive_only,
      index,
      count,
    })
  }

  fn read_mutual_scores_helper(
    graph: &AugGraph,
    ego: &str,
  ) -> Vec<ScoreResult> {
    graph.read_mutual_scores(OpReadMutualScores {
      ego: ego.into(),
    })
  }

  fn read_neighbors_helper(
    graph: &AugGraph,
    ego: &str,
    focus: &str,
    direction: i64,
    kind_str: &str,
    hide_personal: bool,
    lt: f64,
    lte: bool,
    gt: f64,
    gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<ScoreResult> {
    graph.read_neighbors(OpReadNeighbors {
      ego: ego.into(),
      focus: focus.into(),
      direction,
      kind: node_kind_from_prefix(kind_str),
      hide_personal,
      lt,
      lte,
      gt,
      gte,
      index,
      count,
    })
  }

  // --- Score tests ---

  #[test]
  fn scores_uncontexted() {
    let mut graph = AugGraph::new(Settings {
      num_walks:              500,
      zero_opinion_num_walks: 100,
      zero_opinion_factor:    0.0,
      ..Settings::default()
    });

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

    let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

    assert_eq!(res.len(), 3);

    for x in &res {
      assert_eq!(x.ego, "U1");
      match x.target.as_str() {
        "U1" => {
          assert!(x.score > 0.2);
          assert!(x.score < 0.5);
        },
        "U2" => {
          assert!(x.score > 0.18);
          assert!(x.score < 0.5);
        },
        "U3" => {
          assert!(x.score > 0.2);
          assert!(x.score < 0.5);
        },
        _ => panic!("Unexpected target: {}", x.target),
      }
    }
  }

  #[test]
  fn scores_reversed() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);
    graph.set_edge("U2".into(), "U1".into(), 4.0, 0);
    graph.set_edge("U3".into(), "U1".into(), -5.0, 0);

    let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

    assert!(res.len() >= 2);
    assert!(res.len() <= 3);

    for x in &res {
      assert_eq!(x.ego, "U1");
      match x.target.as_str() {
        "U1" => {
          assert!(x.score > 0.0);
          assert!(x.score < 0.4);
          assert!(x.reverse_score > 0.0);
          assert!(x.reverse_score < 0.4);
        },
        "U2" => {
          assert!(x.score > -0.1);
          assert!(x.score < 0.3);
          assert!(x.reverse_score > -0.3);
          assert!(x.reverse_score < 0.1);
        },
        "U3" => {
          assert!(x.score > -0.1);
          assert!(x.score < 0.3);
          assert!(x.reverse_score > -0.6);
          assert!(x.reverse_score < 0.0);
        },
        _ => panic!("Unexpected target: {}", x.target),
      }
    }
  }

  #[test]
  fn scores_sort_order() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

    let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

    assert!(res.len() > 1);
    for n in 1..res.len() {
      assert!(res[n - 1].score.abs() >= res[n].score.abs());
    }
  }

  #[test]
  fn scores_without_recalculate() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
    graph.set_edge("U1".into(), "U0".into(), 1.0, 0);

    let res = read_scores(&graph, "U1", "U", true, 100.0, false, -100.0, false, 0, u32::MAX);
    let n = res.len();
    assert_eq!(n, 3);
  }

  #[test]
  fn scores_self() {
    let mut graph = default_graph();

    graph.set_edge("B1".into(), "U1".into(), 3.0, 0);

    let res = read_scores(&graph, "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].ego, "U1");
    assert_eq!(res[0].target, "U1");
    assert!(res[0].score > 0.999);
    assert!(res[0].score < 1.001);
  }

  // --- Node score tests ---

  #[test]
  fn node_score_uncontexted() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U3".into(), "U2".into(), 3.0, 0);

    let res = read_node_score_helper(&graph, "U1", "U2");

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].ego, "U1");
    assert_eq!(res[0].target, "U2");
    assert!(res[0].score > 0.3);
    assert!(res[0].score < 0.45);
  }

  #[test]
  fn node_score_reversed() {
    let mut graph = default_graph_zero();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U3".into(), "U2".into(), 3.0, 0);
    graph.set_edge("U2".into(), "U1".into(), 4.0, 0);

    let res = read_node_score_helper(&graph, "U1", "U2");

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].ego, "U1");
    assert_eq!(res[0].target, "U2");
    assert!(res[0].score > 0.2);
    assert!(res[0].score < 0.4);
    assert!(res[0].reverse_score > 0.2);
    assert!(res[0].reverse_score < 0.4);
  }

  // --- Mutual score tests ---

  #[test]
  fn mutual_scores_uncontexted() {
    let mut graph = default_graph_zero();

    graph.set_edge("U1".into(), "U2".into(), 3.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U1".into(), 2.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 4.0, 0);
    graph.set_edge("U3".into(), "U1".into(), 3.0, 0);
    graph.set_edge("U3".into(), "U2".into(), 2.0, 0);

    let res = read_mutual_scores_helper(&graph, "U1");

    assert_eq!(res.len(), 3);

    let mut u1 = true;
    let mut u2 = true;
    let mut u3 = true;

    for x in &res {
      assert_eq!(x.ego, "U1");
      match x.target.as_str() {
        "U1" => {
          assert!(x.score > 0.25);
          assert!(x.score < 0.5);
          assert!(x.reverse_score > 0.25);
          assert!(x.reverse_score < 0.5);
          assert!(u1);
          u1 = false;
        },
        "U2" => {
          assert!(x.score > 0.15);
          assert!(x.score < 0.35);
          assert!(x.reverse_score > 0.15);
          assert!(x.reverse_score < 0.35);
          assert!(u2);
          u2 = false;
        },
        "U3" => {
          assert!(x.score > 0.15);
          assert!(x.score < 0.35);
          assert!(x.reverse_score > 0.15);
          assert!(x.reverse_score < 0.35);
          assert!(u3);
          u3 = false;
        },
        _ => panic!("Unexpected target"),
      }
    }
  }

  #[test]
  fn mutual_scores_self() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 3.0, 0);

    let ego_id = graph.nodes.get_by_name("U1").unwrap().id;
    let dst_id = graph.nodes.get_by_name("U2").unwrap().id;
    graph.mr.set_edge(ego_id, dst_id, 0.0).unwrap();

    let res = read_mutual_scores_helper(&graph, "U1");

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].ego, "U1");
    assert_eq!(res[0].target, "U1");
    assert!(res[0].score > 0.99);
    assert!(res[0].score < 1.01);
    assert!(res[0].reverse_score > 0.99);
    assert!(res[0].reverse_score < 1.01);
  }

  #[test]
  fn mutual_scores_cluster_single_score_uncontexted() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 10.0, 0);

    let res = read_mutual_scores_helper(&graph, "U1");

    assert_eq!(res.len(), 2);
    assert_eq!(res[0].cluster, 100);
    assert_eq!(res[1].cluster, 1);
  }

  // --- Graph tests ---

  #[test]
  fn graph_uncontexted() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

    let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

    assert_eq!(res.len(), 2);

    let mut has_u1 = false;
    let mut has_u2 = false;

    for x in &res {
      match x.src.as_str() {
        "U1" => {
          assert_eq!(x.dst, "U2");
          assert!(x.weight > 0.65);
          assert!(x.weight < 0.67);
          has_u1 = true;
        },
        "U2" => {
          assert_eq!(x.dst, "U3");
          assert!(x.weight > 0.99);
          assert!(x.weight < 1.01);
          has_u2 = true;
        },
        _ => panic!("Unexpected src: {}", x.src),
      }
    }

    assert!(has_u1);
    assert!(has_u2);
  }

  #[test]
  fn graph_reversed() {
    let mut graph = default_graph_zero();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);
    graph.set_edge("U2".into(), "U1".into(), 4.0, 0);

    let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

    assert_eq!(res.len(), 3);

    for x in &res {
      match x.src.as_str() {
        "U1" => {
          assert_eq!(x.dst, "U2");
          assert!(x.weight > 0.6);
          assert!(x.weight < 0.7);
          assert!(x.score > 0.05);
          assert!(x.score < 0.4);
        },
        "U2" => {
          if x.dst == "U1" {
            assert!(x.weight > 0.5);
            assert!(x.weight < 0.6);
            assert!(x.score > 0.2);
            assert!(x.score < 0.5);
          }
          if x.dst == "U3" {
            assert!(x.weight > 0.39);
            assert!(x.weight < 0.49);
            assert!(x.score > 0.16);
            assert!(x.score < 0.4);
          }
        },
        _ => panic!("Unexpected src: {}", x.src),
      }
    }
  }

  #[test]
  fn graph_sort_order() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

    let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

    assert!(res.len() > 1);
    for n in 1..res.len() {
      assert!(res[n - 1].weight.abs() >= res[n].weight.abs());
    }
  }

  #[test]
  fn graph_empty() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);

    let ego_id = graph.nodes.get_by_name("U1").unwrap().id;
    let u2_id = graph.nodes.get_by_name("U2").unwrap().id;
    let u3_id = graph.nodes.get_by_name("U3").unwrap().id;
    graph.mr.set_edge(ego_id, u2_id, 0.0).unwrap();
    graph.mr.set_edge(ego_id, u3_id, 0.0).unwrap();
    graph.mr.set_edge(u2_id, u3_id, 0.0).unwrap();

    let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);
    assert_eq!(res.len(), 0);
  }

  #[test]
  fn graph_no_direct_connectivity() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "B1".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "B2".into(), 1.0, 0);

    let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].src, "U2");
    assert_eq!(res[0].dst, "U3");
  }

  #[test]
  fn graph_force_connectivity() {
    let mut graph = AugGraph::new(Settings {
      num_walks:              100,
      zero_opinion_num_walks: 100,
      force_read_graph_conn:  true,
      ..Settings::default()
    });

    graph.set_edge("U1".into(), "B1".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U2".into(), "B2".into(), 1.0, 0);

    let res = read_graph_helper(&graph, "U1", "U2", false, 0, 10000);

    assert_eq!(res.len(), 2);
    assert_eq!(res[0].src, "U1");
    assert_eq!(res[0].dst, "U2");
    assert_eq!(res[1].src, "U2");
    assert_eq!(res[1].dst, "U3");
  }

  // --- Clustering tests ---

  #[test]
  fn five_user_scores_clustering() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 5.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 2.5, 0);
    graph.set_edge("U1".into(), "U4".into(), 2.0, 0);
    graph.set_edge("U1".into(), "U5".into(), 3.0, 0);
    graph.set_edge("U2".into(), "U1".into(), 4.0, 0);

    let res = read_scores(&graph, "U1", "", true, 100.0, false, -100.0, false, 0, u32::MAX);

    assert_eq!(res.len(), 5);

    assert!(res[0].cluster <= 100);
    assert!(res[0].cluster >= 40);

    assert!(res[1].cluster <= 100);
    assert!(res[1].cluster >= 20);

    assert!(res[2].cluster <= 100);
    assert!(res[2].cluster >= 1);

    assert!(res[3].cluster <= 80);
    assert!(res[3].cluster >= 1);

    assert!(res[4].cluster <= 60);
    assert!(res[4].cluster >= 1);
  }

  #[test]
  fn five_beacon_scores_clustering() {
    let mut graph = AugGraph::new(Settings {
      num_walks: 500,
      ..Settings::default()
    });

    graph.set_edge("U1".into(), "B2".into(), 5.0, 0);
    graph.set_edge("U1".into(), "B3".into(), 1.0, 0);
    graph.set_edge("U1".into(), "B4".into(), 2.0, 0);
    graph.set_edge("U1".into(), "B5".into(), 3.0, 0);
    graph.set_edge("U1".into(), "B6".into(), 4.0, 0);

    let res = read_scores(&graph, "U1", "B", true, 100.0, false, -100.0, false, 0, u32::MAX);

    assert_eq!(res.len(), 5);

    assert!(res[0].cluster <= 100);
    assert!(res[0].cluster >= 40);

    assert!(res[1].cluster <= 100);
    assert!(res[1].cluster >= 20);

    assert!(res[2].cluster <= 100);
    assert!(res[2].cluster >= 1);

    assert!(res[3].cluster <= 80);
    assert!(res[3].cluster >= 1);

    assert!(res[4].cluster <= 60);
    assert!(res[4].cluster >= 1);
  }

  #[test]
  fn three_scores_chain_clustering() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 3.0, 0);
    graph.set_edge("U3".into(), "U1".into(), 4.0, 0);

    let res = read_scores(&graph, "U1", "", true, 100.0, false, -100.0, false, 0, u32::MAX);

    assert_eq!(res.len(), 3);

    assert!(res[0].cluster <= 100);
    assert!(res[0].cluster >= 40);

    assert!(res[1].cluster <= 80);
    assert!(res[1].cluster >= 20);

    assert!(res[2].cluster <= 60);
    assert!(res[2].cluster >= 1);
  }

  #[test]
  fn separate_clusters_without_users() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "B1".into(), 3.0, 0);
    graph.set_edge("U1".into(), "C1".into(), 4.0, 0);

    let res = read_scores(&graph, "U1", "", true, 100.0, false, -100.0, false, 0, u32::MAX);

    assert_eq!(res.len(), 3);

    assert_eq!(res[0].cluster, 100);
    assert_eq!(res[1].cluster, 100);
    assert_eq!(res[2].cluster, 100);
  }

  #[test]
  fn separate_clusters_self_score() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 2.0, 0);
    graph.set_edge("U1".into(), "B1".into(), 3.0, 0);
    graph.set_edge("U1".into(), "C1".into(), 4.0, 0);

    let res = read_scores(&graph, "U1", "U", true, 100.0, false, -100.0, false, 0, u32::MAX);

    assert_eq!(res.len(), 2);

    assert_eq!(res[0].cluster, 100);
    assert_eq!(res[1].cluster, 1);
  }

  // --- Neighbor tests ---

  #[test]
  fn neighbors_all() {
    let mut graph = default_graph();
    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 2.0, 0);
    graph.set_edge("U3".into(), "U1".into(), 3.0, 0);

    let neighbors = read_neighbors_helper(
      &graph, "U1", "U2", NEIGHBORS_ALL, "", false, 100.0, false, -100.0,
      false, 0, 100,
    );

    assert_eq!(neighbors.len(), 2);
    let targets: Vec<&str> = neighbors.iter().map(|n| n.target.as_str()).collect();
    assert!(targets.contains(&"U1"));
    assert!(targets.contains(&"U3"));
  }

  #[test]
  fn neighbors_inbound() {
    let mut graph = default_graph();
    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 2.0, 0);
    graph.set_edge("U3".into(), "U1".into(), 3.0, 0);

    let neighbors = read_neighbors_helper(
      &graph, "U1", "U2", NEIGHBORS_INBOUND, "", false, 100.0, false, -100.0,
      false, 0, 100,
    );

    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].target, "U1");
  }

  #[test]
  fn neighbors_outbound() {
    let mut graph = default_graph();
    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 2.0, 0);
    graph.set_edge("U3".into(), "U1".into(), 3.0, 0);

    let neighbors = read_neighbors_helper(
      &graph, "U1", "U2", NEIGHBORS_OUTBOUND, "", false, 100.0, false, -100.0,
      false, 0, 100,
    );

    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].target, "U3");
  }

  #[test]
  fn neighbors_non_ego_score() {
    let mut graph = AugGraph::new(Settings {
      num_walks:              500,
      zero_opinion_num_walks: 100,
      zero_opinion_factor:    0.0,
      ..Settings::default()
    });

    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U1".into(), "U4".into(), 1.0, 0);
    graph.set_edge("U4".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U3".into(), "U4".into(), 1.0, 0);

    let neighbors = read_neighbors_helper(
      &graph, "U1", "U3", NEIGHBORS_INBOUND, "", false, 100.0, false, -100.0,
      false, 0, 100,
    );

    assert_eq!(neighbors.len(), 2);
    let targets: Vec<&str> = neighbors.iter().map(|n| n.target.as_str()).collect();
    assert!(targets.contains(&"U2"));
    assert!(targets.contains(&"U4"));
    for n in &neighbors {
      assert_eq!(n.ego, "U1");
      assert!(n.score > 0.0, "Scores should be calculated from ego's standpoint");
    }
  }

  #[test]
  fn neighbors_prioritize_ego_owned_objects() {
    let mut graph = default_graph();

    graph.set_edge("O1".into(), "U1".into(), 1.0, 0);
    graph.set_edge("U1".into(), "U2".into(), 100.0, 0);
    graph.set_edge("U2".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U1".into(), "O1".into(), 1.0, 0);
    graph.set_edge("O1".into(), "U3".into(), 1.0, 0);

    let neighbors = read_neighbors_helper(
      &graph, "U1", "U3", NEIGHBORS_INBOUND, "", false, 100.0, false, -100.0,
      false, 0, 100,
    );

    assert_eq!(neighbors[0].ego, "U1");
    assert_eq!(neighbors[0].target, "O1");
    assert_eq!(neighbors[1].ego, "U1");
    assert_eq!(neighbors[1].target, "U2");
  }

  #[test]
  fn neighbors_omit_opinions_from_self_to_focus() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);

    graph.set_edge("O2".into(), "U2".into(), 1.0, 0);
    graph.set_edge("U2".into(), "O2".into(), 1.0, 0);
    graph.set_edge("O2".into(), "U3".into(), 1.0, 0);

    graph.set_edge("O3".into(), "U3".into(), 1.0, 0);
    graph.set_edge("U3".into(), "O3".into(), 1.0, 0);
    graph.set_edge("O3".into(), "U1".into(), 1.0, 0);

    let neighbors = read_neighbors_helper(
      &graph, "U1", "U3", NEIGHBORS_INBOUND, "O", false, 100.0, false, -100.0,
      false, 0, 100,
    );

    assert_eq!(neighbors[0].ego, "U1");
    assert_eq!(neighbors[0].target, "O2");
    assert_eq!(neighbors.len(), 1);
  }

  // --- Edge / delete / regression tests ---

  #[test]
  fn edge_uncontexted() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 1.5, 0);

    let edges: Vec<_> = graph
      .mr
      .graph
      .get_node_data(graph.nodes.get_by_name("U1").unwrap().id)
      .unwrap()
      .get_outgoing_edges()
      .collect();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].1, 1.5);
  }

  #[test]
  fn delete_nodes() {
    let mut graph = default_graph();

    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);

    let u1_id = graph.nodes.get_by_name("U1").unwrap().id;
    let u2_id = graph.nodes.get_by_name("U2").unwrap().id;

    graph.mr.set_edge(u1_id, u2_id, 0.0).unwrap();

    let data = graph.mr.graph.get_node_data(u1_id);
    let edge_count = match data {
      Some(d) => d.get_outgoing_edges().count(),
      None => 0,
    };
    assert_eq!(edge_count, 0);
  }

  #[test]
  fn regression_delete_self_reference_panic() {
    let mut graph = default_graph();
    graph.set_edge("U1".into(), "U2".into(), 1.0, 0);
    // Self-reference should be rejected gracefully (no panic)
    graph.set_edge("U1".into(), "U1".into(), 1.0, 0);
  }

  // --- Zero opinion tests ---

  #[test]
  fn set_zero_opinion_uncontexted() {
    let mut graph = default_graph_zero();
    graph.set_edge("U1".into(), "U2".into(), -5.0, 0);
    let s0 = read_node_score_helper(&graph, "U1", "U2")[0].score;

    let u2_id = graph.nodes.get_by_name("U2").unwrap().id;
    if u2_id >= graph.zero_opinion.len() {
      graph.zero_opinion.resize(u2_id + 1, 0.0);
    }
    graph.zero_opinion[u2_id] = 10.0;

    let s1 = read_node_score_helper(&graph, "U1", "U2")[0].score;

    assert_ne!(s0, s1);
  }

  // --- VSIDS tests ---

  #[test]
  fn vsids_write_edge() {
    let mut graph = AugGraph::new(Settings {
      num_walks: 500,
      ..Settings::default()
    });

    graph.set_edge("U1".into(), "U4".into(), 3.0, 0);
    graph.set_edge("U1".into(), "U2".into(), 3.0, 0);
    graph.set_edge("U1".into(), "U3".into(), 1.0, 20);

    let u12 = read_node_score_helper(&graph, "U1", "U2");
    let u13 = read_node_score_helper(&graph, "U1", "U3");

    assert!(
      u12[0].score < u13[0].score,
      "Assert that thanks to magnitude, U3 has a higher score than U2"
    );

    graph.set_edge("U1".into(), "U4".into(), 1.0, 200);
    let u12_final = read_node_score_helper(&graph, "U1", "U2");
    let u13_final = read_node_score_helper(&graph, "U1", "U3");
    assert!(
      u12_final.is_empty() || u12_final[0].score == 0.0,
      "U1->U2 edge should not exist"
    );
    assert!(
      u13_final.is_empty() || u13_final[0].score == 0.0,
      "U1->U3 edge should not exist"
    );
  }

  // --- omit_neg_edges_scores setting ---

  #[test]
  fn omit_neg_edges_scores_setting() {
    let mut graph_omit = AugGraph::new(Settings {
      num_walks:              50,
      zero_opinion_num_walks: 100,
      omit_neg_edges_scores:  true,
      ..Settings::default()
    });

    let mut graph_include = AugGraph::new(Settings {
      num_walks:              50,
      zero_opinion_num_walks: 100,
      omit_neg_edges_scores:  false,
      ..Settings::default()
    });

    let edges = vec![
      ("U1", "U2", -1.0),
      ("U1", "U3", 10.0),
      ("U3", "U2", 1.0),
    ];

    // Set zero opinion for U2
    {
      graph_include.set_edge("U1".into(), "U2".into(), 1.0, 0);
      let u2_id = graph_include.nodes.get_by_name("U2").unwrap().id;
      if u2_id >= graph_include.zero_opinion.len() {
        graph_include.zero_opinion.resize(u2_id + 1, 0.0);
      }
      graph_include.zero_opinion[u2_id] = 10.0;
      graph_include.mr.set_edge(
        graph_include.nodes.get_by_name("U1").unwrap().id,
        u2_id,
        0.0,
      ).unwrap();
    }
    {
      graph_omit.set_edge("U1".into(), "U2".into(), 1.0, 0);
      let u2_id = graph_omit.nodes.get_by_name("U2").unwrap().id;
      if u2_id >= graph_omit.zero_opinion.len() {
        graph_omit.zero_opinion.resize(u2_id + 1, 0.0);
      }
      graph_omit.zero_opinion[u2_id] = 10.0;
      graph_omit.mr.set_edge(
        graph_omit.nodes.get_by_name("U1").unwrap().id,
        u2_id,
        0.0,
      ).unwrap();
    }

    for (src, dst, weight) in edges {
      graph_omit.set_edge(src.into(), dst.into(), weight, 0);
      graph_include.set_edge(src.into(), dst.into(), weight, 0);
    }

    let scores_omit = read_scores(
      &graph_omit, "U1", "U", false, 100.0, false, -100.0, false, 0, u32::MAX,
    );

    let scores_include = read_scores(
      &graph_include, "U1", "U", false, 100.0, false, -100.0, false, 0, u32::MAX,
    );

    let find_node_score = |scores: &[ScoreResult], node: &str| -> Option<f64> {
      scores.iter().find(|s| s.target == node).map(|s| s.score)
    };

    let u2_score_include = find_node_score(&scores_include, "U2");
    let u2_score_omit = find_node_score(&scores_omit, "U2");

    assert!(
      u2_score_include.is_some(),
      "U2 should have a score when negative edges are included"
    );
    assert!(
      u2_score_omit.is_none(),
      "U2 should not have a score when negative edges are omitted"
    );
  }
}
