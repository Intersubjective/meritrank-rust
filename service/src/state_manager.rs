use crate::data::*;
use crate::utils::{log::*, quantiles::*};
use crate::vsids::{Magnitude, VSIDSManager};

use dashmap::DashMap;
use left_right::{Absorb, ReadHandleFactory, WriteHandle};
use meritrank_core::{Graph, MeritRank, NodeId, Weight};
use moka::sync::Cache;
use serde::Deserialize;
use tokio::{sync::mpsc, task::JoinSet};

use std::{collections::HashMap, fmt, thread, time::Duration};

fn _num_walks() -> usize {
  10000
}
fn _zero_opinion_num_walks() -> usize {
  1000
}
fn _top_nodes_limit() -> usize {
  100
}
fn _zero_opinion_factor() -> f64 {
  0.20
}
fn _score_clusters_cache_size() -> usize {
  1024 * 10
}
fn _score_clusters_timeout() -> u64 {
  21600
} // 60 * 60 * 6 (6 hours)
fn _scores_cache_size() -> usize {
  1024 * 10
} // 1024 * 10
fn _scores_cache_timeout() -> u64 {
  3600
} // 60 * 60 (1 hour)
fn _walks_cache_size() -> usize {
  1024
}
fn _filter_num_hashes() -> usize {
  10
}
fn _filter_max_size() -> usize {
  8192
}
fn _filter_min_size() -> usize {
  32
}
fn _omit_neg_edges_scores() -> bool {
  false
}
fn _force_read_graph_conn() -> bool {
  false
}
fn _num_score_quantiles() -> usize {
  100
}
fn _cache_capacity() -> u64 {
  1_000_000
}
fn _cache_ttl() -> u64 {
  3600
}

#[derive(Clone, Deserialize)]
pub struct AugGraphSettings {
  #[serde(default = "_num_walks")]
  pub num_walks: usize,

  // #[serde(default = "_zero_opinion_num_walks")]
  // pub zero_opinion_num_walks: usize,

  // #[serde(default = "_top_nodes_limit")]
  // pub top_nodes_limit: usize,
  #[serde(default = "_zero_opinion_factor")]
  pub zero_opinion_factor: f64,

  #[serde(default = "_score_clusters_cache_size")]
  pub score_clusters_cache_size: usize,

  #[serde(default = "_score_clusters_timeout")]
  pub score_clusters_timeout: u64,

  #[serde(default = "_scores_cache_size")]
  pub scores_cache_size: usize,

  #[serde(default = "_scores_cache_timeout")]
  pub scores_cache_timeout: u64,

  // #[serde(default = "_walks_cache_size")]
  // pub walks_cache_size: usize,

  // #[serde(default = "_filter_num_hashes")]
  // pub filter_num_hashes: usize,

  // #[serde(default = "_filter_max_size")]
  // pub filter_max_size: usize,

  // #[serde(default = "_filter_min_size")]
  // pub filter_min_size: usize,
  #[serde(default = "_omit_neg_edges_scores")]
  pub omit_neg_edges_scores: bool,

  // #[serde(default = "_force_read_graph_conn")]
  // pub force_read_graph_conn: bool,
  #[serde(default = "_num_score_quantiles")]
  pub num_score_quantiles: usize,
  // #[serde(default = "_cache_capacity")]
  // pub cache_capacity: u64,

  // #[serde(default = "_cache_ttl")]
  // pub cache_ttl: u64,
}

impl AugGraphSettings {
  pub fn from_env() -> Result<Self, envy::Error> {
    envy::from_env::<AugGraphSettings>()
  }
}

impl Default for AugGraphSettings {
  fn default() -> Self {
    // FIXME: Panic is not necessary here!
    envy::from_iter::<_, AugGraphSettings>(
      std::iter::empty::<(String, String)>(),
    )
    .expect("Failed to create default settings")
  }
}

#[derive(Clone)]
pub struct AugGraph {
  pub mr:                    MeritRank,
  pub nodes:                 NodeRegistry,
  pub settings:              AugGraphSettings,
  pub zero_opinion:          Vec<NodeScore>, // FIXME: change to map because of sparseness
  pub cached_scores:         Cache<(NodeId, NodeId), NodeScore>,
  pub cached_score_clusters: Cache<(NodeId, NodeKind), ClusterGroupBounds>,
  // pub poll_store:            PollStore,
  pub vsids:                 VSIDSManager,
}

