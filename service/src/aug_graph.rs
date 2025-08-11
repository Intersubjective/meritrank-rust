use crate::data::*;
use crate::helpers::*;
use crate::legacy_protocol::*;
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
      self.fetch_score_cached(dst_id, ego_info.id);

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

  fn get_object_owner(
    &self,
    node: NodeId,
  ) -> Option<NodeId> {
    Some(node)

    //  FIXME
    //
    // match self.nodes.id_to_info.get(node) {
    //   Some(info) => match info.owner {
    //     Some(id) => Some(id),
    //     None => Some(node),
    //   },
    //   None => Some(node),
    // }
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

    let edges: Vec<_> = match dir {
      NEIGHBORS_OUTBOUND => node_data.pos_edges.iter().collect(),
      NEIGHBORS_INBOUND => node_data.neg_edges.iter().collect(),
      NEIGHBORS_ALL => node_data
        .pos_edges
        .iter()
        .chain(node_data.neg_edges.iter())
        .collect(),
      _ => {
        log_error!("Invalid direction: {}", dir);
        return vec![];
      },
    };

    edges
      .into_iter()
      .map(|(dst_id, &weight)| {
        let (_score, cluster) = self.fetch_score_cached(ego_id, *dst_id);
        (
          self.nodes.get_by_id(*dst_id).unwrap().clone(),
          weight,
          cluster,
        )
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
          self.fetch_score_cached(target_info.id, ego_info.id);
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

    self.mr.set_edge(src_id, dst_id, amount);

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
      self.mr.set_edge(src_id, dst_id_iter, weight_iter);
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

    match self.reg_owner_and_get_ids(src, dst) {
      Ok((src_id, dst_id)) => {
        self.set_edge_by_id(src_id, dst_id, amount, magnitude);
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
        let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
        let dst_id =
          self
            .nodes
            .register_with_owner(&mut self.mr, dst, src_kind, src_id);
        Ok((src_id, dst_id))
      },
      (Some(src_kind), Some(dst_kind)) => {
        if self.settings.legacy_connections_mode {
          let src_id = self.nodes.register(&mut self.mr, src, src_kind);
          let dst_id = self.nodes.register(&mut self.mr, dst, dst_kind);
          Ok((src_id, dst_id))
        } else {
          Err(AugGraphError::IncorrectNodeKinds(src, dst))
        }
      },
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
}
