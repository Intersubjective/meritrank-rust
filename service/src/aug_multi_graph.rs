//  ================================================================
//
//    Augmented multi-layer graph
//
//  ================================================================

use crate::log::*;
use crate::vsids::VSIDSManager;
use lru::LruCache;
use meritrank_core::{constants::EPSILON, Graph, MeritRank, NodeId};
use std::{
  collections::hash_map::*, collections::HashMap, string::ToString,
  sync::atomic::Ordering, time::Instant,
};

use crate::constants::*;
use crate::nodes::*;
use crate::quantiles::*;
use crate::subgraph::*;

pub type Cluster = i32;

#[derive(Clone)]
pub struct AugMultiGraphSettings {
  pub num_walks:              usize,
  pub zero_opinion_num_walks: usize,
  pub top_nodes_limit:        usize,
  pub zero_opinion_factor:    f64,
  pub score_clusters_timeout: u64,
  pub scores_cache_size:      NonZeroUsize,
  pub walks_cache_size:       NonZeroUsize,
  pub filter_num_hashes:      usize,
  pub filter_max_size:        usize,
  pub filter_min_size:        usize,
}

#[derive(Clone)]
pub struct AugMultiGraph {
  pub settings:   AugMultiGraphSettings,
  pub node_count: usize,
  pub node_infos: Vec<NodeInfo>,
  pub node_ids:   HashMap<String, NodeId>,
  pub subgraphs:  HashMap<String, Subgraph>,
  pub time_begin: Instant,
  pub vsids:      VSIDSManager,
}

impl Default for AugMultiGraphSettings {
  fn default() -> AugMultiGraphSettings {
    AugMultiGraphSettings {
      scores_cache_size:      DEFAULT_SCORES_CACHE_SIZE,
      walks_cache_size:       DEFAULT_WALKS_CACHE_SIZE,
      zero_opinion_factor:    DEFAULT_ZERO_OPINION_FACTOR,
      num_walks:              DEFAULT_NUM_WALKS,
      zero_opinion_num_walks: DEFAULT_ZERO_OPINION_NUM_WALKS,
      score_clusters_timeout: DEFAULT_SCORE_CLUSTERS_TIMEOUT,
      filter_num_hashes:      DEFAULT_FILTER_NUM_HASHES,
      filter_max_size:        DEFAULT_FILTER_MAX_SIZE,
      filter_min_size:        DEFAULT_FILTER_MIN_SIZE,
      top_nodes_limit:        DEFAULT_TOP_NODES_LIMIT,
    }
  }
}

impl Default for AugMultiGraph {
  fn default() -> AugMultiGraph {
    AugMultiGraph::new(AugMultiGraphSettings::default())
  }
}

impl AugMultiGraph {
  pub fn cache_score_add(
    &mut self,
    context: &str,
    ego: NodeId,
    dst: NodeId,
    score: Weight,
  ) {
    log_trace!("{:?} {} {} {}", context, ego, dst, score);
    self
      .subgraph_from_context(context)
      .cache_score_add(ego, dst, score);
  }

  pub fn new(settings: AugMultiGraphSettings) -> AugMultiGraph {
    log_trace!();

    AugMultiGraph {
      settings:   settings.clone(),
      node_count: 0,
      node_infos: Vec::new(),
      node_ids:   HashMap::new(),
      subgraphs:  HashMap::new(),
      time_begin: Instant::now(),
      vsids:      VSIDSManager::new(),
    }
  }

  pub fn copy_from(
    &mut self,
    other: &AugMultiGraph,
  ) {
    log_trace!();

    self.node_count = other.node_count;
    self.node_infos = other.node_infos.clone();
    self.node_ids = other.node_ids.clone();
    self.subgraphs = other.subgraphs.clone();
    self.time_begin = other.time_begin.clone();
    self.vsids = other.vsids.clone();
  }

  pub fn reset(&mut self) {
    log_trace!();

    self.node_count = 0;
    self.node_infos = vec![];
    self.node_ids = HashMap::new();
    self.subgraphs = HashMap::new();
    self.time_begin = Instant::now();
    self.vsids = VSIDSManager::new();
  }