impl AugGraph {
  pub fn new(settings: AugGraphSettings) -> AugGraph {
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
    }
  }
}

impl Absorb<AugGraphOp> for AugGraph {
  fn absorb_first(
    &mut self,
    op: &mut AugGraphOp,
    _: &Self,
  ) {
    log_trace!();

    //  FIXME: Pass strings by reference, no clones!

    match op {
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
      _ => {
        log_error!("Not implemented.");
      },
    }
  }

  fn sync_with(
    &mut self,
    first: &Self,
  ) {
    *self = first.clone()
  }
}

pub type GraphProcessor = ConcurrentDataProcessor<AugGraph, AugGraphOp>;

pub type ClusterGroupBounds = Vec<NodeScore>;

impl AugGraph {
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
  pub id:    NodeId,
  pub name:  NodeName,
  pub kind:  NodeKind,
  pub owner: Option<NodeId>,
}

#[derive(Clone)]
pub struct NodeRegistry {
  name_to_id: HashMap<NodeName, NodeId>,
  id_to_info: Vec<NodeInfo>,
  next_id:    NodeId,
}

impl NodeRegistry {
  pub fn new() -> Self {
    NodeRegistry {
      name_to_id: HashMap::new(),
      id_to_info: Vec::new(),
      next_id:    0,
    }
  }

  pub fn register(
    &mut self,
    mr: &mut MeritRank,
    name: NodeName,
    kind: NodeKind,
  ) -> NodeId {
    if let Some(&id) = self.name_to_id.get(&name) {
      return id;
    }

    let id = self.next_id;
    self.next_id += 1;

    if id != mr.get_new_nodeid() {
      log_error!("Got unexpected node id.");
    }

    let info = NodeInfo {
      id,
      name: name.clone(),
      kind,
      owner: None,
    };
    self.name_to_id.insert(name, id);
    self.id_to_info.push(info);

    id
  }

  pub fn register_with_owner(
    &mut self,
    mr: &mut MeritRank,
    name: NodeName,
    kind: NodeKind,
    owner: NodeId,
  ) -> NodeId {
    if let Some(&id) = self.name_to_id.get(&name) {
      return id;
    }

    let id = self.next_id;
    self.next_id += 1;

    if id != mr.get_new_nodeid() {
      log_error!("Got unexpected node id.");
    }

    let info = NodeInfo {
      id,
      name: name.clone(),
      kind,
      owner: Some(owner),
    };
    self.name_to_id.insert(name, id);
    self.id_to_info.push(info);

    id
  }

  pub fn get_by_id(
    &self,
    id: NodeId,
  ) -> Option<&NodeInfo> {
    self.id_to_info.get(id)
  }

  pub fn get_by_name(
    &self,
    name: &str,
  ) -> Option<&NodeInfo> {
    self
      .name_to_id
      .get(name)
      .and_then(|&id| self.id_to_info.get(id))
  }

  //  FIXME: Used in testing.
  #[allow(unused)]
  pub fn update_owner(
    &mut self,
    id: NodeId,
    new_owner: Option<NodeId>,
  ) -> bool {
    if let Some(info) = self.id_to_info.get_mut(id) {
      info.owner = new_owner;
      true
    } else {
      false
    }
  }

  //  FIXME: Used in testing.
  #[allow(unused)]
  pub fn len(&self) -> usize {
    self.id_to_info.len()
  }

  //  FIXME: Used in testing.
  #[allow(unused)]
  pub fn is_empty(&self) -> bool {
    self.id_to_info.is_empty()
  }

  pub fn nodes_by_kind(
    &self,
    kind: NodeKind,
  ) -> Vec<NodeId> {
    self
      .id_to_info
      .iter()
      .enumerate()
      .filter(|(_, info)| info.kind == kind)
      .map(|(id, _)| id)
      .collect()
  }
}

impl fmt::Display for NodeKind {
  fn fmt(
    &self,
    f: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    match self {
      NodeKind::User => write!(f, "User"),
      NodeKind::Beacon => write!(f, "Beacon"),
      NodeKind::Comment => write!(f, "Comment"),
      NodeKind::Opinion => write!(f, "Opinion"),
      NodeKind::PollVariant => write!(f, "PollVariant"),
      NodeKind::Poll => write!(f, "Poll"),
    }
  }
}

