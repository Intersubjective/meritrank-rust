use meritrank_core::{constants::EPSILON, Graph, MeritRank, NodeId};
use petgraph::{
  graph::{DiGraph, NodeIndex},
  visit::EdgeRef,
};
use rand::{thread_rng, Rng};
use simple_pagerank::Pagerank;
use std::{
  collections::HashMap, env::var, string::ToString, sync::atomic::Ordering,
  time::Instant,
};

use crate::astar::*;
use crate::log::*;
use crate::log_error;
use crate::log_info;
use crate::log_trace;
use crate::log_verbose;
use crate::log_warning;

pub use meritrank_core::Weight;
pub type Cluster = f64;

//  ================================================================
//
//    Constants
//
//  ================================================================

pub const VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
  Some(x) => x,
  None => "dev",
};

fn parse_env_with_default<T>(
  name: &str,
  default: T,
) -> T
where
  T: std::str::FromStr,
{
  var(name)
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(default)
}

fn parse_env_with_transform<T, F>(
  name: &str,
  default: T,
  transform: F,
) -> T
where
  T: std::str::FromStr,
  F: FnOnce(T) -> Option<T>,
{
  var(name)
    .ok()
    .and_then(|s| s.parse().ok())
    .and_then(transform)
    .unwrap_or(default)
}

lazy_static::lazy_static! {
    pub static ref ZERO_OPINION_FACTOR: usize = parse_env_with_default("MERITRANK_ZERO_OPINION_FACTOR", 20);
    pub static ref NUM_WALK: usize = parse_env_with_default("MERITRANK_NUM_WALK", 10000);
    pub static ref TOP_NODES_LIMIT: usize = parse_env_with_default("MERITRANK_TOP_NODES_LIMIT", 100);
    pub static ref FILTER_NUM_HASHES: usize = parse_env_with_default("MERITRANK_FILTER_NUM_HASHES", 10);
    pub static ref FILTER_MIN_SIZE: usize = parse_env_with_transform("MERITRANK_FILTER_MIN_SIZE",32,|n| Some(std::cmp::max(n, 1)));
    pub static ref FILTER_MAX_SIZE: usize = parse_env_with_default("MERITRANK_FILTER_MAX_SIZE", 8192);
    pub static ref SCORES_CACHE_SIZE: usize = parse_env_with_default("MERITRANK_SCORES_CACHE_SIZE", 1024 * 10);
    pub static ref WALKS_CACHE_SIZE: usize = parse_env_with_default("MERITRANK_WALKS_CACHE_SIZE", 1024);
    pub static ref NUM_SCORE_CLUSTERS: usize = parse_env_with_default("MERITRANK_NUM_SCORE_CLUSTERS", 5);
    pub static ref SCORE_CLUSTERS_TIMEOUT: u64 = parse_env_with_default("MERITRANK_SCORE_CLUSTERS_TIMEOUT", 60 * 60 * 6);
    pub static ref KMEANS_TOLERANCE: f64 = parse_env_with_default("MERITRANK_KMEANS_TOLERANCE", 0.01);
    pub static ref KMEANS_ITERATIONS: usize = parse_env_with_default("MERITRANK_KMEANS_ITERATIONS", 200);
}

//  ================================================================
//
//    Basic declarations
//
//  ================================================================

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum NodeKind {
  #[default]
  Unknown,
  User,
  Beacon,
  Comment,
}

#[derive(PartialEq, Clone, Default)]
pub struct NodeInfo {
  pub kind: NodeKind,
  pub name: String,

  // Bloom filter of nodes marked as seen by this node in the null context
  pub seen_nodes: Vec<u64>,
}

#[derive(PartialEq, Clone, Default)]
pub struct CachedScore {
  pub context: String,
  pub ego:     NodeId,
  pub dst:     NodeId,
  pub score:   Weight,
}

#[derive(PartialEq, Eq, Clone, Default)]
pub struct CachedWalk {
  pub context: String,
  pub ego:     NodeId,
}

#[derive(PartialEq, Clone, Default)]
pub struct ClusterGroupBounds {
  pub updated_sec: u64,
  pub bounds:      Vec<Weight>,
}

//  Augmented multi-context graph
//
#[derive(Clone)]
pub struct AugMultiGraph {
  pub node_count:            usize,
  pub node_infos:            Vec<NodeInfo>,
  pub node_ids:              HashMap<String, NodeId>,
  pub contexts:              HashMap<String, MeritRank>,
  pub cached_scores:         Vec<CachedScore>,
  pub cached_walks:          Vec<CachedWalk>,
  pub zero_opinion:          Vec<Weight>,
  pub time_begin:            Instant,
  pub cached_score_clusters: HashMap<String, Vec<ClusterGroupBounds>>,

  pub dummy_info:     NodeInfo,
  pub dummy_graph:    MeritRank,
  pub dummy_clusters: Vec<ClusterGroupBounds>,
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
    log_error!("(bloom_filter_add) Invalid arguments");
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
    log_error!("(bloom_filter_contains) Invalid arguments");
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
    log_trace!("cache_score_add {:?} {} {} {}", context, ego, dst, score);

