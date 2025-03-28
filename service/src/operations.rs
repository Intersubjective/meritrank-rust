use lru::LruCache;
use meritrank_core::{constants::EPSILON, Graph, MeritRank, NodeId};
use petgraph::{
  graph::{DiGraph, NodeIndex},
  visit::EdgeRef,
};
use simple_pagerank::Pagerank;
use std::{
  collections::HashMap, string::ToString, sync::atomic::Ordering, time::Instant,
};

use crate::astar::*;
use crate::log::*;
use crate::log_command;
use crate::log_error;
use crate::log_trace;
use crate::log_verbose;
use crate::log_warning;
use crate::protocol::*;
use crate::vsids::VSIDSManager;

pub use meritrank_core::Weight;
pub use std::num::NonZeroUsize;
pub type Cluster = i32;

//  ================================================================
//
//    Constants
//
//  ================================================================

pub const VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
  Some(x) => x,
  None => "dev",
};

pub const NUM_SCORE_QUANTILES: usize = 100;

pub const DEFAULT_NUM_WALKS: usize = 50;
pub const DEFAULT_TOP_NODES_LIMIT: usize = 100;
pub const DEFAULT_ZERO_OPINION_FACTOR: f64 = 0.20;
pub const DEFAULT_SCORE_CLUSTERS_TIMEOUT: u64 = 60 * 60 * 6; // 6 hours
pub const DEFAULT_SCORES_CACHE_SIZE: NonZeroUsize =
  NonZeroUsize::new(1024 * 10).unwrap();
pub const DEFAULT_WALKS_CACHE_SIZE: NonZeroUsize =
  NonZeroUsize::new(1024).unwrap();
pub const DEFAULT_FILTER_NUM_HASHES: usize = 10;
pub const DEFAULT_FILTER_MAX_SIZE: usize = 8192;
pub const DEFAULT_FILTER_MIN_SIZE: usize = 32;

//  ================================================================
//
//    Basic declarations
//
//  ================================================================

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum NodeKind {
  #[default]
  Unknown,
  User,
  Beacon,
  Comment,
  Opinion,
}

#[derive(Debug, PartialEq)]
pub enum NeighborDirection {
  All,
  Outbound,
  Inbound,
}

#[derive(PartialEq, Clone, Default)]
pub struct NodeInfo {
  pub kind: NodeKind,
  pub name: String,

  // Bloom filter of nodes marked as seen by this node in the null context
  pub seen_nodes: Vec<u64>,
}

#[derive(PartialEq, Clone)]
pub struct ClusterGroupBounds {
  pub updated_sec: u64,
  pub bounds:      [Weight; NUM_SCORE_QUANTILES - 1],
}

impl Default for ClusterGroupBounds {
  fn default() -> ClusterGroupBounds {
    ClusterGroupBounds {
      updated_sec: 0,
      bounds:      [0.0; NUM_SCORE_QUANTILES - 1],
    }
  }
}

#[derive(PartialEq, Clone, Default)]
pub struct ScoreClustersByKind {
  //  FIXME: Refactor this to be more general.
  pub users:    ClusterGroupBounds,
  pub beacons:  ClusterGroupBounds,
  pub comments: ClusterGroupBounds,
  pub opinions: ClusterGroupBounds,
}

//  Augmented multi-context graph settings
//
#[derive(Clone)]
pub struct AugMultiGraphSettings {
  pub num_walks:              usize,
  pub top_nodes_limit:        usize,
  pub zero_opinion_factor:    f64,
  pub score_clusters_timeout: u64,
  pub scores_cache_size:      NonZeroUsize,
  pub walks_cache_size:       NonZeroUsize,
  pub filter_num_hashes:      usize,
  pub filter_max_size:        usize,
  pub filter_min_size:        usize,
}

//  Augmented multi-context graph
//
#[derive(Clone)]
pub struct AugMultiGraph {
  pub settings:              AugMultiGraphSettings,
  pub node_count:            usize,
  pub node_infos:            Vec<NodeInfo>,
  pub node_ids:              HashMap<String, NodeId>,
  pub contexts:              HashMap<String, MeritRank>,
  pub score_cache:           LruCache<(String, NodeId, NodeId), Weight>,
  pub cached_walks:          LruCache<(String, NodeId), ()>,
  pub zero_opinion:          HashMap<String, Vec<Weight>>,
  pub time_begin:            Instant,
  pub cached_score_clusters: HashMap<String, Vec<ScoreClustersByKind>>,
  pub vsids:                 VSIDSManager,

  //  We need this dummy value so we can return a reference to it when the requested
  //  node does not exist. We don't want to use static variable instead, because we don't want
  //  any potential bugs related to it to bleed into other instances of AugMultiGraph. See node_info_from_id.
  pub dummy_node_info: NodeInfo,
}

//  ================================================================
//
//    Bloom filter
//
//  ================================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

pub fn bloom_filter_bits(
  size: usize,
  num_hashes: usize,
  id: usize,
) -> Vec<u64> {
  let mut v: Vec<u64> = vec![];
  v.resize(size, 0);

  for n in 1..=num_hashes {
    let mut h = DefaultHasher::new();
    h.write_u16(n as u16);
    h.write_u64(id as u64);
    let hash = h.finish();

    let u64_index = ((hash / 64u64) as usize) % size;
    let bit_index = hash % 64u64;

    v[u64_index] |= 1u64 << bit_index;
  }

  v
}

pub fn bloom_filter_add(
  mask: &mut [u64],
  bits: &[u64],
) {
  if mask.len() != bits.len() {
    log_error!("Invalid arguments");
    return;
  }

  for i in 0..mask.len() {
    mask[i] |= bits[i];
  }
}

pub fn bloom_filter_contains(
  mask: &[u64],
  bits: &[u64],
) -> bool {
  if mask.len() != bits.len() {
    log_error!("Invalid arguments");
    return false;
  }

  for i in 0..mask.len() {
    if (mask[i] & bits[i]) != bits[i] {
      return false;
    }
  }

  return true;
}

//  ================================================================
//
//    Caches
//
//  ================================================================

impl AugMultiGraph {
  pub fn cache_score_add(
    &mut self,
    context: &str,
    ego: NodeId,
    dst: NodeId,
    score: Weight,
  ) {
    log_trace!("{:?} {} {} {}", context, ego, dst, score);
    self.score_cache.put((context.to_string(), ego, dst), score);
  }

  pub fn cache_score_get(
    &mut self,
    context: &str,
    ego: NodeId,
    dst: NodeId,
  ) -> Option<Weight> {
    log_trace!("{:?} {} {}", context, ego, dst);
    self
      .score_cache
      .get(&(context.to_string(), ego, dst))
      .copied()
  }

  pub fn cache_walk_add(
    &mut self,
    context: &str,
    ego: NodeId,
  ) {
    log_trace!("{:?} {}", context, ego);

    let cached_walk = (context.to_string(), ego);

    if let Some(((old_context, old_ego), _)) =
      self.cached_walks.push(cached_walk, ())
    {
      if !(old_context == context && old_ego == ego) {
        log_verbose!("Drop walks {:?}, {}", old_context, old_ego);

        // HACK!!!
        // We "drop" the walks by recalculating the node with 0.
        match self.graph_from_ctx_mut(&old_context).calculate(old_ego, 0) {
          Ok(()) => {},
          Err(e) => {
            log_error!("{}", e);
          },
        }
      }
    }
  }

  pub fn cache_walk_get(
    &mut self,
    context: &str,
    ego: NodeId,
  ) -> bool {
    log_trace!();

    self.cached_walks.get(&(context.to_string(), ego)).is_some()
  }
}

//  ================================================================
//
//    Quantiles
//
//  ================================================================

fn bounds_are_empty(bounds: &[Weight; NUM_SCORE_QUANTILES - 1]) -> bool {
  return bounds[0] == 0.0 && bounds[bounds.len() - 1] == 0.0;
}