pub fn node_kind_from_prefix(name: &str) -> Option<NodeKind> {
  if name.is_empty() {
    return None;
  }
  match name.chars().next() {
    Some('U') => Some(NodeKind::User),
    Some('B') => Some(NodeKind::Beacon),
    Some('C') => Some(NodeKind::Comment),
    Some('O') => Some(NodeKind::Opinion),
    Some('V') => Some(NodeKind::PollVariant),
    Some('P') => Some(NodeKind::Poll),
    _ => None,
  }
}

pub struct ConcurrentDataProcessor<T, Op> {
  #[allow(unused)]
  processing_thread:       thread::JoinHandle<()>,
  pub op_sender:           mpsc::Sender<Op>,
  pub data_reader_factory: ReadHandleFactory<T>,
  // _phantom:                PhantomData<T>, // This is needed because T is not used directly in the struct
}

impl<T, Op> ConcurrentDataProcessor<T, Op>
where
  T: 'static + Send + Sync + Clone + Absorb<Op>,
  Op: 'static + Send,
{
  pub fn new(
    t: T,
    sleep: u64,
    queue_len: usize,
  ) -> Self {
    let (writer, reader) = left_right::new_from_empty::<T, Op>(t);
    let (tx, rx) = mpsc::channel::<Op>(queue_len);
    let loop_thread = thread::spawn(move || processing_loop(writer, rx, sleep));
    ConcurrentDataProcessor {
      processing_thread:   loop_thread,
      op_sender:           tx,
      data_reader_factory: reader.factory(),
      // _phantom:            PhantomData,
    }
  }

  //  FIXME: Used in testing.
  #[allow(unused)]
  pub fn shutdown(self) -> thread::Result<()> {
    // Drop the sender, which will close the channel
    drop(self.op_sender);
    // Join the thread
    self.processing_thread.join()
  }
}

fn processing_loop<T, Op>(
  mut writer: WriteHandle<T, Op>,
  mut rx_ops_queue: mpsc::Receiver<Op>,
  sleep: u64,
) where
  T: 'static + Send + Sync + Absorb<Op>,
  Op: 'static + Send,
{
  while let Some(op) = rx_ops_queue.blocking_recv() {
    writer.append(op);
    //println!("Ops: {}", rx_ops_queue.len());
    // Note that left-right is not really eventually-consistent,
    // but instead strong-consistent. This means that in case of
    // high load on reading, publish() will block readers until all
    // the _reading_ operations are finished, and then all the operations
    // are applied in the correct order.
    // There are two ways to handle this:
    // 1. sleep a bit on the write execution thread to allow the readers to flush
    // 2. implement a truly eventually-consistent version of left-right that never blocks (arc-swap)
    thread::sleep(std::time::Duration::from_millis(sleep));
    writer.publish();
  }
}

pub struct MultiGraphProcessorSettings {
  pub sleep_duration_after_publish_ms: u64,
  pub subgraph_queue_capacity:         usize,
}

impl Default for MultiGraphProcessorSettings {
  fn default() -> Self {
    MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 100,
      subgraph_queue_capacity:         1024,
    }
  }
}

pub struct MultiGraphProcessor {
  subgraphs_map: DashMap<SubgraphName, GraphProcessor>,
  settings:      MultiGraphProcessorSettings,
}

impl MultiGraphProcessor {
  pub fn new(settings: MultiGraphProcessorSettings) -> Self {
    MultiGraphProcessor {
      subgraphs_map: DashMap::new(),
      settings,
    }
  }

  fn get_tx_channel(
    &self,
    subgraph_name: &SubgraphName,
  ) -> mpsc::Sender<AugGraphOp> {
    log_trace!();

    match self.subgraphs_map.get(subgraph_name) {
      Some(subgraph) => subgraph.op_sender.clone(),
      None => self
        .subgraphs_map
        .entry(subgraph_name.clone())
        .or_insert((|| {
          log_trace!("Create subgraph");
          GraphProcessor::new(
            AugGraph::new(AugGraphSettings::from_env().unwrap_or_default()),
            self.settings.sleep_duration_after_publish_ms,
            self.settings.subgraph_queue_capacity,
          )
        })())
        .op_sender
        .clone(),
    }
  }

