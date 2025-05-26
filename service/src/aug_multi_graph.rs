//  ================================================================
//
//    Augmented multi-layer graph
//
//  ================================================================

use crate::constants::*;
use crate::log::*;
use crate::nodes::*;
use crate::quantiles::*;
use crate::subgraph::*;
use crate::vsids::VSIDSManager;
use lru::LruCache;
use meritrank_core::{constants::EPSILON, Graph, MeritRank, NodeId};
use std::{
  collections::HashMap, string::ToString, sync::atomic::Ordering, time::Instant,
};

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
  pub omit_neg_edges_scores:  bool,
  pub force_read_graph_conn:  bool,
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
      omit_neg_edges_scores:  DEFAULT_OMIT_NEG_EDGES_SCORES,
      force_read_graph_conn:  DEFAULT_FORCE_READ_GRAPH_CONN,
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

  fn create_zero_context(&mut self) {
    let zero_context = "".to_string();
    let must_add_nodes = !self.subgraphs.contains_key(&zero_context);

    // Insert the zero context subgraph if it doesn't exist
    self
      .subgraphs
      .entry(zero_context.clone())
      .or_insert_with(|| Subgraph {
        meritrank_data:        MeritRank::new(Graph::new()),
        zero_opinion:          Vec::new(),
        cached_scores:         LruCache::new(self.settings.scores_cache_size),
        cached_walks:          LruCache::new(self.settings.walks_cache_size),
        cached_score_clusters: Vec::new(),
        omit_neg_edges_scores: self.settings.omit_neg_edges_scores,
      });

    // Add nodes to the zero context if needed
    if must_add_nodes {
      // Unwrap is safe here because we've just inserted the zero context subgraph
      let zero_subgraph = self.subgraphs.get_mut(&zero_context).unwrap();
      for _ in 0..self.node_count {
        zero_subgraph.meritrank_data.get_new_nodeid();
      }
    }
  }

  fn mr_graph_with_users_from_zero_context(&mut self) -> MeritRank {
    let mut new_graph_instance = MeritRank::new(Graph::new());
    // Copy user-to-user edges from zero context if needed
    for _ in 0..self.node_count {
      new_graph_instance.get_new_nodeid();
    }
    let zero_graph = &self
      .subgraphs
      .get(&"".to_string())
      .unwrap()
      .meritrank_data
      .graph;

    for (src_id, src) in zero_graph.nodes.iter().enumerate() {
      let all_edges = src.pos_edges.iter().chain(src.neg_edges.iter());
      for (dst_id, weight) in all_edges {
        if node_kind_from_id(&self.node_infos, src_id) == NodeKind::User
          && node_kind_from_id(&self.node_infos, *dst_id) == NodeKind::User
        {
          new_graph_instance.set_edge(src_id, *dst_id, *weight);
        }
      }
    }
    new_graph_instance
  }

  pub fn subgraph_from_context(
    &mut self,
    context: &str,
  ) -> &mut Subgraph {
    log_trace!("{:?}", context);

    // ACHTUNG: we have to search the map twice here to avoid triggering borrow checker error.
    if self.subgraphs.contains_key(context) {
      log_verbose!("Subgraph already exists: {:?}", context);
      return self.subgraphs.get_mut(context).unwrap();
    }
    // Create the zero context if it doesn't exist
    if !self.subgraphs.contains_key(&"".to_string()) {
      self.create_zero_context();
      if context == "" {
        // Handle the case where the first call to `subgraph_from_context` is for the zero context
        return self.subgraphs.get_mut(context).unwrap();
      }
    }

    // We must first create the new graph entry, and then move it to the subgraph
    // object to avoid triggering borrow checker error.
    log_verbose!("Add subgraph: {:?}", context);
    let new_graph_instance = self.mr_graph_with_users_from_zero_context();
    self.subgraphs.insert(
      context.to_string(),
      Subgraph {
        meritrank_data:        new_graph_instance,
        zero_opinion:          Vec::new(),
        cached_scores:         LruCache::new(self.settings.scores_cache_size),
        cached_walks:          LruCache::new(self.settings.walks_cache_size),
        cached_score_clusters: Vec::new(),
        omit_neg_edges_scores: self.settings.omit_neg_edges_scores,
      },
    );

    // Return the requested subgraph
    self.subgraphs.get_mut(&context.to_string()).unwrap()
  }

  pub fn update_node_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
    kind: NodeKind,
  ) {
    log_trace!("{:?} {} {:?}", context, ego, kind);

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    let node_count = self.node_count;

    let time_secs = self.time_begin.elapsed().as_secs() as u64;

    let node_ids = nodes_by_kind(kind, &self.node_infos);

    self
      .subgraph_from_context(context)
      .update_node_score_clustering(
        ego, kind, time_secs, node_count, num_walks, k, &node_ids,
      )
  }

  pub fn update_all_nodes_score_clustering(&mut self) {
    log_trace!();

    for context in self.subgraphs.keys().map(|s| s.clone()).collect::<Vec<_>>()
    {
      for node_id in 0..self.node_count {
        for kind in ALL_NODE_KINDS {
          self.update_node_score_clustering(context.as_str(), node_id, kind);
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
      self.update_node_score_clustering(context, ego, kind);
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
          self.apply_score_clustering(context, ego_id, *score, kind).1,
        )
      })
      .collect()
  }

  pub fn fetch_neighbors(
    &mut self,
    context: &str,
    ego: NodeId,
    focus: NodeId,
    dir: crate::protocol::NeighborDirection, // Changed to use protocol::NeighborDirection
  ) -> Vec<(NodeId, Weight, Cluster)> {
    log_trace!("{:?} {} {} {:?}", context, ego, focus, dir);
    self
      .subgraph_from_context(context)
      .meritrank_data
      .graph
      .get_node_data(focus)
      .map(|node_data| {
        let edges: Vec<_> = match dir {
          crate::protocol::NeighborDirection::Outbound => { // Changed to use protocol::NeighborDirection
            node_data.get_outgoing_edges().collect()
          },
          crate::protocol::NeighborDirection::Inbound => { // Changed to use protocol::NeighborDirection
            node_data.get_inbound_edges().collect()
          },
          crate::protocol::NeighborDirection::All => node_data // Changed to use protocol::NeighborDirection
            .get_outgoing_edges()
            .chain(node_data.get_inbound_edges())
            .collect(),
        };
        edges.into_iter()
      })
      .unwrap_or_default()
      .map(|(dst, _)| {
        let (score, cluster) = self.fetch_score(context, ego, dst);
        (dst, score, cluster)
      })
      .collect::<Vec<_>>()
  }

  pub fn fetch_score(
    &mut self,
    context: &str,
    ego: NodeId,
    dst: NodeId,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, ego, dst);

    let num_walks = self.settings.num_walks;
    let k = self.settings.zero_opinion_factor;

    let score = self
      .subgraph_from_context(context)
      .fetch_raw_score(ego, dst, num_walks, k);
    let kind = node_kind_from_id(&self.node_infos, dst);
    self.apply_score_clustering(context, ego, score, kind)
  }

  pub fn fetch_score_cached(
    &mut self,
    context: &str,
    ego_id: NodeId,
    dst_id: NodeId,
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

    self.apply_score_clustering(context, ego_id, score, kind)
  }

  pub fn get_object_owner(
    &mut self,
    context: &str,
    dst_id: NodeId,
  ) -> Option<NodeId> {
    log_trace!("{:?} {}", context, dst_id);

    let node_kind = node_kind_from_id(&self.node_infos, dst_id);
    if node_kind == NodeKind::User {
      return Some(dst_id);
    }

    match self
      .subgraph_from_context(context)
      .meritrank_data
      .graph
      .get_node_data(dst_id)
    {
      Some(x) => {
        if x.pos_edges.len() == 1 {
          Some(x.pos_edges.keys()[0])
        } else {
          if x.pos_edges.len() == 0 {
            log_error!("Non-user node has no owner");
          }
          if x.pos_edges.len() > 1 && node_kind != NodeKind::Opinion {
            log_error!("Non-user node has too many edges");
          }
          if x.pos_edges.len() == 2 && node_kind == NodeKind::Opinion {
            // FIXME! This might produce incorrect results in case the first edge is the edge to the opinion's target
            return Some(x.pos_edges.keys()[0]);
          }
          log_error!("Something went wrong with finding the node's owner");
          None
        }
      },

      None => {
        log_error!("Node does not exist");
        None
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
      self.subgraph_from_context(context);

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