    for (i, x) in self.cached_scores.iter().enumerate().rev() {
      if x.ego == ego && x.dst == dst && x.context == context {
        self.cached_scores.remove(i);
        self.cached_scores.push(CachedScore {
          context: context.to_string(),
          ego,
          dst,
          score,
        });
        return;
      }
    }

    if self.cached_scores.len() >= *SCORES_CACHE_SIZE {
      log_trace!(
        "drop score {:?}, {} -> {}",
        self.cached_scores[0].context,
        self.cached_scores[0].ego,
        self.cached_scores[0].dst
      );
      self.cached_scores.remove(0);
    }

    self.cached_scores.push(CachedScore {
      context: context.to_string(),
      ego,
      dst,
      score,
    });
  }

  pub fn cache_score_get(
    &mut self,
    context: &str,
    ego: NodeId,
    dst: NodeId,
  ) -> Option<Weight> {
    log_trace!("cache_score_get {:?} {} {}", context, ego, dst);

    for (i, x) in self.cached_scores.iter().enumerate().rev() {
      if x.ego == ego && x.dst == dst && x.context == context {
        let score = x.score;
        self.cached_scores.remove(i);
        self.cached_scores.push(CachedScore {
          context: context.to_string(),
          ego,
          dst,
          score,
        });
        return Some(score);
      }
    }

    return None;
  }

  pub fn cache_walk_add(
    &mut self,
    context: &str,
    ego: NodeId,
  ) {
    log_trace!("cache_walk_add {:?} {}", context, ego);

    for (i, x) in self.cached_walks.iter().enumerate().rev() {
      if x.ego == ego && x.context == context {
        self.cached_walks.remove(i);
        self.cached_walks.push(CachedWalk {
          context: context.to_string(),
          ego,
        });
        return;
      }
    }

    if self.cached_walks.len() >= *WALKS_CACHE_SIZE {
      log_trace!(
        "drop walks {:?}, {}",
        self.cached_walks[0].context,
        self.cached_walks[0].ego
      );

      //  HACK!!!
      //  We "drop" the walks by recalculating the node with 0.
      let drop_walk = self.cached_walks[0].clone(); // RUST!!!
      match self
        .graph_from(drop_walk.context.as_str())
        .calculate(drop_walk.ego, 0)
      {
        Ok(()) => {},
        Err(e) => {
          log_error!("(cache_walk_add) {}", e);
        },
      }

      self.cached_walks.remove(0);
    }

    self.cached_walks.push(CachedWalk {
      context: context.to_string(),
      ego,
    });
  }

  pub fn cache_walk_get(
    &mut self,
    context: &str,
    ego: NodeId,
  ) -> bool {
    log_trace!("cache_walk_get");

    for (i, x) in self.cached_walks.iter().enumerate().rev() {
      if x.ego == ego && x.context == context {
        self.cached_walks.remove(i);
        self.cached_walks.push(CachedWalk {
          context: context.to_string(),
          ego,
        });
        return true;
      }
    }

    return false;
  }
}

//  ================================================================
//
//    Utils
//
//  ================================================================

pub fn kind_from_name(name: &str) -> NodeKind {
  log_trace!("kind_from_name: {:?}", name);

  match name.chars().nth(0) {
    Some('U') => NodeKind::User,
    Some('B') => NodeKind::Beacon,
    Some('C') => NodeKind::Comment,
    _ => NodeKind::Unknown,
  }
}

impl Default for AugMultiGraph {
  fn default() -> AugMultiGraph {
    AugMultiGraph::new()
  }
}

impl AugMultiGraph {
  pub fn new() -> AugMultiGraph {
    log_trace!("AugMultiGraph::new");

    AugMultiGraph {
      node_count:            0,
      node_infos:            Vec::new(),
      node_ids:              HashMap::new(),
      contexts:              HashMap::new(),
      cached_scores:         vec![],
      cached_walks:          vec![],
      zero_opinion:          vec![],
      time_begin:            Instant::now(),
      cached_score_clusters: HashMap::new(),
      dummy_info:            Default::default(),
      dummy_graph:           MeritRank::new(Graph::new()),
      dummy_clusters:        vec![],
    }
  }

  pub fn copy_from(
    &mut self,
    other: &AugMultiGraph,
  ) {
    log_trace!("copy_from");

    self.node_count = other.node_count;
    self.node_infos = other.node_infos.clone();
    self.node_ids = other.node_ids.clone();
    self.contexts = other.contexts.clone();
    self.cached_scores = other.cached_scores.clone();
    self.cached_walks = other.cached_walks.clone();
    self.zero_opinion = other.zero_opinion.clone();
    self.time_begin = other.time_begin.clone();
    self.cached_score_clusters = other.cached_score_clusters.clone();
  }