  async fn send_op(
    &self,
    subgraph_name: &SubgraphName,
    op: AugGraphOp,
  ) -> Response {
    log_trace!();

    if self.get_tx_channel(subgraph_name).send(op).await.is_ok() {
      Response::Ok
    } else {
      Response::Fail
    }
  }

  fn process_read<F>(
    &self,
    subgraph_name: &SubgraphName,
    read_function: F,
  ) -> Response
  where
    F: FnOnce(&AugGraph) -> Response,
  {
    let subgraph = match self.subgraphs_map.get(subgraph_name) {
      Some(subgraph) => {
        log_verbose!("Found subgraph for name: {:?}", subgraph_name);
        subgraph
      },
      None => {
        log_warning!("Subgraph not found for name: {:?}", subgraph_name);
        return Response::Fail;
      },
    };

    let reader_handle = subgraph.data_reader_factory.handle();
    log_trace!("Obtained reader handle for subgraph: {:?}", subgraph_name);

    let guard = match reader_handle.enter() {
      Some(guard) => {
        log_trace!(
          "Successfully entered reader handle for subgraph: {:?}",
          subgraph_name
        );
        guard
      },
      None => {
        log_warning!("Failed to enter reader handle for subgraph: {:?}. WriteHandle might have been dropped.", subgraph_name);
        return Response::Fail;
      },
    };

    let aug_graph: &AugGraph = &*guard;
    log_trace!(
      "Successfully accessed AugGraph for subgraph: {:?}",
      subgraph_name
    );

    let response = read_function(aug_graph);
    log_verbose!("Executed read function for subgraph: {:?}", subgraph_name);

    response
  }

  pub async fn process_request(
    &self,
    req: &Request,
  ) -> Response {
    //  FIXME: No need to clone here, but borrow checker!!!
    let data = req.data.clone();

    match data {
      ReqData::WriteEdge(data) => {
        self.process_write_edge(&req.subgraph, &data).await
      },
      ReqData::WriteCalculate(data) => {
        self.process_write_calculate(&req.subgraph, &data).await
      },
      ReqData::ReadScores(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_scores(&data.ego, &data.score_options),
          })
        })
      },
      _ => {
        log_error!("Request not implemented: {:?}", data);
        Response::Fail
      },
    }
  }

  async fn process_write_calculate(
    &self,
    subgraph_name: &SubgraphName,
    data: &OpWriteCalculate,
  ) -> Response {
    log_trace!("{:?} {:?}", subgraph_name, data);

    self
      .send_op(
        subgraph_name,
        AugGraphOp::WriteCalculate(OpWriteCalculate {
          ego: data.ego.clone(),
        }),
      )
      .await
  }

  async fn process_write_edge(
    &self,
    subgraph_name: &SubgraphName,
    data: &OpWriteEdge,
  ) -> Response {
    log_trace!("{:?} {:?}", subgraph_name, data);

    if data.src == data.dst {
      log_error!("Self-reference is not allowed.");
      return Response::Fail;
    }

    let src_kind_opt = node_kind_from_prefix(&data.src);
    let dst_kind_opt = node_kind_from_prefix(&data.dst);

    match (src_kind_opt, dst_kind_opt) {
      (Some(NodeKind::User), Some(NodeKind::User)) => {
        self
          .process_user_to_user_edge(
            subgraph_name,
            &data.src,
            &data.dst,
            data.amount,
            data.magnitude,
          )
          .await
      },

      (Some(NodeKind::User), Some(NodeKind::PollVariant)) => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::SetUserVote(OpSetUserVote {
              user_id:    data.src.clone(),
              variant_id: data.dst.clone(),
              amount:     data.amount,
            }),
          )
          .await
      },
      (Some(NodeKind::PollVariant), Some(NodeKind::Poll)) => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::AddPollVariant(OpAddPollVariant {
              poll_id:    data.dst.clone(),
              variant_id: data.src.clone(),
            }),
          )
          .await
      },
      (Some(src_kind), Some(dst_kind))
        if src_kind == NodeKind::PollVariant
          || src_kind == NodeKind::Poll
          || dst_kind == NodeKind::PollVariant
          || dst_kind == NodeKind::Poll =>
      {
        log_error!("Unexpected edge type: {:?} -> {:?} in context {:?}. No action taken.", src_kind_opt, dst_kind_opt, subgraph_name);
        Response::Fail
      },
      _ => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::WriteEdge(OpWriteEdge {
              src:       data.src.clone(),
              dst:       data.dst.clone(),
              amount:    data.amount,
              magnitude: data.magnitude,
            }),
          )
          .await
      },
    }
  }

  fn insert_subgraph_if_does_not_exist(
    &self,
    subgraph_name: &SubgraphName,
  ) {
    log_trace!();

    //  FIXME: Cleanup! Code duplication here, but generic types are tricky.
    self
      .subgraphs_map
      .entry(subgraph_name.clone())
      .or_insert((|| {
        log_trace!("Create subgraph");
        GraphProcessor::new(
          AugGraph::new(AugGraphSettings::from_env().unwrap_or_default()),
          self.settings.sleep_duration_after_publish_ms,
          self.settings.subgraph_queue_capacity,
        )
      })());
  }

  async fn process_user_to_user_edge(
    &self,
    subgraph_name: &SubgraphName,
    src: &NodeName,
    dst: &NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) -> Response {
    log_trace!();

    self.insert_subgraph_if_does_not_exist(subgraph_name);

    let mut join_set = JoinSet::new();

    for ref_multi in self.subgraphs_map.iter() {
      let subgraph = ref_multi.value();
      let op_sender = subgraph.op_sender.clone();
      let src = src.clone();
      let dst = dst.clone();

      join_set.spawn(async move {
        op_sender
          .send(AugGraphOp::WriteEdge(OpWriteEdge {
            src,
            dst,
            amount,
            magnitude,
          }))
          .await
      });
    }

    let mut all_successful = true;
    while let Some(result) = join_set.join_next().await {
      match result {
        Ok(Ok(())) => {},
        _ => {
          log_error!("Failed to send WriteEdge operation to a subgraph");
          all_successful = false;
        },
      }
    }

    if all_successful {
      Response::Ok
    } else {
      Response::Fail
    }
  }
}