fn calculate_quantiles_bounds(
  mut scores: Vec<Weight>
) -> [Weight; NUM_SCORE_QUANTILES - 1] {
  if scores.is_empty() {
    return [0.0; NUM_SCORE_QUANTILES - 1];
  }

  if scores.len() == 1 {
    let bound = scores[0] - EPSILON - EPSILON;
    return [bound; NUM_SCORE_QUANTILES - 1];
  }

  scores.sort_by(|a, b| b.total_cmp(a));

  let mut bounds = [0.0; NUM_SCORE_QUANTILES - 1];

  for i in 0..bounds.len() {
    let n = std::cmp::min(
      (((i * scores.len()) as f64) / ((bounds.len() + 1) as f64)).floor()
        as usize,
      scores.len() - 2,
    );

    bounds[bounds.len() - i - 1] = (scores[n] + scores[n + 1]) / 2.0;
  }

  bounds
}

//  ================================================================
//
//    Utils
//
//  ================================================================

pub fn kind_from_name(name: &str) -> NodeKind {
  log_trace!("{:?}", name);

  match name.chars().nth(0) {
    Some('U') => NodeKind::User,
    Some('B') => NodeKind::Beacon,
    Some('C') => NodeKind::Comment,
    Some('O') => NodeKind::Comment,
    _ => NodeKind::Unknown,
  }
}

pub fn kind_from_prefix(prefix: &str) -> Result<NodeKind, ()> {
  match prefix {
    "" => Ok(NodeKind::Unknown),
    "U" => Ok(NodeKind::User),
    "B" => Ok(NodeKind::Beacon),
    "C" => Ok(NodeKind::Comment),
    "O" => Ok(NodeKind::Opinion),
    _ => Err(()),
  }
}

pub fn neighbor_dir_from(dir: i64) -> Result<NeighborDirection, ()> {
  match dir {
    NEIGHBORS_ALL => Ok(NeighborDirection::All),
    NEIGHBORS_OUTBOUND => Ok(NeighborDirection::Outbound),
    NEIGHBORS_INBOUND => Ok(NeighborDirection::Inbound),
    _ => Err(()),
  }
}