  pub fn reset(&mut self) {
    log_trace!("reset");

    self.node_count = 0;
    self.node_infos = vec![];
    self.node_ids = HashMap::new();
    self.contexts = HashMap::new();
    self.cached_scores = vec![];
    self.cached_walks = vec![];
    self.zero_opinion = vec![];
    self.time_begin = Instant::now();
    self.cached_score_clusters = HashMap::new();
  }

  pub fn node_exists(
    &self,
    node_name: &str,
  ) -> bool {
    log_trace!("node_exists");
    self.node_ids.get(node_name).is_some()
  }

  pub fn node_info_from_id(
    &mut self,
    node_id: NodeId,
  ) -> &NodeInfo {
    log_trace!("node_info_from_id: {}", node_id);

    match self.node_infos.get(node_id) {
      Some(x) => x,
      _ => {
        log_error!("(node_info_from_id) Node does not exist: {:?}", node_id);
        self.dummy_info = Default::default();
        &self.dummy_info
      },
    }
  }

  pub fn is_user_edge(
    &mut self,
    src: NodeId,
    dst: NodeId,
  ) -> bool {
    log_trace!("is_user_edge: {} {}", src, dst);
    return self.node_info_from_id(src).kind == NodeKind::User
      && self.node_info_from_id(dst).kind == NodeKind::User;
  }