// #[derive(Debug, Clone, Copy, PartialEq)]
// pub enum NeighborDirection {
//   All,
//   Outbound,
//   Inbound,
// }

impl AugGraph {
  pub fn read_scores(
    &self,
    ego: &str,
    filter_options: &FilterOptions,
  ) -> Vec<ScoreResult> {
    log_command!("{:?} {:?}", ego, filter_options);

    if let Some(ego_info) = self.nodes.get_by_name(ego) {
      if ego_info.kind != NodeKind::User {
        log_warning!("Trying to use non-user as ego {}", ego);
        return vec![];
      }
      let scores = self.fetch_all_scores(ego_info);
      self.apply_filters_and_pagination(scores, ego_info, filter_options, false)
    } else {
      log_warning!("Ego not found: {:?}", ego);
      vec![]
    }
  }

  fn apply_filters_and_pagination(
    &self,
    scores: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    filter_options: &FilterOptions,
    prioritize_ego_owned_nodes: bool,
  ) -> Vec<ScoreResult> {
    let mut filtered_sorted_scores =
      self._filter_and_sort_scores(scores, ego_info, filter_options);

    if prioritize_ego_owned_nodes {
      self._prioritize_ego_owned_items(&mut filtered_sorted_scores, ego_info);
    }

    self._paginate_and_format_items(
      filtered_sorted_scores,
      ego_info,
      filter_options.index,
      filter_options.count,
    )
  }

  fn _filter_and_sort_scores(
    &self,
    scores: Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
    filter_options: &FilterOptions,
  ) -> Vec<(NodeInfo, NodeScore, NodeCluster)> {
    let mut filtered_scores: Vec<(NodeInfo, NodeScore, NodeCluster)> = scores
      .into_iter()
      .filter(|(node_info, score, _)| {
        // Apply kind filter
        filter_options
          .node_kind
          .map_or(true, |filter_kind| node_info.kind == filter_kind)
          && !(filter_options.hide_personal
            && node_info.owner == Some(ego_info.id))
          && {
            // Apply score filters
            (*score > filter_options.score_gt
              || (!filter_options.score_gte
                && *score >= filter_options.score_gt))
              && (*score < filter_options.score_lt
                || (!filter_options.score_lte
                  && *score <= filter_options.score_lt))
          }
      })
      .collect();

    filtered_scores.sort_by(|(_, a, _), (_, b, _)| b.abs().total_cmp(&a.abs()));
    filtered_scores
  }