  pub fn node_exists(
    &self,
    node_name: &str,
  ) -> bool {
    self.node_ids.get(node_name).is_some()
  }

  pub fn get_subgraph_from_context(
    &mut self,
    context: &str,
  ) -> &Subgraph {
    &*self.subgraph_from_context(context)
  }

  pub fn subgraph_from_context(
    &mut self,
    context: &str,
  ) -> &mut Subgraph {
    log_trace!("{:?}", context);

    let infos = self.node_infos.clone();

    let zero_cloned = if context.is_empty() {
      None
    } else {
      match self.subgraphs.get(context) {
        None => match self.subgraphs.get_mut("") {
          Some(zero) => Some(zero.meritrank_data.clone()),
          None => None,
        },
        _ => None,
      }
    };

    match self.subgraphs.entry(context.to_string()) {
      Entry::Occupied(e) => {
        log_verbose!("Subgraph already exists: {:?}", context);
        return e.into_mut();
      },
      Entry::Vacant(e) => {
        log_verbose!("Add subgraph: {:?}", context);

        let mut graph = MeritRank::new(Graph::new());

        for _ in 0..self.node_count {
          graph.get_new_nodeid();
        }

        if !context.is_empty() {
          match zero_cloned {
            Some(zero_data) => {
              log_verbose!("Copy user edges from \"\" into {:?}", context);

              let all_nodes = zero_data.graph.nodes.iter().enumerate();

              for (src_id, src) in all_nodes {
                let all_edges =
                  src.pos_edges.iter().chain(src.neg_edges.iter());

                for (dst_id, weight) in all_edges {
                  if node_kind_from_id(&infos, src_id) == NodeKind::User
                    && node_kind_from_id(&infos, *dst_id) == NodeKind::User
                  {
                    graph.set_edge(src_id, *dst_id, *weight);
                  }
                }
              }
            },

            _ => {},
          }
        }

        e.insert(Subgraph {
          meritrank_data:        graph,
          zero_opinion:          Vec::new(),
          cached_scores:         LruCache::new(self.settings.scores_cache_size),
          cached_walks:          LruCache::new(self.settings.walks_cache_size),
          cached_score_clusters: Vec::new(),
        })
      },
    }
  }