impl Default for AugMultiGraphSettings {
  fn default() -> AugMultiGraphSettings {
    AugMultiGraphSettings {
      scores_cache_size:      DEFAULT_SCORES_CACHE_SIZE,
      walks_cache_size:       DEFAULT_WALKS_CACHE_SIZE,
      zero_opinion_factor:    DEFAULT_ZERO_OPINION_FACTOR,
      num_walks:              DEFAULT_NUM_WALKS,
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

fn create_score_cache(
  cache_size: NonZeroUsize
) -> LruCache<(String, NodeId, NodeId), Weight> {
  LruCache::new(cache_size)
}

fn create_cached_walks(
  cache_size: NonZeroUsize
) -> LruCache<(String, NodeId), ()> {
  LruCache::new(cache_size)
}

impl AugMultiGraph {
  pub fn new(settings: AugMultiGraphSettings) -> AugMultiGraph {
    log_trace!();

    AugMultiGraph {
      settings:              settings.clone(),
      node_count:            0,
      node_infos:            Vec::new(),
      node_ids:              HashMap::new(),
      contexts:              HashMap::new(),
      score_cache:           create_score_cache(settings.scores_cache_size),
      cached_walks:          create_cached_walks(settings.walks_cache_size),
      zero_opinion:          HashMap::new(),
      time_begin:            Instant::now(),
      cached_score_clusters: HashMap::new(),
      vsids:                 VSIDSManager::new(),
      dummy_node_info:       NodeInfo::default(),
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
    self.contexts = other.contexts.clone();
    self.score_cache = other.score_cache.clone();
    self.cached_walks = other.cached_walks.clone();
    self.zero_opinion = other.zero_opinion.clone();
    self.time_begin = other.time_begin.clone();
    self.cached_score_clusters = other.cached_score_clusters.clone();
    self.vsids = other.vsids.clone();
  }

  pub fn reset(&mut self) {
    log_trace!();

    self.node_count = 0;
    self.node_infos = vec![];
    self.node_ids = HashMap::new();
    self.contexts = HashMap::new();
    self.score_cache = create_score_cache(self.settings.scores_cache_size);
    self.cached_walks = create_cached_walks(self.settings.walks_cache_size);
    self.zero_opinion = HashMap::new();
    self.time_begin = Instant::now();
    self.cached_score_clusters = HashMap::new();
    self.vsids = VSIDSManager::new();
  }

  pub fn node_exists(
    &self,
    node_name: &str,
  ) -> bool {
    log_trace!("{:?}", node_name);

    self.node_ids.get(node_name).is_some()
  }

  pub fn node_info_from_id(
    &self,
    node_id: NodeId,
  ) -> &NodeInfo {
    log_trace!("{}", node_id);

    self.node_infos.get(node_id).unwrap_or_else(|| {
      log_error!("Node does not exist: {:?}", node_id);
      &self.dummy_node_info
    })
  }

  pub fn is_user_edge(
    &self,
    src: NodeId,
    dst: NodeId,
  ) -> bool {
    log_trace!("{} {}", src, dst);

    self.node_info_from_id(src).kind == NodeKind::User
      && self.node_info_from_id(dst).kind == NodeKind::User
  }

  pub fn create_context_if_does_not_exist(
    &mut self,
    context: &str,
  ) {
    log_trace!("{:?}", context);

    if self.contexts.contains_key(context) {
      log_verbose!("Context already exists: {:?}", context);
      return;
    }

    log_verbose!("Add context: {:?}", context);

    let mut graph = MeritRank::new(Graph::new());

    for _ in 0..self.node_count {
      graph.get_new_nodeid();
    }

    if !context.is_empty() {
      match self.contexts.get_mut("") {
        Some(zero) => {
          log_verbose!("Copy user edges from \"\" into {:?}", context);

          let zero_cloned = zero.clone();
          let all_nodes = zero_cloned.graph.nodes.iter().enumerate();

          for (src_id, src) in all_nodes {
            let all_edges = src.pos_edges.iter().chain(src.neg_edges.iter());

            for (dst_id, weight) in all_edges {
              if self.is_user_edge(src_id, *dst_id) {
                graph.set_edge(src_id, *dst_id, *weight);
              }
            }
          }
        },

        _ => {},
      }
    }

    self.contexts.insert(context.to_string(), graph);
  }

  pub fn graph_from_ctx_mut(
    &mut self,
    context: &str,
  ) -> &mut MeritRank {
    log_trace!("{:?}", context);

    self.create_context_if_does_not_exist(context);

    self.contexts.get_mut(context).unwrap_or_else(|| {
          panic!("Failed to get context '{}' after creation attempt. This is likely a bug in the create_context_if_does_not_exist function.", context)
      })
  }

  pub fn graph_from_ctx(
    &mut self,
    context: &str,
  ) -> &MeritRank {
    log_trace!("{:?}", context);

    self.create_context_if_does_not_exist(context);

    self.contexts.get(context).unwrap_or_else(|| {
      panic!("Failed to get context '{}' after creation attempt. This is likely a bug in the create_context_if_does_not_exist function.", context)
    })
  }

  pub fn edge_weight(
    &mut self,
    context: &str,
    src: NodeId,
    dst: NodeId,
  ) -> Weight {
    log_trace!("{:?} {} {}", context, src, dst);

    self
      .graph_from_ctx(context)
      .graph
      .edge_weight(src, dst)
      .unwrap_or(None)
      .unwrap_or(0.0)
  }

  pub fn edge_weight_normalized(
    &mut self,
    context: &str,
    src: NodeId,
    dst: NodeId,
  ) -> Weight {
    log_trace!("{:?} {} {}", context, src, dst);

    let graph = self.graph_from_ctx(context);

    let pos_sum = match graph.graph.get_node_data(src) {
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

    graph
      .graph
      .edge_weight(src, dst)
      .unwrap_or(None)
      .unwrap_or(0.0)
      / pos_sum
  }

  pub fn all_outbound_neighbors_normalized(
    &mut self,
    context: &str,
    node: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{:?} {}", context, node);

    let mut v = vec![];

    match self.graph_from_ctx(context).graph.get_node_data(node) {
      None => {},
      Some(data) => {
        v.reserve_exact(data.pos_edges.len() + data.neg_edges.len());

        let pos_sum = if data.pos_sum < EPSILON {
          log_warning!(
            "Unable to normalize node weight, positive sum is zero."
          );
          1.0
        } else {
          data.pos_sum
        };

        for x in &data.pos_edges {
          v.push((*x.0, *x.1 / pos_sum));
        }

        for x in &data.neg_edges {
          v.push((*x.0, *x.1 / pos_sum));
        }
      },
    }

    v
  }

  fn with_zero_opinion(
    &mut self,
    context: &str,
    dst_id: NodeId,
    score: Weight,
  ) -> Weight {
    log_trace!("{:?} {} {}", context, dst_id, score);

    match self.zero_opinion.get(context) {
      Some(zero_opinion) => {
        let zero_score = match zero_opinion.get(dst_id) {
          Some(x) => *x,
          _ => 0.0,
        };
        let k = self.settings.zero_opinion_factor;
        score * (1.0 - k) + k * zero_score
      },
      _ => score,
    }
  }

  fn with_zero_opinions(
    &mut self,
    context: &str,
    scores: Vec<(NodeId, Weight)>,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{:?}", context);

    match self.zero_opinion.get(context) {
      Some(zero_opinion) => {
        if context.is_empty() {
          let k = self.settings.zero_opinion_factor;

          let mut res: Vec<(NodeId, Weight)> = vec![];
          res.resize(zero_opinion.len(), (0, 0.0));

          for (id, zero_score) in zero_opinion.iter().enumerate() {
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

          return res
            .into_iter()
            .filter(|(_id, score)| *score != 0.0)
            .collect::<Vec<_>>();
        }
      },
      _ => {},
    };

    return scores;
  }

  fn fetch_all_raw_scores(
    &mut self,
    context: &str,
    ego_id: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("{:?} {}", context, ego_id);

    if self.cache_walk_get(context, ego_id) {
      let graph = self.graph_from_ctx(context);
      match graph.get_ranks(ego_id, None) {
        Ok(scores) => {
          for (dst_id, score) in &scores {
            self.cache_score_add(context, ego_id, *dst_id, *score);
          }
          self.with_zero_opinions(context, scores)
        },
        Err(e) => {
          log_error!("{}", e);
          vec![]
        },
      }
    } else {
      let num_walks = self.settings.num_walks;

      match self
        .graph_from_ctx_mut(context)
        .calculate(ego_id, num_walks)
      {
        Ok(()) => {
          self.cache_walk_add(context, ego_id);
        },
        Err(e) => {
          log_error!("{}", e);
          return vec![];
        },
      }
      match self.graph_from_ctx(context).get_ranks(ego_id, None) {
        Ok(scores) => {
          for (dst_id, score) in &scores {
            self.cache_score_add(context, ego_id, *dst_id, *score);
          }
          self.with_zero_opinions(context, scores)
        },
        Err(e) => {
          log_error!("{}", e);
          vec![]
        },
      }
    }
  }

  fn fetch_raw_score(
    &mut self,
    context: &str,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> Weight {
    log_trace!("{:?} {} {}", context, ego_id, dst_id);

    let num_walks = self.settings.num_walks;

    if !self.cache_walk_get(context, ego_id) {
      if let Err(e) = self
        .graph_from_ctx_mut(context)
        .calculate(ego_id, num_walks)
      {
        log_error!("Failed to calculate: {}", e);
        return 0.0;
      }
      self.cache_walk_add(context, ego_id);
    }

    match self.graph_from_ctx(context).get_node_score(ego_id, dst_id) {
      Ok(score) => {
        self.cache_score_add(context, ego_id, dst_id, score);
        self.with_zero_opinion(context, dst_id, score)
      },
      Err(e) => {
        log_error!("Failed to get node score: {}", e);
        0.0
      },
    }
  }

  fn calculate_score_clusters_bounds(
    &mut self,
    context: &str,
    ego: NodeId,
    kind: NodeKind,
  ) -> [Weight; NUM_SCORE_QUANTILES - 1] {
    log_trace!("{:?} {} {:?}", ego, context, kind);

    let scores: Vec<Weight> = (0..self.node_count)
      .filter(|dst| self.node_info_from_id(*dst).kind == kind)
      .collect::<Vec<_>>()
      .into_iter()
      .map(|dst| self.fetch_raw_score(context, ego, dst))
      .filter(|score| *score >= EPSILON)
      .collect();

    if scores.is_empty() {
      return [0.0; NUM_SCORE_QUANTILES - 1];
    }

    calculate_quantiles_bounds(scores)
  }

  fn clusters_from(
    &mut self,
    context: &str,
  ) -> &mut Vec<ScoreClustersByKind> {
    log_trace!("{:?}", context);

    if !self.cached_score_clusters.contains_key(context) {
      self
        .cached_score_clusters
        .insert(context.to_string(), vec![]);
    }

    // We can safely unwrap here because we've just ensured the key exists
    self.cached_score_clusters.get_mut(context).unwrap()
  }

  fn update_node_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
    kind: NodeKind,
  ) {
    log_trace!("{:?} {} {:?}", context, ego, kind);

    let time = self.time_begin.elapsed().as_secs() as u64;
    let bounds = self.calculate_score_clusters_bounds(context, ego, kind);

    if ego >= self.node_count {
      log_error!("Node does not exist: {}", ego);
      return;
    }

    let node_count = self.node_count;

    let clusters = self.clusters_from(context);

    clusters.resize(node_count, Default::default());

    match kind {
      NodeKind::User => {
        clusters[ego].users.updated_sec = time;
        clusters[ego].users.bounds = bounds;
      },

      NodeKind::Beacon => {
        clusters[ego].beacons.updated_sec = time;
        clusters[ego].beacons.bounds = bounds;
      },

      NodeKind::Comment => {
        clusters[ego].comments.updated_sec = time;
        clusters[ego].comments.bounds = bounds;
      },

      NodeKind::Opinion => {
        clusters[ego].opinions.updated_sec = time;
        clusters[ego].opinions.bounds = bounds;
      },

      _ => {
        log_error!("Unknown node kind");
      },
    };
  }

  fn update_all_nodes_score_clustering(&mut self) {
    log_trace!();

    for (context, _) in self.contexts.clone() {
      for node_id in 0..self.node_count {
        self.update_node_score_clustering(
          context.as_str(),
          node_id,
          NodeKind::User,
        );
        self.update_node_score_clustering(
          context.as_str(),
          node_id,
          NodeKind::Beacon,
        );
        self.update_node_score_clustering(
          context.as_str(),
          node_id,
          NodeKind::Comment,
        );
        self.update_node_score_clustering(
          context.as_str(),
          node_id,
          NodeKind::Opinion,
        );
      }
    }
  }

  fn init_node_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
  ) {
    log_trace!("{:?} {}", context, ego);

    let node_count = self.node_count;

    let clusters = self.clusters_from(context);

    clusters.resize(node_count, Default::default());

    let users_empty = bounds_are_empty(&clusters[ego].users.bounds);
    let beacons_empty = bounds_are_empty(&clusters[ego].beacons.bounds);
    let comments_empty = bounds_are_empty(&clusters[ego].comments.bounds);
    let opinions_empty = bounds_are_empty(&clusters[ego].opinions.bounds);

    if users_empty {
      self.update_node_score_clustering(context, ego, NodeKind::User);
    }

    if beacons_empty {
      self.update_node_score_clustering(context, ego, NodeKind::Beacon);
    }

    if comments_empty {
      self.update_node_score_clustering(context, ego, NodeKind::Comment);
    }

    if opinions_empty {
      self.update_node_score_clustering(context, ego, NodeKind::Opinion);
    }
  }

  fn apply_score_clustering(
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

    if self.node_info_from_id(ego).kind != NodeKind::User {
      //  We apply score clustering only for user nodes.
      return (score, 0);
    }

    self.init_node_score_clustering(context, ego);

    let elapsed_secs = self.time_begin.elapsed().as_secs();

    let clusters = self.clusters_from(context);

    let updated_sec = match kind {
      NodeKind::User => clusters[ego].users.updated_sec,
      NodeKind::Beacon => clusters[ego].beacons.updated_sec,
      NodeKind::Comment => clusters[ego].comments.updated_sec,
      NodeKind::Opinion => clusters[ego].opinions.updated_sec,
      _ => {
        log_error!("Unknown node kind.");
        return (score, 0);
      },
    };

    if elapsed_secs >= updated_sec + self.settings.score_clusters_timeout {
      log_verbose!("Recalculate clustering for node {} in {:?}", ego, context);
      self.update_node_score_clustering(context, ego, kind);
    }

    let clusters = self.clusters_from(context);

    let bounds = match kind {
      NodeKind::User => &clusters[ego].users.bounds,
      NodeKind::Beacon => &clusters[ego].beacons.bounds,
      NodeKind::Comment => &clusters[ego].comments.bounds,
      NodeKind::Opinion => &clusters[ego].opinions.bounds,
      _ => {
        log_error!("Unknown node kind.");
        return (score, 0);
      },
    };

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

  fn fetch_all_scores(
    &mut self,
    context: &str,
    ego_id: NodeId,
  ) -> Vec<(NodeId, Weight, Cluster)> {
    log_trace!("{:?} {}", context, ego_id);

    self
      .fetch_all_raw_scores(context, ego_id)
      .iter()
      .map(|(dst_id, score)| {
        let kind = self.node_info_from_id(*dst_id).kind;
        (
          *dst_id,
          *score,
          self.apply_score_clustering(context, ego_id, *score, kind).1,
        )
      })
      .collect()
  }

  fn fetch_neighbors(
    &mut self,
    context: &str,
    ego: NodeId,
    dir: NeighborDirection,
  ) -> Vec<(NodeId, Weight, Cluster)> {
    log_trace!("{:?} {} {:?}", context, ego, dir);

    let mut v = vec![];

    match dir {
      NeighborDirection::Outbound => {
        match self.graph_from_ctx(context).graph.get_node_data(ego) {
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
          match self.graph_from_ctx(context).graph.get_node_data(src) {
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

    for i in 0..v.len() {
      let dst = v[i].0;
      let score = self.fetch_raw_score(context, ego, dst);
      let kind = self.node_info_from_id(dst).kind;
      let cluster = self.apply_score_clustering(context, ego, score, kind).1;

      v[i].1 = score;
      v[i].2 = cluster;
    }

    v
  }

  fn fetch_score(
    &mut self,
    context: &str,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, ego_id, dst_id);

    let score = self.fetch_raw_score(context, ego_id, dst_id);
    let kind = self.node_info_from_id(dst_id).kind;
    self.apply_score_clustering(context, ego_id, score, kind)
  }

  fn fetch_score_reversed(
    &mut self,
    context: &str,
    dst_id: NodeId,
    ego_id: NodeId,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, dst_id, ego_id);

    let score = match self.cache_score_get(context, ego_id, dst_id) {
      Some(score) => self.with_zero_opinion(context, dst_id, score),
      None => self.fetch_raw_score(context, ego_id, dst_id),
    };
    let kind = self.node_info_from_id(dst_id).kind;

    self.apply_score_clustering(context, ego_id, score, kind)
  }

  fn fetch_user_score_reversed(
    &mut self,
    context: &str,
    dst_id: NodeId,
    ego_id: NodeId,
  ) -> (Weight, Cluster) {
    log_trace!("{:?} {} {}", context, dst_id, ego_id);

    if self.node_info_from_id(ego_id).kind == NodeKind::User {
      return self.fetch_score_reversed(context, dst_id, ego_id);
    }

    match self.graph_from_ctx(context).graph.get_node_data(ego_id) {
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

          self.fetch_score_reversed(context, dst_id, parent_id)
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

    for (context, graph) in &mut self.contexts {
      if graph.graph.contains_node(node_id) {
        continue;
      }

      log_verbose!("Add node in {:?}: {}", context, node_name);

      //  HACK!!!
      while graph.get_new_nodeid() < node_id {}
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
    log_trace!("{:?} {:?} {:?} {}", context, src, dst, amount);

    if src == dst {
      log_error!("Self-reference is not allowed.");
      return;
    }

    if self.is_user_edge(src, dst) {
      // TODO: move this to the initializer
      self.graph_from_ctx("");
      if !context.is_empty() {
        self.graph_from_ctx(context);
      }

      for (enum_context, graph) in &mut self.contexts {
        log_verbose!(
          "Set user edge in {:?}: {} -> {} for {}",
          enum_context,
          src,
          dst,
          amount
        );
        graph.set_edge(src, dst, amount);
      }
    } else if context.is_empty() {
      log_verbose!("Set edge in \"\": {} -> {} for {}", src, dst, amount);
      self.graph_from_ctx_mut(context).set_edge(src, dst, amount);
    } else {
      let null_weight = self.edge_weight("", src, dst);
      let old_weight = self.edge_weight(context, src, dst);
      let delta = null_weight + amount - old_weight;

      log_verbose!("Set edge in \"\": {} -> {} for {}", src, dst, delta);
      self.graph_from_ctx_mut("").set_edge(src, dst, delta);

      log_verbose!(
        "Set edge in {:?}: {} -> {} for {}",
        context,
        src,
        dst,
        amount
      );
      self.graph_from_ctx_mut(context).set_edge(src, dst, amount);
    }
  }

  pub fn recalculate_all(
    &mut self,
    context: &str,
    num_walk: usize,
  ) {
    log_trace!("{}", num_walk);

    let infos = self.node_infos.clone();

    let graph = self.graph_from_ctx_mut(context);

    for id in 0..infos.len() {
      if (id % 100) == 90 {
        log_verbose!("{}%", (id * 100) / infos.len());
      }
      if infos[id].kind == NodeKind::User {
        match graph.calculate(id, num_walk) {
          Ok(_) => {},
          Err(e) => log_error!("{}", e),
        };
      }
    }
  }
}

//  ================================================
//
//    Commands
//
//  ================================================

pub fn read_version() -> &'static str {
  log_command!();
  VERSION
}

pub fn write_log_level(log_level: u32) {
  log_command!("{}", log_level);

  ERROR.store(log_level > 0, Ordering::Relaxed);
  WARNING.store(log_level > 1, Ordering::Relaxed);
  INFO.store(log_level > 2, Ordering::Relaxed);
  VERBOSE.store(log_level > 3, Ordering::Relaxed);
  TRACE.store(log_level > 4, Ordering::Relaxed);
}

impl AugMultiGraph {
  pub fn read_node_score(
    &mut self,
    context: &str,
    ego: &str,
    dst: &str,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?} {:?}", context, ego, dst);

    if !self.contexts.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    if !self.node_exists(ego) {
      log_error!("Node does not exist: {:?}", ego);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    if !self.node_exists(dst) {
      log_error!("Node does not exist: {:?}", dst);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0, 0)].to_vec();
    }

    let ego_id = self.find_or_add_node_by_name(ego);
    let dst_id = self.find_or_add_node_by_name(dst);

    let (score_of_dst_from_ego, score_cluster_of_dst) =
      self.fetch_score(context, ego_id, dst_id);
    let (score_of_ego_from_dst, score_cluster_of_ego) =
      self.fetch_user_score_reversed(context, ego_id, dst_id);

    [(
      ego.to_string(),
      dst.to_string(),
      score_of_dst_from_ego,
      score_of_ego_from_dst,
      score_cluster_of_dst,
      score_cluster_of_ego,
    )]
    .to_vec()
  }

  pub fn apply_filters_and_pagination(
    &mut self,
    scores: Vec<(NodeId, Weight, Cluster)>,
    context: &str,
    ego: &str,
    ego_id: NodeId,
    kind: NodeKind,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    let mut im: Vec<(NodeId, Weight, Cluster)> = scores
      .into_iter()
      .map(|(n, w, cluster)| (n, self.node_info_from_id(n).kind, w, cluster))
      .filter(|(_, target_kind, _, _)| {
        kind == NodeKind::Unknown || kind == *target_kind
      })
      .filter(|(_, _, score, _)| {
        score_gt < *score || (score_gte && score_gt <= *score)
      })
      .filter(|(_, _, score, _)| {
        *score < score_lt || (score_lte && score_lt >= *score)
      })
      .collect::<Vec<(NodeId, NodeKind, Weight, Cluster)>>()
      .into_iter()
      .filter(|(target_id, target_kind, _, _)| {
        if !hide_personal
          || (*target_kind != NodeKind::Comment
            && *target_kind != NodeKind::Beacon
            && *target_kind != NodeKind::Opinion)
        {
          return true;
        }
        match self
          .graph_from_ctx(context)
          .graph
          .edge_weight(*target_id, ego_id)
        {
          Ok(Some(_)) => false,
          _ => true,
        }
      })
      .map(|(target_id, _, score, cluster)| (target_id, score, cluster))
      .collect();

    im.sort_by(|(_, a, _), (_, b, _)| b.abs().total_cmp(&a.abs()));

    let index = index as usize;
    let count = count as usize;

    let mut page: Vec<(String, String, Weight, Weight, Cluster, Cluster)> =
      vec![];
    page.reserve_exact(if count < im.len() { count } else { im.len() });

    for i in index..count {
      if i >= im.len() {
        break;
      }

      let score_value_of_dst = im[i].1;
      let score_cluster_of_dst = im[i].2;

      let (score_value_of_ego, score_cluster_of_ego) =
        self.fetch_user_score_reversed(context, ego_id, im[i].0);

      page.push((
        ego.to_string(),
        self.node_info_from_id(im[i].0).name.clone(),
        score_value_of_dst,
        score_value_of_ego,
        score_cluster_of_dst,
        score_cluster_of_ego,
      ));
    }

    page
  }

  pub fn read_scores(
    &mut self,
    context: &str,
    ego: &str,
    kind_str: &str,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!(
      "{:?} {:?} {:?} {} {} {} {} {} {} {}",
      context,
      ego,
      kind_str,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count
    );

    let kind = match kind_from_prefix(kind_str) {
      Ok(x) => x,
      _ => {
        log_error!("Invalid node kind string: {:?}", kind_str);
        return vec![];
      },
    };

    if !self.contexts.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);

    let scores = self.fetch_all_scores(context, ego_id);

    return self.apply_filters_and_pagination(
      scores,
      context,
      ego,
      ego_id,
      kind,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
    );
  }

  pub fn read_neighbors(
    &mut self,
    context: &str,
    ego: &str,
    focus: &str,
    direction: i64,
    kind_str: &str,
    hide_personal: bool,
    score_lt: f64,
    score_lte: bool,
    score_gt: f64,
    score_gte: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!(
      "{:?} {} {} {} {:?} {} {} {} {} {} {} {}",
      context,
      ego,
      focus,
      direction,
      kind_str,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count
    );

    let kind = match kind_from_prefix(kind_str) {
      Ok(x) => x,
      _ => {
        log_error!("Invalid node kind string: {:?}", kind_str);
        return vec![];
      },
    };

    let dir = match neighbor_dir_from(direction) {
      Ok(x) => x,
      _ => {
        log_error!("Invalid neighbors direction: {}", direction);
        return vec![];
      },
    };

    let ego_id = self.find_or_add_node_by_name(ego);
    let focus_id = self.find_or_add_node_by_name(focus);

    let scores = self.fetch_neighbors(context, focus_id, dir);

    return self.apply_filters_and_pagination(
      scores,
      context,
      ego,
      ego_id,
      kind,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
    );
  }

  pub fn write_create_context(
    &mut self,
    context: &str,
  ) {
    log_command!("{:?}", context);
    self.create_context_if_does_not_exist(context);
  }

  pub fn write_put_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    new_weight: f64,
    magnitude: i64,
  ) {
    log_command!(
      "{:?} {:?} {:?} {} {}",
      context,
      src,
      dst,
      new_weight,
      magnitude
    );

    if magnitude < 0 {
      log_verbose!(
              "Negative magnitude detected: context={}, src={}, dst={}, magnitude={}. Converting to 0.",
              context, src, dst, magnitude
          );
    }

    let mag_clamped = magnitude.max(0) as u32;
    let src_id = self.find_or_add_node_by_name(src);
    let dst_id = self.find_or_add_node_by_name(dst);
    let (
      new_weight_scaled,
      mut new_min_weight,
      new_max_weight,
      new_mag_scale,
      rescale_factor,
    ) = self
      .vsids
      .scale_weight(context, src_id, new_weight, mag_clamped);

    // Check for small edges that need deletion
    let edge_deletion_threshold = new_max_weight * self.vsids.deletion_ratio;
    let can_delete_at_least_one_edge =
      new_min_weight <= edge_deletion_threshold;
    let must_rescale = rescale_factor > 1.0;
    // TODO: handle rewriting existing node case
    if can_delete_at_least_one_edge || must_rescale {
      // This means there is at least one edge to delete,
      // but maybe there is more, so we check everything.
      // In principle, we could have optimized this by storing the edges in a sorted heap structure.
      //new_min_weight = new_max_weight;
      let (edges_to_modify, new_min_weight_from_scan) = self
        .graph_from_ctx(context)
        .graph
        .get_node_data(src_id)
        .unwrap()
        .get_outgoing_edges()
        .fold(
          (Vec::new(), new_min_weight),
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
              (to_modify, min.min(abs_weight))
            }
          },
        );
      new_min_weight = new_min_weight_from_scan;

      for (dst_id, weight) in edges_to_modify {
        log_verbose!(
          "Rescale or delete node: context={:?}, src={}, dst={}, new_weight={}",
          context,
          self.node_info_from_id(src_id).name,
          self.node_info_from_id(dst_id).name,
          weight
        );
        self.set_edge(context, src_id, dst_id, weight);
      }
    }
    self.set_edge(context, src_id, dst_id, new_weight_scaled);
    if must_rescale {
      log_verbose!(
          "Rescale performed: context={:?}, src={}, dst={}, normalized_new_weight={}",
          context,src,dst, new_weight_scaled);
    } else {
      log_verbose!(
          "Edge updated without rescale: context={:?}, src={}, dst={}, new_weight_scaled={}",
          context,src,dst,new_weight_scaled);
    }
    self.vsids.min_max_weights.insert(
      (context.to_string(), src_id),
      (new_min_weight, new_max_weight, new_mag_scale),
    );
  }

  pub fn write_delete_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    _index: i64,
  ) {
    log_command!("{:?} {:?} {:?}", context, src, dst);

    if !self.node_exists(src) || !self.node_exists(dst) {
      return;
    }

    let src_id = self.find_or_add_node_by_name(src);
    let dst_id = self.find_or_add_node_by_name(dst);

    self.set_edge(context, src_id, dst_id, 0.0);
  }

  pub fn write_delete_node(
    &mut self,
    context: &str,
    node: &str,
    _index: i64,
  ) {
    log_command!("{:?} {:?}", context, node);

    if !self.node_exists(node) {
      return;
    }

    let id = self.find_or_add_node_by_name(node);

    // Collect the outgoing edges first
    let outgoing_edges: Vec<NodeId> = self
      .graph_from_ctx(context)
      .graph
      .get_node_data(id)
      .map(|data| {
        data
          .get_outgoing_edges()
          .into_iter()
          .map(|(n, _)| n)
          .collect()
      })
      .unwrap();

    // Then remove the edges
    for n in outgoing_edges {
      self.set_edge(context, id, n, 0.0);
    }
  }

  pub fn read_graph(
    &mut self,
    context: &str,
    ego: &str,
    focus: &str,
    positive_only: bool,
    index: u32,
    count: u32,
  ) -> Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)> {
    log_command!(
      "{:?} {:?} {:?} {} {} {}",
      context,
      ego,
      focus,
      positive_only,
      index,
      count
    );

    if !self.contexts.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    if !self.node_exists(ego) {
      log_error!("Node does not exist: {:?}", ego);
      return vec![];
    }

    if !self.node_exists(focus) {
      log_error!("Node does not exist: {:?}", focus);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);
    let focus_id = self.find_or_add_node_by_name(focus);

    let mut indices = HashMap::<NodeId, NodeIndex>::new();
    let mut ids = HashMap::<NodeIndex, NodeId>::new();
    let mut im_graph = DiGraph::<NodeId, Weight>::new();

    {
      let index = im_graph.add_node(focus_id);
      indices.insert(focus_id, index);
      ids.insert(index, focus_id);
    }

    log_verbose!("Enumerate focus neighbors");

    let focus_neighbors =
      self.all_outbound_neighbors_normalized(context, focus_id);

    for (dst_id, focus_dst_weight) in focus_neighbors {
      let dst_kind = self.node_info_from_id(dst_id).kind;

      if dst_kind == NodeKind::User {
        if positive_only && self.fetch_raw_score(context, ego_id, dst_id) <= 0.0
        {
          continue;
        }

        if !indices.contains_key(&dst_id) {
          let index = im_graph.add_node(focus_id);
          indices.insert(dst_id, index);
          ids.insert(index, dst_id);
        }

        if let (Some(focus_idx), Some(dst_idx)) =
          (indices.get(&focus_id), indices.get(&dst_id))
        {
          im_graph.add_edge(*focus_idx, *dst_idx, focus_dst_weight);
        } else {
          log_error!("Got invalid node id");
        }
      } else if dst_kind == NodeKind::Comment
        || dst_kind == NodeKind::Beacon
        || dst_kind == NodeKind::Opinion
      {
        let dst_neighbors =
          self.all_outbound_neighbors_normalized(context, dst_id);

        for (ngh_id, dst_ngh_weight) in dst_neighbors {
          if (positive_only && dst_ngh_weight <= 0.0)
            || ngh_id == focus_id
            || self.node_info_from_id(ngh_id).kind != NodeKind::User
          {
            continue;
          }

          // Calculate the weight of the edge from focus to this neighbor
          let focus_ngh_weight = focus_dst_weight
            * dst_ngh_weight
            * if focus_dst_weight < 0.0 && dst_ngh_weight < 0.0 {
              -1.0
            } else {
              1.0
            };

          // Calculate the weight of the edge from focus to this neighbor
          if !indices.contains_key(&ngh_id) {
            let index = im_graph.add_node(ngh_id);
            indices.insert(ngh_id, index);
            ids.insert(index, ngh_id);
          }

          // Calculate the weight of the edge from focus to this neighbor
          if let (Some(focus_idx), Some(ngh_idx)) =
            (indices.get(&focus_id), indices.get(&ngh_id))
          {
            im_graph.add_edge(*focus_idx, *ngh_idx, focus_ngh_weight);
          } else {
            log_error!("Got invalid node id");
          }
        }
      }
    }

    if ego_id == focus_id {
      log_verbose!("Ego is same as focus");
    } else {
      log_verbose!("Search shortest path");

      let graph_cloned = self.graph_from_ctx(context).graph.clone();

      //  ================================
      //
      //    A* search
      //

      let mut open: Vec<Node<NodeId, Weight>> = vec![];
      let mut closed: Vec<Node<NodeId, Weight>> = vec![];

      open.resize(1024, Node::default());
      closed.resize(1024, Node::default());

      let mut astar_state = init(&mut open, ego_id, focus_id, 0.0);

      let mut steps = 0;
      let mut neighbor = None;
      let mut status = Status::PROGRESS;

      //  Do 10000 iterations max

      for _ in 0..10000 {
        steps += 1;

        status =
          iteration(&mut open, &mut closed, &mut astar_state, neighbor.clone());

        match status.clone() {
          Status::NEIGHBOR(request) => {
            match graph_cloned.get_node_data(request.node) {
              None => neighbor = None,
              Some(data) => {
                let kv: Vec<_> =
                  data.pos_edges.iter().skip(request.index).take(1).collect();

                if kv.is_empty() {
                  neighbor = None;
                } else {
                  let n = kv[0].0;
                  let mut w = *kv[0].1;

                  if data.pos_sum > EPSILON {
                    w /= data.pos_sum;
                  }

                  neighbor = Some(Link::<NodeId, Weight> {
                    neighbor:       *n,
                    exact_distance: if w.abs() < EPSILON {
                      1_000_000.0
                    } else {
                      1.0 / w
                    },
                    estimate:       0.0,
                  });
                }
              },
            }
          },
          Status::OUT_OF_MEMORY => {
            open.resize(open.len() * 2, Node::default());
            closed.resize(closed.len() * 2, Node::default());
          },
          Status::SUCCESS => break,
          Status::FAIL => break,
          Status::PROGRESS => {},
        };
      }

      log_verbose!("Did {} A* iterations", steps);

      if status == Status::SUCCESS {
        log_verbose!("Path found");
      } else if status == Status::FAIL {
        log_error!("Path does not exist from {} to {}", ego_id, focus_id);
        return vec![];
      } else {
        log_error!("Unable to find a path from {} to {}", ego_id, focus_id);
        return vec![];
      }

      let mut ego_to_focus: Vec<NodeId> = vec![];
      ego_to_focus.resize(astar_state.num_closed, 0);
      let n = path(&closed, &astar_state, &mut ego_to_focus);
      ego_to_focus.resize(n, 0);

      for node in ego_to_focus.iter() {
        log_verbose!("Path: {}", self.node_info_from_id(*node).name);
      }

      //  ================================

      let mut edges = Vec::<(NodeId, NodeId, Weight)>::new();
      edges.reserve_exact(ego_to_focus.len() - 1);

      log_verbose!("Process shortest path");

      for k in 0..ego_to_focus.len() - 1 {
        let a = ego_to_focus[k];
        let b = ego_to_focus[k + 1];

        let a_kind = self.node_info_from_id(a).kind;
        let b_kind = self.node_info_from_id(b).kind;

        let a_b_weight = self.edge_weight_normalized(context, a, b);

        if k + 2 == ego_to_focus.len() {
          if a_kind == NodeKind::User {
            edges.push((a, b, a_b_weight));
          } else {
            log_verbose!("Ignore node {}", self.node_info_from_id(a).name);
          }
        } else if b_kind != NodeKind::User {
          log_verbose!("Ignore node {}", self.node_info_from_id(b).name);
          let c = ego_to_focus[k + 2];
          let b_c_weight = self.edge_weight_normalized(context, b, c);
          let a_c_weight = a_b_weight
            * b_c_weight
            * if a_b_weight < 0.0 && b_c_weight < 0.0 {
              -1.0
            } else {
              1.0
            };
          edges.push((a, c, a_c_weight));
        } else if a_kind == NodeKind::User {
          edges.push((a, b, a_b_weight));
        } else {
          log_verbose!("Ignore node {}", self.node_info_from_id(a).name);
        }
      }

      log_verbose!("Add path to the graph");

      for (src, dst, weight) in edges {
        if !indices.contains_key(&src) {
          let index = im_graph.add_node(src);
          indices.insert(src, index);
          ids.insert(index, src);
        }

        if !indices.contains_key(&dst) {
          let index = im_graph.add_node(dst);
          indices.insert(dst, index);
          ids.insert(index, dst);
        }

        if let (Some(src_idx), Some(dst_idx)) =
          (indices.get(&src), indices.get(&dst))
        {
          im_graph.add_edge(*src_idx, *dst_idx, weight);
        } else {
          log_error!("Got invalid node id");
        }
      }
    }

    log_verbose!("Remove self references");

    for (_, src_index) in indices.iter() {
      let neighbors: Vec<_> = im_graph
        .edges(*src_index)
        .map(|edge| (edge.target(), edge.id()))
        .collect();

      for (dst_index, edge_id) in neighbors {
        if *src_index == dst_index {
          im_graph.remove_edge(edge_id);
        }
      }
    }

    let mut edge_ids = Vec::<(NodeId, NodeId, Weight)>::new();
    edge_ids.reserve_exact(indices.len() * 2); // ad hok

    log_verbose!("Build final array");

    for (_, src_index) in indices {
      for edge in im_graph.edges(src_index) {
        if let (Some(src_id), Some(dst_id)) =
          (ids.get(&src_index), ids.get(&edge.target()))
        {
          let w = *edge.weight();
          if w > -EPSILON && w < EPSILON {
            log_error!(
              "Got zero edge weight: {} -> {}",
              self.node_info_from_id(*src_id).name.clone(),
              self.node_info_from_id(*dst_id).name.clone()
            );
          } else {
            let mut found = false;
            for (x, y, _) in edge_ids.iter() {
              if *src_id == *x && *dst_id == *y {
                found = true;
                break;
              }
            }
            if !found {
              edge_ids.push((*src_id, *dst_id, w));
            }
          }
        } else {
          log_error!("Got invalid node index");
        }
      }
    }

    edge_ids.sort_by(|(_, _, a), (_, _, b)| b.abs().total_cmp(&a.abs()));

    edge_ids
      .into_iter()
      .skip(index as usize)
      .take(count as usize)
      .map(|(src_id, dst_id, weight_of_dst)| {
        let (score_value_of_dst, score_cluster_of_dst) =
          self.fetch_score(context, ego_id, dst_id);
        let (score_value_of_ego, score_cluster_of_ego) =
          self.fetch_user_score_reversed(context, ego_id, dst_id);

        (
          self.node_info_from_id(src_id).name.clone(),
          self.node_info_from_id(dst_id).name.clone(),
          weight_of_dst,
          score_value_of_dst,
          score_value_of_ego,
          score_cluster_of_dst,
          score_cluster_of_ego,
        )
      })
      .collect()
  }

  pub fn read_connected(
    &mut self,
    context: &str,
    ego: &str,
  ) -> Vec<(String, String)> {
    log_command!("{:?} {:?}", context, ego);

    if !self.contexts.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    if !self.node_exists(ego) {
      log_error!("Node does not exist: {:?}", ego);
      return vec![];
    }

    let src_id = self.find_or_add_node_by_name(ego);

    let outgoing_edges: Vec<_> = self
      .graph_from_ctx(context)
      .graph
      .get_node_data(src_id)
      .unwrap()
      .get_outgoing_edges()
      .collect();

    outgoing_edges
      .into_iter()
      .map(|(dst_id, _)| {
        (ego.to_string(), self.node_info_from_id(dst_id).name.clone())
      })
      .collect()
  }

  pub fn read_node_list(&self) -> Vec<(String,)> {
    log_command!();

    self
      .node_infos
      .iter()
      .map(|info| (info.name.clone(),))
      .collect()
  }

  pub fn read_edges(
    &mut self,
    context: &str,
  ) -> Vec<(String, String, Weight)> {
    log_command!("{:?}", context);

    if !self.contexts.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    let infos = self.node_infos.clone();

    let mut v: Vec<(String, String, Weight)> = vec![];
    v.reserve(infos.len() * 2); // ad hok

    for src_id in 0..infos.len() {
      let src_name = infos[src_id].name.as_str();

      match self.graph_from_ctx(context).graph.get_node_data(src_id) {
        Some(data) => {
          for (dst_id, weight) in data.get_outgoing_edges() {
            match infos.get(dst_id) {
              Some(x) => v.push((src_name.to_string(), x.name.clone(), weight)),
              None => log_error!("Node does not exist: {}", dst_id),
            }
          }
        },
        _ => {},
      };
    }

    v
  }

  pub fn read_mutual_scores(
    &mut self,
    context: &str,
    ego: &str,
  ) -> Vec<(String, String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?}", context, ego);

    if !self.contexts.contains_key(context) {
      log_error!("Context does not exist: {:?}", context);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);
    let ranks = self.fetch_all_scores(context, ego_id);
    let mut v =
      Vec::<(String, String, Weight, Weight, Cluster, Cluster)>::new();

    v.reserve_exact(ranks.len());

    for (node, score_value_of_dst, score_cluster_of_dst) in ranks {
      let info = self.node_info_from_id(node).clone();
      if score_value_of_dst > 0.0 && info.kind == NodeKind::User {
        let (score_value_of_ego, score_cluster_of_ego) =
          self.fetch_user_score_reversed(context, ego_id, node);

        v.push((
          ego.to_string(),
          info.name,
          score_value_of_dst,
          score_value_of_ego,
          score_cluster_of_dst,
          score_cluster_of_ego,
        ));
      }
    }

    v
  }