  fn _prioritize_ego_owned_items(
    &self,
    items: &mut Vec<(NodeInfo, NodeScore, NodeCluster)>,
    ego_info: &NodeInfo,
  ) {
    let mut insert_index = 0;
    for i in 0..items.len() {
      if let Some(owner) = items[i].0.owner {
        if owner == ego_info.id {
          items.swap(i, insert_index);
          insert_index += 1;
        }
      }
    }
  }

  fn _paginate_and_format_items(
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

  // pub fn fetch_neighbors(
  //   &self,
  //   ego_id: NodeId,
  //   focus_id: NodeId,
  //   dir: NeighborDirection,
  // ) -> Vec<(NodeId, Weight, NodeCluster)> {
  //   log_trace!("{} {} {:?}", ego_id, focus_id, dir);

  //   let node_data = match self.mr.graph.get_node_data(focus_id) {
  //     Some(data) => data,
  //     None => {
  //       log_warning!("Node not found: {}", focus_id);
  //       return vec![];
  //     },
  //   };

  //   let edges: Vec<_> = match dir {
  //     NeighborDirection::Outbound => node_data.pos_edges.iter().collect(),
  //     NeighborDirection::Inbound => node_data.neg_edges.iter().collect(),
  //     NeighborDirection::All => node_data
  //       .pos_edges
  //       .iter()
  //       .chain(node_data.neg_edges.iter())
  //       .collect(),
  //   };

  //   edges
  //     .into_iter()
  //     .map(|(dst_id, &weight)| {
  //       let (_score, cluster) = self.fetch_score_cached(ego_id, *dst_id);
  //       (*dst_id, weight, cluster)
  //     })
  //     .collect()
  // }
}

#[derive(Debug)]
enum AugGraphError {
  SelfReference,
  IncorrectNodeKinds(NodeName, NodeName),
}

impl AugGraph {
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
    new_min_weight = self._apply_edge_rescales_and_deletions(
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

  fn _apply_edge_rescales_and_deletions(
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

  fn reg_owner_and_get_ids(
    &mut self,
    src: NodeName,
    dst: NodeName,
  ) -> Result<(NodeId, NodeId), AugGraphError> {
    if src == dst {
      return Err(AugGraphError::SelfReference);
    }

    let (src, dst) = (src.clone(), dst.clone());
    match (node_kind_from_prefix(&src), node_kind_from_prefix(&dst)) {
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
      _ => Err(AugGraphError::IncorrectNodeKinds(src, dst)),
    }
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

    // Test update_owner
    assert!(registry.update_owner(1, Some(0)));
    let updated_info = registry.get_by_id(1).unwrap();
    assert_eq!(updated_info.owner, Some(0));

    // Test update_owner for non-existent id
    assert!(!registry.update_owner(2, Some(0)));

    assert_eq!(registry.len(), 2);
    assert!(!registry.is_empty());

    // Test non-existent entries
    assert_eq!(registry.get_by_id(2), None);
    assert_eq!(registry.get_by_name("Bob"), None);
  }

  #[test]
  fn nonblocking() {
    use left_right::Absorb;

    // Define a simple wrapper for i32 that implements Absorb
    #[derive(Clone)]
    pub struct MyDataType(i32);

    struct TestOp(i32);

    impl Absorb<TestOp> for MyDataType {
      fn absorb_first(
        &mut self,
        operation: &mut TestOp,
        _: &Self,
      ) {
        self.0 += operation.0;
      }
      fn absorb_second(
        &mut self,
        operation: TestOp,
        _: &Self,
      ) {
        self.0 += operation.0;
      }
      fn sync_with(
        &mut self,
        first: &Self,
      ) {
        *self = first.clone();
      }
    }

    let processor =
      ConcurrentDataProcessor::<MyDataType, TestOp>::new(MyDataType(0), 0, 10);
    processor.op_sender.blocking_send(TestOp(1)).unwrap();
    processor.op_sender.blocking_send(TestOp(1)).unwrap();
    processor.op_sender.blocking_send(TestOp(1)).unwrap();
    thread::sleep(std::time::Duration::from_millis(10));
    let handle = processor.data_reader_factory.handle();
    assert_eq!(handle.enter().unwrap().0, 3);
    processor
      .shutdown()
      .expect("Failed to shutdown processing loop");
  }
}