  pub fn create_context_if_does_not_exist(
    &mut self,
    context: &str,
  ) {
    log_trace!("create_context_if_does_not_exist: {:?}", context);

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
          log_verbose!("Copy user edges from `` into {:?}", context);

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

  pub fn graph_from(
    &mut self,
    context: &str,
  ) -> &mut MeritRank {
    log_trace!("graph_from: {:?}", context);

    self.create_context_if_does_not_exist(context);

    match self.contexts.get_mut(context) {
      Some(x) => x,
      None => {
        log_error!("(graph_from) No context: {:?}", context);
        self.dummy_graph = MeritRank::new(Graph::new());
        &mut self.dummy_graph
      },
    }
  }

  pub fn edge_weight(
    &mut self,
    context: &str,
    src: NodeId,
    dst: NodeId,
  ) -> Weight {
    log_trace!("edge_weight: {:?} {} {}", context, src, dst);
    self
      .graph_from(context)
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
    log_trace!("edge_weight_normalized: {:?} {} {}", context, src, dst);

    let graph = self.graph_from(context);

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

  pub fn all_neighbors(
    &mut self,
    context: &str,
    node: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("all_neighbors: {:?} {}", context, node);

    let mut v = vec![];

    match self.graph_from(context).graph.get_node_data(node) {
      None => {},
      Some(data) => {
        v.reserve_exact(data.pos_edges.len() + data.neg_edges.len());

        for x in &data.pos_edges {
          v.push((*x.0, *x.1));
        }

        for x in &data.neg_edges {
          v.push((*x.0, *x.1));
        }
      },
    }

    v
  }

  pub fn all_neighbors_normalized(
    &mut self,
    context: &str,
    node: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("all_neighbors_normalized: {:?} {}", context, node);

    let mut v = vec![];

    match self.graph_from(context).graph.get_node_data(node) {
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
    log_trace!("with_zero_opinion: {:?} {} {}", context, dst_id, score);

    if context.is_empty()
      && self.node_info_from_id(dst_id).kind == NodeKind::User
    {
      let zero_score = match self.zero_opinion.get(dst_id) {
        Some(x) => *x,
        None => 0.0,
      };
      let k = 0.01 * (*ZERO_OPINION_FACTOR as f64);
      score * (1.0 - k) + k * zero_score
    } else {
      score
    }
  }

  fn with_zero_opinions(
    &mut self,
    context: &str,
    mut scores: Vec<(NodeId, Weight)>,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("with_zero_opinions: {:?}", context);

    if context.is_empty() {
      for i in 0..scores.len() {
        if self.node_info_from_id(scores[i].0).kind == NodeKind::User {
          let zero_score = match self.zero_opinion.get(scores[i].0) {
            Some(x) => *x,
            None => 0.0,
          };
          let k = 0.01 * (*ZERO_OPINION_FACTOR as f64);
          scores[i].1 = scores[i].1 * (1.0 - k) + k * zero_score
        }
      }
    }

    scores
  }

  fn fetch_all_raw_scores(
    &mut self,
    context: &str,
    ego_id: NodeId,
  ) -> Vec<(NodeId, Weight)> {
    log_trace!("fetch_all_raw_scores");

    if self.cache_walk_get(context, ego_id) {
      let graph = self.graph_from(context);
      match graph.get_ranks(ego_id, None) {
        Ok(scores) => {
          for (dst_id, score) in &scores {
            self.cache_score_add(context, ego_id, *dst_id, *score);
          }
          self.with_zero_opinions(context, scores)
        },
        Err(e) => {
          log_error!("(fetch_all_raw_scores) {}", e);
          vec![]
        },
      }
    } else {
      match self.graph_from(context).calculate(ego_id, *NUM_WALK) {
        Ok(()) => {
          self.cache_walk_add(context, ego_id);
        },
        Err(e) => {
          log_error!("(fetch_all_raw_scores) {}", e);
          return vec![];
        },
      }
      match self.graph_from(context).get_ranks(ego_id, None) {
        Ok(scores) => {
          for (dst_id, score) in &scores {
            self.cache_score_add(context, ego_id, *dst_id, *score);
          }
          self.with_zero_opinions(context, scores)
        },
        Err(e) => {
          log_error!("(fetch_all_raw_scores) {}", e);
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
    log_trace!("fetch_raw_score");

    if self.cache_walk_get(context, ego_id) {
      let graph = self.graph_from(context);
      match graph.get_node_score(ego_id, dst_id) {
        Ok(score) => {
          self.cache_score_add(context, ego_id, dst_id, score);
          self.with_zero_opinion(context, dst_id, score)
        },
        Err(e) => {
          log_error!("(fetch_raw_score) {}", e);
          0.0
        },
      }
    } else {
      match self.graph_from(context).calculate(ego_id, *NUM_WALK) {
        Ok(()) => {
          self.cache_walk_add(context, ego_id);
        },
        Err(e) => {
          log_error!("(fetch_raw_score) {}", e);
          return 0.0;
        },
      }
      match self.graph_from(context).get_node_score(ego_id, dst_id) {
        Ok(score) => {
          self.cache_score_add(context, ego_id, dst_id, score);
          self.with_zero_opinion(context, dst_id, score)
        },
        Err(e) => {
          log_error!("(fetch_raw_score) {}", e);
          0.0
        },
      }
    }
  }

  fn calculate_score_clusters_bounds(
    &mut self,
    context: &str,
    ego: NodeId,
  ) -> Vec<Weight> {
    log_trace!("calculate_score_clusters_bounds: {}", ego);

    if *NUM_SCORE_CLUSTERS <= 1 {
      return vec![];
    }

    let mut scores: Vec<Weight> = self
      .all_neighbors(context, ego)
      .into_iter()
      .map(|(dst, _)| self.fetch_raw_score(context, ego, dst))
      .collect();

    if scores.is_empty() {
      return vec![];
    }

    //  Add self score
    scores.push(self.fetch_raw_score(context, ego, ego));

    let distance = |a: f64, b: f64| (a - b).abs();

    //  K-means

    let mut rng = thread_rng();
    let mut centroids = Vec::with_capacity(*NUM_SCORE_CLUSTERS);
    let mut distances: Vec<f64> = vec![f64::MAX; scores.len()];
    centroids.push(scores[rng.gen_range(0..scores.len())]);

    for _ in 1..*NUM_SCORE_CLUSTERS {
      for (i, &point) in scores.iter().enumerate() {
        let dist = distance(point, *centroids.last().unwrap());
        distances[i] = distances[i].min(dist);
      }

      let sum: f64 = distances.iter().sum();
      let target = if sum > EPSILON {
        // LMAO RUST LIBRARIES
        rng.gen_range(0.0..sum)
      } else {
        0.0
      };
      let mut cumulative_sum = 0.0;

      for (i, &d) in distances.iter().enumerate() {
        cumulative_sum += d;
        if cumulative_sum >= target {
          centroids.push(scores[i]);
          break;
        }
      }
    }

    let mut assignments = vec![0.0; scores.len()];
    let mut changed = true;

    for _ in 0..*KMEANS_ITERATIONS {
      if !changed {
        break;
      }
      changed = false;

      for (i, &point) in scores.iter().enumerate() {
        let (closest, _) = centroids
          .iter()
          .enumerate()
          .map(|(j, &centroid)| (j as f64, distance(point, centroid)))
          .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
          .unwrap();

        if assignments[i] != closest {
          assignments[i] = closest;
          changed = true;
        }
      }

      let mut new_centroids = vec![0.0; *NUM_SCORE_CLUSTERS];
      let mut counts = vec![0; *NUM_SCORE_CLUSTERS];

      for (i, &assignment) in assignments.iter().enumerate() {
        let cluster_idx = assignment as usize;
        new_centroids[cluster_idx] += scores[i];
        counts[cluster_idx] += 1;
      }

      let mut total_shift = 0.0;
      for (i, centroid) in new_centroids.iter_mut().enumerate() {
        if counts[i] > 0 {
          let new_centroid = *centroid / counts[i] as f64;
          total_shift += (new_centroid - centroids[i]).powi(2);
          centroids[i] = new_centroid;
        } else {
          *centroid = scores[rng.gen_range(0..scores.len())];
        }
      }

      if total_shift < *KMEANS_TOLERANCE {
        break;
      }
    }

    //  Find clusters' bounds

    let mut bounds: Vec<Weight> = vec![0.0; *NUM_SCORE_CLUSTERS - 1];

    scores.sort_by(|a, b| a.total_cmp(b));
    centroids.sort_by(|a, b| a.total_cmp(b));
    assignments.sort_by(|a, b| a.total_cmp(b));

    for i in 1..centroids.len() {
      let mut left = scores[0];
      let mut right = scores[scores.len() - 1];

      for k in 1..assignments.len() {
        let middle = (assignments[k - 1] + assignments[k]) * 0.5;
        if middle >= centroids[i - 1] && middle <= centroids[i] {
          left = scores[k - 1];
          right = scores[k];
          break;
        }
      }

      bounds[i - 1] = (left + right) * 0.5;
    }

    bounds
  }

  fn clusters_from(
    &mut self,
    context: &str,
  ) -> &mut Vec<ClusterGroupBounds> {
    log_trace!("clusters_from: {:?}", context);

    if !self.cached_score_clusters.contains_key(context) {
      self
        .cached_score_clusters
        .insert(context.to_string(), vec![]);
    }

    match self.cached_score_clusters.get_mut(context) {
      Some(clusters) => clusters,

      None => {
        log_error!("(clusters_from) No context: {:?}", context);
        self.dummy_clusters = vec![];
        &mut self.dummy_clusters
      },
    }
  }

  fn update_node_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
  ) {
    log_trace!("update_node_score_clustering: {} ", ego);

    let time = self.time_begin.elapsed().as_secs() as u64;
    let bounds = self.calculate_score_clusters_bounds(context, ego);

    if ego >= self.node_count {
      log_error!(
        "(update_node_score_clustering) Node does not exist: {}",
        ego
      );
      return;
    }

    let node_count = self.node_count;

    let clusters = self.clusters_from(context);

    clusters.resize(node_count, Default::default());
    clusters[ego].updated_sec = time;
    clusters[ego].bounds = bounds;
  }

  fn update_all_nodes_score_clustering(&mut self) {
    log_trace!("update_all_nodes_score_clustering");

    for (context, _) in self.contexts.clone() {
      for node_id in 0..self.node_count {
        self.update_node_score_clustering(context.as_str(), node_id);
      }
    }
  }

  fn init_node_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
  ) {
    log_trace!("init_node_score_clustering: {:?} {}", context, ego);

    let node_count = self.node_count;

    let clusters = self.clusters_from(context);

    clusters.resize(node_count, Default::default());

    if !clusters[ego].bounds.is_empty() {
      return;
    }

    log_verbose!("Init score clustering for node {} in {:?}", ego, context);
    self.update_node_score_clustering(context, ego);
  }

  fn apply_score_clustering(
    &mut self,
    context: &str,
    ego: NodeId,
    score: Weight,
  ) -> (Weight, Cluster) {
    log_trace!("apply_score_clustering: {:?} {} {}", context, ego, score);

    if ego >= self.node_count {
      log_error!("(apply_score_clustering) Node does not exist: {}", ego);
      return (score, 0.0);
    }

    if self.node_info_from_id(ego).kind != NodeKind::User {
      //  We apply score clustering only for user nodes.
      return (score, 0.0);
    }

    self.init_node_score_clustering(context, ego);

    let elapsed_secs = self.time_begin.elapsed().as_secs();

    let clusters = self.clusters_from(context);

    if elapsed_secs >= clusters[ego].updated_sec + *SCORE_CLUSTERS_TIMEOUT {
      log_verbose!("Recalculate clustering for node {} in {:?}", ego, context);
      self.update_node_score_clustering(context, ego);
    }

    let clusters = self.clusters_from(context);

    let bounds = &clusters[ego].bounds;

    if bounds.is_empty() {
      //  Default cluster value when there's no clustering data available.
      return (score, 0.5);
    }

    let step = 1.0 / (bounds.len() as f64);

    let mut cluster = 0.0;

    for bound in bounds {
      if score < *bound + EPSILON {
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
    log_trace!("fetch_all_scores: {:?} {}", context, ego_id);

    self
      .fetch_all_raw_scores(context, ego_id)
      .iter()
      .map(|(dst_id, score)| {
        (
          *dst_id,
          *score,
          self.apply_score_clustering(context, ego_id, *score).1,
        )
      })
      .collect()
  }

  fn fetch_score(
    &mut self,
    context: &str,
    ego_id: NodeId,
    dst_id: NodeId,
  ) -> (Weight, Cluster) {
    log_trace!("fetch_score: {:?} {} {}", context, ego_id, dst_id);

    let score = self.fetch_raw_score(context, ego_id, dst_id);
    self.apply_score_clustering(context, ego_id, score)
  }

  fn fetch_score_reversed(
    &mut self,
    context: &str,
    dst_id: NodeId,
    ego_id: NodeId,
  ) -> (Weight, Cluster) {
    log_trace!("fetch_score_reversed");

    let score = match self.cache_score_get(context, ego_id, dst_id) {
      Some(score) => self.with_zero_opinion(context, dst_id, score),
      None => self.fetch_raw_score(context, ego_id, dst_id),
    };

    self.apply_score_clustering(context, ego_id, score)
  }

  fn fetch_user_score_reversed(
    &mut self,
    context: &str,
    dst_id: NodeId,
    ego_id: NodeId,
  ) -> (Weight, Cluster) {
    log_trace!("fetch_user_score_reversed");

    if self.node_info_from_id(ego_id).kind == NodeKind::User {
      return self.fetch_score_reversed(context, dst_id, ego_id);
    }

    match self.graph_from(context).graph.get_node_data(ego_id) {
      Some(x) => {
        if x.pos_edges.len() + x.neg_edges.len() == 0 {
          log_error!("(fetch_user_score_reversed) Non-user node has no owner");
          (0.0, 0.0)
        } else {
          if x.pos_edges.len() + x.neg_edges.len() != 1 {
            log_error!(
              "(fetch_user_score_reversed) Non-user node has too many edges"
            );
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
        log_error!("(fetch_user_score_reversed) Node does not exist");
        (0.0, 0.0)
      },
    }
  }

  pub fn find_or_add_node_by_name(
    &mut self,
    node_name: &str,
  ) -> NodeId {
    log_trace!("find_or_add_node_by_name: {:?}", node_name);

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
    log_trace!("set_edge: {:?} {:?} {:?} {}", context, src, dst, amount);

    if self.is_user_edge(src, dst) {
      //  Create context if does not exist

      self.graph_from("");
      if !context.is_empty() {
        self.graph_from(context);
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
      log_verbose!("Set edge in ``: {} -> {} for {}", src, dst, amount);
      self.graph_from(context).set_edge(src, dst, amount);
    } else {
      let null_weight = self.edge_weight("", src, dst);
      let old_weight = self.edge_weight(context, src, dst);
      let delta = null_weight + amount - old_weight;

      log_verbose!("Set edge in ``: {} -> {} for {}", src, dst, delta);
      self.graph_from("").set_edge(src, dst, delta);

      log_verbose!(
        "Set edge in {:?}: {} -> {} for {}",
        context,
        src,
        dst,
        amount
      );
      self.graph_from(context).set_edge(src, dst, amount);
    }
  }

  pub fn recalculate_all(
    &mut self,
    num_walk: usize,
  ) {
    log_trace!("recalculate_all: {}", num_walk);

    let infos = self.node_infos.clone();

    let graph = self.graph_from("");

    for id in 0..infos.len() {
      if (id % 100) == 90 {
        log_trace!("{}%", (id * 100) / infos.len());
      }
      if infos[id].kind == NodeKind::User {
        match graph.calculate(id, num_walk) {
          Ok(_) => {},
          Err(e) => log_error!("(recalculate_all) {}", e),
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
  log_info!("CMD read_version");
  VERSION
}

pub fn write_log_level(log_level: u32) {
  log_info!("CMD write_log_level: {}", log_level);

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
  ) -> Vec<(String, String, Weight, Weight, Weight, Weight)> {
    log_info!("CMD read_node_score: {:?} {:?} {:?}", context, ego, dst);

    if !self.contexts.contains_key(context) {
      log_error!("(read_node_score) Context does not exist: {:?}", context);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0.0, 0.0)].to_vec();
    }

    if !self.node_exists(ego) {
      log_error!("(read_node_score) Node does not exist: {:?}", ego);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0.0, 0.0)].to_vec();
    }

    if !self.node_exists(dst) {
      log_error!("(read_node_score) Node does not exist: {:?}", dst);
      return [(ego.to_string(), dst.to_string(), 0.0, 0.0, 0.0, 0.0)].to_vec();
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
    log_info!(
      "CMD read_scores: {:?} {:?} {:?} {} {} {} {} {} {} {}",
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

    let kind = match kind_str {
      "" => NodeKind::Unknown,
      "U" => NodeKind::User,
      "B" => NodeKind::Beacon,
      "C" => NodeKind::Comment,
      _ => {
        log_error!("(read_scores) Invalid node kind string: {:?}", kind_str);
        return vec![];
      },
    };

    if !self.contexts.contains_key(context) {
      log_error!("(read_scores) Context does not exist: {:?}", context);
      return vec![];
    }

    if !self.node_exists(ego) {
      log_error!("(read_scores) Node does not exist: {:?}", ego);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);

    let ranks = self.fetch_all_scores(context, ego_id);

    let mut im: Vec<(NodeId, Weight, Cluster)> = ranks
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
            && *target_kind != NodeKind::Beacon)
        {
          return true;
        }
        match self
          .graph_from(context)
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

  pub fn write_create_context(
    &mut self,
    context: &str,
  ) {
    log_info!("CMD write_create_context: {:?}", context);
    self.create_context_if_does_not_exist(context);
  }

  pub fn write_put_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    amount: f64,
    _index: i64,
  ) {
    log_info!(
      "CMD write_put_edge: {:?} {:?} {:?} {}",
      context,
      src,
      dst,
      amount
    );

    let src_id = self.find_or_add_node_by_name(src);
    let dst_id = self.find_or_add_node_by_name(dst);

    self.set_edge(context, src_id, dst_id, amount);
  }

  pub fn write_delete_edge(
    &mut self,
    context: &str,
    src: &str,
    dst: &str,
    _index: i64,
  ) {
    log_info!("CMD write_delete_edge: {:?} {:?} {:?}", context, src, dst);

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
    log_info!("CMD write_delete_node: {:?} {:?}", context, node);

    if !self.node_exists(node) {
      return;
    }

    let id = self.find_or_add_node_by_name(node);

    for (n, _) in self.all_neighbors(context, id) {
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
    log_info!(
      "CMD read_graph: {:?} {:?} {:?} {} {} {}",
      context,
      ego,
      focus,
      positive_only,
      index,
      count
    );

    if !self.contexts.contains_key(context) {
      log_error!("(read_graph) Context does not exist: {:?}", context);
      return vec![];
    }

    if !self.node_exists(ego) {
      log_error!("(read_graph) Node does not exist: {:?}", ego);
      return vec![];
    }

    if !self.node_exists(focus) {
      log_error!("(read_graph) Node does not exist: {:?}", focus);
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

    log_trace!("enumerate focus neighbors");

    let focus_neighbors = self.all_neighbors_normalized(context, focus_id);

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
          log_error!("(read_graph) Got invalid node id");
        }
      } else if dst_kind == NodeKind::Comment || dst_kind == NodeKind::Beacon {
        let dst_neighbors = self.all_neighbors_normalized(context, dst_id);

        for (ngh_id, dst_ngh_weight) in dst_neighbors {
          if (positive_only && dst_ngh_weight <= 0.0)
            || ngh_id == focus_id
            || self.node_info_from_id(ngh_id).kind != NodeKind::User
          {
            continue;
          }

          let focus_ngh_weight = focus_dst_weight
            * dst_ngh_weight
            * if focus_dst_weight < 0.0 && dst_ngh_weight < 0.0 {
              -1.0
            } else {
              1.0
            };

          if !indices.contains_key(&ngh_id) {
            let index = im_graph.add_node(ngh_id);
            indices.insert(ngh_id, index);
            ids.insert(index, ngh_id);
          }

          if let (Some(focus_idx), Some(ngh_idx)) =
            (indices.get(&focus_id), indices.get(&ngh_id))
          {
            im_graph.add_edge(*focus_idx, *ngh_idx, focus_ngh_weight);
          } else {
            log_error!("(read_graph) Got invalid node id");
          }
        }
      }
    }

    if ego_id == focus_id {
      log_trace!("ego is same as focus");
    } else {
      log_trace!("search shortest path");

      let graph_cloned = self.graph_from(context).graph.clone();

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

      log_trace!("did {} A* iterations", steps);

      if status == Status::SUCCESS {
        log_trace!("path found");
      } else if status == Status::FAIL {
        log_error!(
          "(read_graph) Path does not exist from {} to {}",
          ego_id,
          focus_id
        );
        return vec![];
      } else {
        log_error!(
          "(read_graph) Unable to find a path from {} to {}",
          ego_id,
          focus_id
        );
        return vec![];
      }

      let mut ego_to_focus: Vec<NodeId> = vec![];
      ego_to_focus.resize(astar_state.num_closed, 0);
      let n = path(&closed, &astar_state, &mut ego_to_focus);
      ego_to_focus.resize(n, 0);

      for node in ego_to_focus.iter() {
        log_trace!("path: {}", self.node_info_from_id(*node).name);
      }

      //  ================================

      let mut edges = Vec::<(NodeId, NodeId, Weight)>::new();
      edges.reserve_exact(ego_to_focus.len() - 1);

      log_trace!("process shortest path");

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
            log_trace!("ignore node {}", self.node_info_from_id(a).name);
          }
        } else if b_kind != NodeKind::User {
          log_trace!("ignore node {}", self.node_info_from_id(b).name);
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
          log_trace!("ignore node {}", self.node_info_from_id(a).name);
        }
      }

      log_trace!("add path to the graph");

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
          log_error!("(read_graph) Got invalid node id");
        }
      }
    }

    log_trace!("remove self references");

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

    log_trace!("build final array");

    for (_, src_index) in indices {
      for edge in im_graph.edges(src_index) {
        if let (Some(src_id), Some(dst_id)) =
          (ids.get(&src_index), ids.get(&edge.target()))
        {
          let w = *edge.weight();
          if w > -EPSILON && w < EPSILON {
            log_error!(
              "(read_graph) Got zero edge weight: {} -> {}",
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
          log_error!("(read_graph) Got invalid node index");
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
    log_info!("CMD read_connected: {:?} {:?}", context, ego);

    if !self.contexts.contains_key(context) {
      log_error!("(read_connected) Context does not exist: {:?}", context);
      return vec![];
    }

    if !self.node_exists(ego) {
      log_error!("(read_connected) Node does not exist: {:?}", ego);
      return vec![];
    }

    let src_id = self.find_or_add_node_by_name(ego);

    let mut v = vec![];

    for (dst_id, _) in self.all_neighbors(context, src_id) {
      v.push((ego.to_string(), self.node_info_from_id(dst_id).name.clone()));
    }

    v
  }

  pub fn read_node_list(&self) -> Vec<(String,)> {
    log_info!("CMD read_node_list");

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
    log_info!("CMD read_edges: {:?}", context);

    if !self.contexts.contains_key(context) {
      log_error!("(read_edges) Context does not exist: {:?}", context);
      return vec![];
    }

    let infos = self.node_infos.clone();

    let mut v: Vec<(String, String, Weight)> = vec![];
    v.reserve(infos.len() * 2); // ad hok

    for src_id in 0..infos.len() {
      let src_name = infos[src_id].name.as_str();

      for (dst_id, weight) in self.all_neighbors(context, src_id) {
        match infos.get(dst_id) {
          Some(x) => v.push((src_name.to_string(), x.name.clone(), weight)),
          None => log_error!("(read_edges) Node does not exist: {}", dst_id),
        }
      }
    }

    v
  }

  pub fn read_mutual_scores(
    &mut self,
    context: &str,
    ego: &str,
  ) -> Vec<(String, String, Weight, Weight, Weight, Weight)> {
    log_info!("CMD read_mutual_scores: {:?} {:?}", context, ego);

    if !self.contexts.contains_key(context) {
      log_error!("(read_mutual_scores) Context does not exist: {:?}", context);
      return vec![];
    }

    if !self.node_exists(ego) {
      log_error!("(read_mutual_scores) Node does not exist: {:?}", ego);
      return vec![];
    }

    let ego_id = self.find_or_add_node_by_name(ego);
    let ranks = self.fetch_all_scores(context, ego_id);
    let mut v = Vec::<(String, String, Weight, Weight, Weight, Weight)>::new();

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
    log_info!("CMD write_reset");
    self.reset();
  }

  pub fn read_new_edges_filter(
    &mut self,
    src: &str,
  ) -> Vec<u8> {
    log_info!("CMD read_new_edges_filter: {:?}", src);

    if !self.node_exists(src) {
      log_error!("(read_new_edges_filter) Node does not exist: {:?}", src);
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
    log_info!("CMD write_new_edges_filter: {:?} {:?}", src, filter_bytes);

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
    log_info!("CMD write_fetch_new_edges: {:?} {:?}", src, prefix);

    let num_hashes = *FILTER_NUM_HASHES;
    let max_size = *FILTER_MAX_SIZE / 8;

    let src_id = self.find_or_add_node_by_name(src);

    if self.node_infos[src_id].seen_nodes.is_empty() {
      self.node_infos[src_id]
        .seen_nodes
        .resize((*FILTER_MIN_SIZE + 7) / 8, 0);

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
}

//  ================================================
//
//    Zero opinion recalculation
//
//  ================================================

impl AugMultiGraph {
  fn reduced_graph(&mut self) -> Vec<(NodeId, NodeId, Weight)> {
    log_trace!("reduced_graph");

    let users: Vec<NodeId> = self
      .node_infos
      .iter()
      .enumerate()
      .filter(|(_id, info)| info.kind == NodeKind::User)
      .map(|(id, _info)| id)
      .collect();

    if users.is_empty() {
      return vec![];
    }

    for id in users.iter() {
      match self.graph_from("").calculate(*id, *NUM_WALK) {
        Ok(_) => {},
        Err(e) => log_error!("(reduced_graph) {}", e),
      };
    }

    let edges: Vec<(NodeId, NodeId, Weight)> = users
      .into_iter()
      .map(|id| -> Vec<(NodeId, NodeId, Weight)> {
        self
          .fetch_all_raw_scores("", id)
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

  fn top_nodes(&mut self) -> Vec<(NodeId, f64)> {
    log_trace!("top_nodes");

    let reduced = self.reduced_graph();

    if reduced.is_empty() {
      log_error!("(top_nodes) Reduced graph is empty");
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
        .take(*TOP_NODES_LIMIT)
        .into_iter()
        .unzip();

    let res = nodes.into_iter().zip(scores).collect::<Vec<_>>();

    if res.is_empty() {
      log_error!("(top_nodes) No top nodes");
    }

    return res;
  }

  pub fn write_recalculate_zero(&mut self) {
    log_info!("CMD write_recalculate_zero");

    self.recalculate_all(0);
    let nodes = self.top_nodes();
    self.recalculate_all(0);

    self.zero_opinion.resize(0, 0.0);
    self.zero_opinion.reserve(nodes.len());

    for (node_id, amount) in nodes.iter() {
      if *node_id > self.zero_opinion.len() {
        self.zero_opinion.resize(*node_id + 1, 0.0);
      }
      self.zero_opinion[*node_id] = *amount;
    }
  }

  pub fn write_recalculate_clustering(&mut self) {
    log_info!("CMD write_recalculate_clustering");

    self.update_all_nodes_score_clustering();
  }
}