  pub fn write_reset(&mut self) {
    log_command!();

    self.reset();
  }

  pub fn read_new_edges_filter(
    &mut self,
    src: &str,
  ) -> Vec<u8> {
    log_command!("{:?}", src);

    if !self.node_exists(src) {
      log_error!("Node does not exist: {:?}", src);
      return vec![];
    }

    let src_id = self.find_or_add_node_by_name(src);

    let mut v: Vec<u8> = vec![];
    v.reserve_exact(self.node_infos[src_id].seen_nodes.len() * 8);

    for &x in &self.node_infos[src_id].seen_nodes {
      for i in 0..8 {
        v.push((x & (0xff << (8 * i)) >> (8 * i)) as u8);
      }
    }

    return v;
  }

  pub fn write_new_edges_filter(
    &mut self,
    src: &str,
    filter_bytes: &[u8],
  ) {
    log_command!("{:?} {:?}", src, filter_bytes);

    let src_id = self.find_or_add_node_by_name(src);

    let mut v: Vec<u64> = vec![];
    v.resize(((filter_bytes.len() + 7) / 8) * 8, 0);

    for i in 0..filter_bytes.len() {
      v[i / 8] = (filter_bytes[i] as u64) << (8 * (i % 8));
    }

    self.node_infos[src_id].seen_nodes = v;
  }