  pub fn update_node_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
    kind: NodeKind,
    time_secs: u64,
  ) {
    log_trace!("{:?} {} {:?}", context, ego, kind);

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    let node_count = self.node_count;

    let node_ids = nodes_by_kind(kind, &self.node_infos);

    self
      .subgraph_from_context(context)
      .update_node_score_clustering(
        ego, kind, time_secs, node_count, num_walks, k, &node_ids,
      );
  }

  pub fn update_all_nodes_score_clustering(&mut self) {
    log_trace!();

    let time_secs = self.time_begin.elapsed().as_secs() as u64;

    for context in self.subgraphs.keys().map(|s| s.clone()).collect::<Vec<_>>()
    {
      for node_id in 0..self.node_count {
        for kind in ALL_NODE_KINDS {
          self.update_node_score_clustering(context.as_str(), node_id, kind, time_secs);
        }
      }
    }
  }

  fn init_node_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
  ) {
    log_trace!("{:?} {}", context, ego);

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    let node_count = self.node_count;

    let time_secs = self.time_begin.elapsed().as_secs() as u64;

    let node_infos = self.node_infos.clone();

    let subgraph = self.subgraph_from_context(context);

    subgraph
      .cached_score_clusters
      .resize(node_count, Default::default());

    for kind in ALL_NODE_KINDS {
      if bounds_are_empty(&subgraph.cached_score_clusters[ego][kind].bounds) {
        let node_ids = nodes_by_kind(kind, &node_infos);

        subgraph.update_node_score_clustering(
          ego, kind, time_secs, node_count, num_walks, k, &node_ids,
        );
      }
    }
  }

  pub fn apply_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
    score: Weight,
    kind: NodeKind,
    time_secs: u64,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, ego, score);

    if score < EPSILON {
      //  Clusterize only positive scores.
      return (score, 0);
    }

    if ego >= self.node_count {
      log_error!("Node does not exist: {}", ego);
      return (score, 0);
    }

    if node_kind_from_id(&self.node_infos, ego) != NodeKind::User {
      //  We apply score clustering only for user nodes.
      return (score, 0);
    }

    self.init_node_score_clustering(context, ego);

    let elapsed_secs = self.time_begin.elapsed().as_secs();

    let clusters = &self.subgraph_from_context(context).cached_score_clusters;

    let updated_sec = clusters[ego][kind].updated_sec;

    if elapsed_secs >= updated_sec + self.settings.score_clusters_timeout {
      log_verbose!("Recalculate clustering for node {} in {:?}", ego, context);
      self.update_node_score_clustering(context, ego, kind, time_secs);
    }

    let clusters = &self.subgraph_from_context(context).cached_score_clusters;

    let bounds = &clusters[ego][kind].bounds;

    if bounds_are_empty(&bounds) {
      return (score, 0);
    }

    let step = 1;
    let mut cluster = 1;

    for bound in bounds {
      if score < *bound - EPSILON {
        break;
      }

      cluster += step;
    }

    return (score, cluster);
  }

  pub fn fetch_all_scores(
    &mut self,
    context: &str,
    ego_id: NodeId,
    time_secs: u64,
  ) -> Vec<(NodeId, Weight, Cluster)> {
    log_trace!("{:?} {}", context, ego_id);

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    self
      .subgraph_from_context(context)
      .fetch_all_raw_scores(ego_id, num_walks, k)
      .iter()
      .map(|(dst_id, score)| {
        let kind = node_kind_from_id(&self.node_infos, *dst_id);
        (
          *dst_id,
          *score,
          self.apply_score_clustering(context, ego_id, *score, kind, time_secs).1,
        )
      })
      .collect()
  }

  pub fn fetch_neighbors(
    &mut self,
    context: &str,
    ego: NodeId,
    dir: NeighborDirection,
    time_secs: u64,
  ) -> Vec<(NodeId, Weight, Cluster)> {
    log_trace!("{:?} {} {:?}", context, ego, dir);

    let mut v = vec![];

    match dir {
      NeighborDirection::Outbound => {
        match self
          .subgraph_from_context(context)
          .meritrank_data
          .graph
          .get_node_data(ego)
        {
          Some(data) => {
            v.reserve_exact(data.pos_edges.len() + data.neg_edges.len());

            for x in &data.pos_edges {
              v.push((*x.0, 0.0, 0));
            }

            for x in &data.neg_edges {
              v.push((*x.0, 0.0, 0));
            }
          },
          _ => {},
        };
      },
      _ => {
        for src in 0..self.node_infos.len() {
          match self
            .subgraph_from_context(context)
            .meritrank_data
            .graph
            .get_node_data(src)
          {
            Some(data) => {
              for (dst, _) in data.get_outgoing_edges() {
                if dir == NeighborDirection::All && src == ego {
                  //  Outbound: ego -> dst
                  v.push((dst, 0.0, 0));
                } else if dst == ego {
                  //  Inbound:  src -> ego
                  v.push((src, 0.0, 0));
                }
              }
            },
            _ => {},
          };
        }
      },
    };

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    for i in 0..v.len() {
      let dst = v[i].0;
      let score = self
        .subgraph_from_context(context)
        .fetch_raw_score(ego, dst, num_walks, k);
      let kind = node_kind_from_id(&self.node_infos, dst);
      let cluster = self.apply_score_clustering(context, ego, score, kind, time_secs).1;

      v[i].1 = score;
      v[i].2 = cluster;
    }

    v
  }

  pub fn fetch_score(
    &mut self,
    context: &str,
    ego: NodeId,
    dst: NodeId,
    time_secs: u64,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, ego, dst);

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    let score = self
      .subgraph_from_context(context)
      .fetch_raw_score(ego, dst, num_walks, k);
    let kind = node_kind_from_id(&self.node_infos, dst);
    self.apply_score_clustering(context, ego, score, kind, time_secs)
  }

  pub fn fetch_score_reversed(
    &mut self,
    context: &str,
    dst_id: NodeId,
    ego_id: NodeId,
    time_secs: u64,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, dst_id, ego_id);

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    let subgraph = self.subgraph_from_context(context);

    let score = match subgraph.cache_score_get(ego_id, dst_id) {
      Some(score) => subgraph.with_zero_opinion(dst_id, score, k),
      None => subgraph.fetch_raw_score(ego_id, dst_id, num_walks, k),
    };

    let kind = node_kind_from_id(&self.node_infos, dst_id);

    self.apply_score_clustering(context, ego_id, score, kind, time_secs)
  }

  pub fn fetch_user_score_reversed(
    &mut self,
    context: &str,
    dst_id: NodeId,
    ego_id: NodeId,
    time_secs: u64,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, dst_id, ego_id);

    if node_kind_from_id(&self.node_infos, ego_id) == NodeKind::User {
      return self.fetch_score_reversed(context, dst_id, ego_id, time_secs);
    }

    match self
      .subgraph_from_context(context)
      .meritrank_data
      .graph
      .get_node_data(ego_id)
    {
      Some(x) => {
        if x.pos_edges.len() + x.neg_edges.len() == 0 {
          log_error!("Non-user node has no owner");
          (0.0, 0)
        } else {
          if x.pos_edges.len() + x.neg_edges.len() != 1 {
            log_error!("Non-user node has too many edges");
          }

          let parent_id = if x.pos_edges.len() > 0 {
            x.pos_edges.keys()[0]
          } else {
            x.neg_edges.keys()[0]
          };

          self.fetch_score_reversed(context, dst_id, parent_id, time_secs)
        }
      },

      None => {
        log_error!("Node does not exist");
        (0.0, 0)
      },
    }
  }

  pub fn find_or_add_node_by_name(
    &mut self,
    node_name: &str,
  ) -> NodeId {
    log_trace!("{:?}", node_name);

    let node_id;

    if let Some(&id) = self.node_ids.get(node_name) {
      node_id = id;
    } else {
      node_id = self.node_count;

      self.node_count += 1;
      self.node_infos.resize(self.node_count, NodeInfo::default());
      self.node_infos[node_id] = NodeInfo {
        kind:       kind_from_name(&node_name),
        name:       node_name.to_string(),
        seen_nodes: Default::default(),
      };
      self.node_ids.insert(node_name.to_string(), node_id);
    }

    for (context, subgraph) in &mut self.subgraphs {
      if subgraph.meritrank_data.graph.contains_node(node_id) {
        continue;
      }

      log_verbose!("Add node in {:?}: {}", context, node_name);

      //  HACK!!!
      while subgraph.meritrank_data.get_new_nodeid() < node_id {}
    }

    node_id
  }

  pub fn set_edge(
    &mut self,
    context: &str,
    src: NodeId,
    dst: NodeId,
    amount: f64,
  ) {
    log_trace!("{:?} {} {} {}", context, src, dst, amount);

    if src == dst {
      log_error!("Self-reference is not allowed.");
      return;
    }

    if node_kind_from_id(&self.node_infos, src) == NodeKind::User
      && node_kind_from_id(&self.node_infos, dst) == NodeKind::User
    {
      // TODO: move this to the initializer
      self.subgraph_from_context("");
      if !context.is_empty() {
        self.subgraph_from_context(context);
      }

      for (enum_context, subgraph) in &mut self.subgraphs {
        log_verbose!(
          "Set user edge in {:?}: {} -> {} for {}",
          enum_context,
          src,
          dst,
          amount
        );
        subgraph.meritrank_data.set_edge(src, dst, amount);
      }
    } else if context.is_empty() {
      log_verbose!("Set edge in \"\": {} -> {} for {}", src, dst, amount);
      self
        .subgraph_from_context(context)
        .meritrank_data
        .set_edge(src, dst, amount);
    } else {
      let null_weight = self.subgraph_from_context("").edge_weight(src, dst);
      let old_weight =
        self.subgraph_from_context(context).edge_weight(src, dst);
      let delta = null_weight + amount - old_weight;

      log_verbose!("Set edge in \"\": {} -> {} for {}", src, dst, delta);
      self
        .subgraph_from_context("")
        .meritrank_data
        .set_edge(src, dst, delta);

      log_verbose!(
        "Set edge in {:?}: {} -> {} for {}",
        context,
        src,
        dst,
        amount
      );
      self
        .subgraph_from_context(context)
        .meritrank_data
        .set_edge(src, dst, amount);
    }
  }
}