  pub fn write_fetch_new_edges(
    &mut self,
    src: &str,
    prefix: &str,
  ) -> Vec<(String, Weight, Weight, Cluster, Cluster)> {
    log_command!("{:?} {:?}", src, prefix);

    let num_hashes = self.settings.filter_num_hashes;
    let max_size = self.settings.filter_max_size / 8;

    let src_id = self.find_or_add_node_by_name(src);

    if self.node_infos[src_id].seen_nodes.is_empty() {
      self.node_infos[src_id]
        .seen_nodes
        .resize((self.settings.filter_min_size + 7) / 8, 0);

      log_verbose!(
        "Create the bloom filter with {} bytes for {:?}",
        8 * self.node_infos[src_id].seen_nodes.len(),
        src
      );
    }

    //  Fetch new edges
    //

    let mut v: Vec<(String, Weight, Weight, Cluster, Cluster)> = vec![];

    for dst_id in 0..self.node_count {
      //  FIXME Probably we should use NodeKind here.
      if !self.node_infos[dst_id].name.starts_with(prefix) {
        continue;
      }

      let (score_value_of_dst, score_cluster_of_dst) =
        self.fetch_score("", src_id, dst_id);
      let (score_value_of_src, score_cluster_of_src) =
        self.fetch_score_reversed("", src_id, dst_id);

      if score_value_of_dst < EPSILON {
        continue;
      }

      let bits = bloom_filter_bits(
        self.node_infos[src_id].seen_nodes.len(),
        num_hashes,
        dst_id,
      );

      if !bloom_filter_contains(&self.node_infos[src_id].seen_nodes, &bits) {
        v.push((
          self.node_infos[dst_id].name.clone(),
          score_value_of_dst,
          score_value_of_src,
          score_cluster_of_dst,
          score_cluster_of_src,
        ));
      }
    }

    //  Rebuild the bloom filter
    //

    let mut seen_nodes = vec![];

    seen_nodes.resize(
      std::cmp::min(self.node_infos[src_id].seen_nodes.len(), max_size),
      0,
    );

    loop {
      let mut saturated = false;

      for x in seen_nodes.iter_mut() {
        *x = 0;
      }

      for dst_id in 0..self.node_count {
        let bits = bloom_filter_bits(seen_nodes.len(), num_hashes, dst_id);
        let collision = bloom_filter_contains(&mut seen_nodes, &bits);

        if collision && seen_nodes.len() < max_size {
          //  Resize the bloom filter if it is saturated

          let n = seen_nodes.len() * 2;
          seen_nodes.resize(n, 0);

          log_verbose!(
            "Resize the bloom filter to {} bytes for {:?}",
            8 * n,
            src
          );

          saturated = true;
          break;
        }

        //  FIXME Probably we should use NodeKind here.
        if self.node_infos[dst_id].name.starts_with(prefix) {
          let score = self.fetch_raw_score("", src_id, dst_id);

          if !(score < EPSILON) {
            bloom_filter_add(&mut seen_nodes, &bits);
          }
        } else {
          //  RUST!!!
          let len = self.node_infos[src_id].seen_nodes.len();

          let already_seen = bloom_filter_contains(
            &mut self.node_infos[src_id].seen_nodes,
            &bloom_filter_bits(len, num_hashes, dst_id),
          );

          if already_seen {
            bloom_filter_add(&mut seen_nodes, &bits);
          }
        }
      }

      if !saturated {
        if seen_nodes.len() >= max_size {
          log_warning!("Max bloom filer size is reached for {:?}", src);
        }

        self.node_infos[src_id].seen_nodes = seen_nodes;
        break;
      }
    }

    //  Return fetched edges
    //

    return v;
  }

  pub fn write_set_zero_opinion(
    &mut self,
    context: &str,
    node: &str,
    score: Weight,
  ) {
    log_command!("{:?} {} {}", context, node, score);

    let id = self.find_or_add_node_by_name(node);

    match self.zero_opinion.get_mut(context) {
      Some(zero_opinion) => {
        if id >= zero_opinion.len() {
          zero_opinion.resize(id + 1, 0.0);
        }

        zero_opinion[id] = score;
      },
      None => {
        let mut v = vec![];
        v.resize(id + 1, 0.0);
        v[id] = score;
        self.zero_opinion.insert(context.to_string(), v);
      },
    };
  }
}

//  ================================================
//
//    Zero opinion recalculation
//
//  ================================================

impl AugMultiGraph {
  pub fn reduced_graph(
    &mut self,
    context: &str,
  ) -> Vec<(NodeId, NodeId, Weight)> {
    log_trace!();

    let mut all_edges = vec![];

    for src in 0..self.node_infos.len() {
      match self.graph_from_ctx(context).graph.get_node_data(src) {
        Some(data) => {
          for (dst, _) in &data.pos_edges {
            all_edges.push((src, *dst));
          }

          for (dst, _) in &data.neg_edges {
            all_edges.push((src, *dst));
          }
        },
        _ => {},
      }
    }

    let users: Vec<NodeId> = self
      .node_infos
      .iter()
      .enumerate()
      .filter(|(_id, info)| info.kind == NodeKind::User)
      .filter(|(id, _info)| {
        for (src, dst) in &all_edges {
          if *id == *src || *id == *dst {
            return true;
          }
        }
        return false;
      })
      .map(|(id, _info)| id)
      .collect();

    if users.is_empty() {
      return vec![];
    }

    let num_walks = self.settings.num_walks;

    for id in users.iter() {
      match self.graph_from_ctx_mut(context).calculate(*id, num_walks) {
        Ok(_) => {},
        Err(e) => log_error!("{}", e),
      };
    }

    let edges: Vec<(NodeId, NodeId, Weight)> = users
      .into_iter()
      .map(|id| -> Vec<(NodeId, NodeId, Weight)> {
        self
          .fetch_all_raw_scores(context, id)
          .into_iter()
          .map(|(node_id, score)| (id, node_id, score))
          .filter(|(ego_id, node_id, score)| {
            let kind = self.node_info_from_id(*node_id).kind;

            (kind == NodeKind::User || kind == NodeKind::Beacon)
              && *score > EPSILON
              && ego_id != node_id
          })
          .collect()
      })
      .flatten()
      .collect();

    let result: Vec<(NodeId, NodeId, Weight)> = edges
      .into_iter()
      .map(|(ego_id, dst_id, weight)| {
        let ego_kind = self.node_info_from_id(ego_id).kind;
        let dst_kind = self.node_info_from_id(dst_id).kind;
        (ego_id, ego_kind, dst_id, dst_kind, weight)
      })
      .filter(|(ego_id, ego_kind, dst_id, dst_kind, _)| {
        ego_id != dst_id
          && *ego_kind == NodeKind::User
          && (*dst_kind == NodeKind::User || *dst_kind == NodeKind::Beacon)
      })
      .map(|(ego_id, _, dst_id, _, weight)| (ego_id, dst_id, weight))
      .collect();

    result
  }

  pub fn top_nodes(
    &mut self,
    context: &str,
  ) -> Vec<(NodeId, f64)> {
    log_trace!();

    let reduced = self.reduced_graph(context);

    if reduced.is_empty() {
      log_error!("Reduced graph is empty");
      return vec![];
    }

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
        .take(self.settings.top_nodes_limit)
        .into_iter()
        .unzip();

    let res = nodes.into_iter().zip(scores).collect::<Vec<_>>();

    if res.is_empty() {
      log_error!("No top nodes");
    }

    return res;
  }

  pub fn write_recalculate_zero(&mut self) {
    log_command!();

    for (context, graph) in &self.contexts.clone() {
      //  Save the current state of the graph
      let graph_bak = graph.clone();

      self.recalculate_all(context, 0);
      let nodes = self.top_nodes(context);

      //  Drop all walks and make sure to empty caches.
      self.recalculate_all(context, 0);
      self.score_cache = create_score_cache(self.settings.scores_cache_size);
      self.cached_walks = create_cached_walks(self.settings.walks_cache_size);

      let mut zero_opinion = vec![];

      zero_opinion.resize(0, 0.0);
      zero_opinion.reserve(nodes.len());

      for (node_id, amount) in nodes.iter() {
        if *node_id >= zero_opinion.len() {
          zero_opinion.resize(*node_id + 1, 0.0);
        }
        zero_opinion[*node_id] = *amount;
      }

      self.zero_opinion.insert(context.to_string(), zero_opinion);

      //  Reset the graph
      self.contexts.insert(context.clone(), graph_bak);
    }
  }

  pub fn write_recalculate_clustering(&mut self) {
    log_command!();

    self.update_all_nodes_score_clustering();
  }
}
