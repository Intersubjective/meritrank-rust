use crate::data::*;
use crate::utils::{log::*, quantiles::*, astar::*};
use crate::vsids::{Magnitude, VSIDSManager};
use crate::legacy_protocol::*;

use dashmap::DashMap;
use left_right::{Absorb, ReadHandleFactory, WriteHandle};
use meritrank_core::{Graph, MeritRank, NodeId, Weight, constants::EPSILON};
use moka::sync::Cache;
use tokio::{sync::mpsc, task::JoinSet};
use petgraph::{graph::{DiGraph, NodeIndex}, visit::EdgeRef};
use simple_pagerank::Pagerank;

use std::{collections::HashMap, fmt, thread, time::Duration};

#[derive(Clone)]
pub struct AugGraphSettings {
  pub num_walks: usize,
  pub zero_opinion_num_walks: usize,
  pub top_nodes_limit: usize,
  pub zero_opinion_factor: f64,
  pub score_clusters_cache_size: usize,
  pub score_clusters_timeout: u64,
  pub scores_cache_size: usize,
  pub scores_cache_timeout: u64,
  // pub walks_cache_size: usize,
  // pub filter_num_hashes: usize,
  // pub filter_max_size: usize,
  // pub filter_min_size: usize,
  pub omit_neg_edges_scores: bool,
  pub force_read_graph_conn: bool,
  pub num_score_quantiles: usize,
  // pub cache_capacity: u64,
  // pub cache_ttl: u64,
  pub legacy_connections_mode: bool,
}

impl Default for AugGraphSettings {
  fn default() -> Self {
    AugGraphSettings {
      num_walks: 10000,
      zero_opinion_num_walks: 1000,
      top_nodes_limit: 100,
      zero_opinion_factor: 0.2,
      score_clusters_cache_size: 1024 * 10,
      score_clusters_timeout: 60 * 60 * 6,
      scores_cache_size: 1024 * 10,
      scores_cache_timeout: 60 * 60,
      omit_neg_edges_scores: false,
      force_read_graph_conn: false,
      num_score_quantiles: 100,      
      legacy_connections_mode: true,
    }
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
      AugGraphOp::WriteRecalculateZero =>
        self.recalculate_zero(),
      AugGraphOp::WriteRecalculateClustering => log_warning!("Recalculate clustering is ignored!"),
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
  pub name_to_id: HashMap<NodeName, NodeId>,
  pub id_to_info: Vec<NodeInfo>,
  pub next_id:    NodeId,
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
  pub num_walks: usize,
  pub zero_opinion_num_walks: usize,
  pub legacy_connections_mode: bool,
}

impl Default for MultiGraphProcessorSettings {
  fn default() -> Self {
    MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 10,
      subgraph_queue_capacity:         1024,
      num_walks:                       10000,
      zero_opinion_num_walks:          1000,
      legacy_connections_mode:         true,
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
            AugGraph::new(AugGraphSettings {
              num_walks: self.settings.num_walks,
              zero_opinion_num_walks: self.settings.zero_opinion_num_walks,
              legacy_connections_mode: self.settings.legacy_connections_mode,
              ..AugGraphSettings::default()
            }),
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
      ReqData::WriteEdge(data) => self.process_write_edge(&req.subgraph, &data).await,
      ReqData::WriteCalculate(data) => self
        .send_op(
          &req.subgraph,
          AugGraphOp::WriteCalculate(OpWriteCalculate {
            ego: data.ego.clone(),
          }),
        )
        .await,
      ReqData::WriteCreateContext => {
        self.insert_subgraph_if_does_not_exist(&req.subgraph);
        Response::Ok
      },
      ReqData::WriteDeleteEdge(data) => self.process_write_edge(&req.subgraph, &OpWriteEdge {
        src: data.src,
        dst: data.dst,
        amount: 0.0,
        magnitude: data.index as u32,
      }).await,
      ReqData::WriteDeleteNode(_) => {
        log_warning!("Delete node request ignored!");
        Response::Ok
      },
      ReqData::WriteZeroOpinion(data) => self
        .send_op(
          &req.subgraph,
          AugGraphOp::WriteZeroOpinion(data.clone()),
        )
        .await,
      ReqData::WriteReset => {
        self.subgraphs_map.clear();
        Response::Ok
      },
      ReqData::WriteRecalculateZero => self
        .send_op(
          &req.subgraph,
          AugGraphOp::WriteRecalculateZero,
        )
        .await,
      ReqData::WriteRecalculateClustering => self
        .send_op(
          &req.subgraph,
          AugGraphOp::WriteRecalculateClustering,
        )
        .await,
      ReqData::WriteFetchNewEdges(_) => self.process_read(&req.subgraph, |_| {
        log_warning!("Fetch new edges request ignored!");
        Response::NewEdges(ResNewEdges { new_edges: vec![] })
      }),
      ReqData::WriteNewEdgesFilter(_) => self.process_read(&req.subgraph, |_| {
        log_warning!("New edges filter request ignored!");
        Response::Ok
      }),
      ReqData::ReadNewEdgesFilter(_) => self.process_read(&req.subgraph, |_| {
        log_warning!("New edges filter request ignored!");
        Response::NewEdgesFilter(ResNewEdgesFilter { bytes: vec![] })
      }),
      ReqData::ReadScores(data) => self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_scores(data),
          })
        }),
      ReqData::ReadNodeScore(data) => self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_node_score(data),
          })
        }),
      ReqData::ReadGraph(data) => self.process_read(&req.subgraph, |aug_graph| {
          Response::Graph(ResGraph {
            graph: aug_graph.read_graph(data),
          })
        }),
      ReqData::ReadNeighbors(data) =>
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_neighbors(data),
          })
        })
      ,
      ReqData::ReadNodeList => self.process_read(&req.subgraph, |aug_graph| {
          Response::NodeList(ResNodeList {
             nodes: aug_graph.nodes.id_to_info
               .iter()
               .map(|info| (info.name.clone(),))
               .collect(),
          })
        })
      ,
      ReqData::ReadEdges =>
        self.process_read(&req.subgraph, |aug_graph| {
          let mut edges = vec![];
          edges.reserve(aug_graph.nodes.id_to_info.len() * 2);

          for (src_id, info) in aug_graph.nodes.id_to_info.iter().enumerate() {
            if let Some(data) = aug_graph
              .mr
              .graph
              .get_node_data(src_id)
            {
              let src_name = &info.name;

              for (dst_id, weight) in data.get_outgoing_edges() {
                match aug_graph.nodes.get_by_id(dst_id) {
                  Some(x) => edges.push(EdgeResult { src: src_name.to_string(), dst: x.name.clone(), weight }),
                  None => log_error!("Node does not exist: {}", dst_id),
                }
              }
            };
          }

          Response::Edges(ResEdges { edges })
        })
      ,
      ReqData::ReadConnected(data) => self.process_read(&req.subgraph, |aug_graph| {
          match aug_graph.nodes.get_by_name(&data.node) {
            Some(src) =>
              Response::Connections(ResConnections {
                 connections: aug_graph
                   .mr
                  .graph
                  .get_node_data(src.id)
                  .unwrap()
                  .get_outgoing_edges()
                  .map(|(dst_id, _)| {
                    ConnectionResult {
                      src: data.node.clone(),
                      dst: aug_graph.nodes.get_by_id(dst_id).unwrap().name.clone(),
                    }
                  })
                  .collect(),
              }),
            None => {
              log_error!("Node not found: {:?}", data.node);
              Response::Fail
            },
          }
        }),
      ReqData::ReadMutualScores(data) => self.process_read(&req.subgraph, |aug_graph| {
        Response::Scores(ResScores {
          scores: aug_graph.read_mutual_scores(data),
        })
      }),
    }
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
          AugGraph::new(AugGraphSettings {
              num_walks: self.settings.num_walks,
              zero_opinion_num_walks: self.settings.zero_opinion_num_walks,
              legacy_connections_mode: self.settings.legacy_connections_mode,
              ..AugGraphSettings::default()
            }),
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
      self.apply_filters_and_pagination(scores, ego_info, &filter_options, false)
    } else {
      log_error!("Ego not found: {:?}", ego);
      vec![]
    }
  }

  pub fn read_node_score(&self, data: OpReadNodeScore) -> Vec<ScoreResult> {
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

    let (score, cluster) = self.apply_score_clustering(ego_info.id, self.fetch_raw_score(ego_info.id, dst_id), ego_info.kind);
    let (reverse_score, reverse_cluster) = self.fetch_score_cached(dst_id, ego_info.id);

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

  fn perform_astar_search(
    &self,
    graph: &Graph,
    ego_id: NodeId,
    focus_id: NodeId,
  ) -> Result<Vec<NodeId>, ()> {
    log_trace!();

    let mut open: Vec<Node<NodeId, Weight>> = vec![];
    let mut closed: Vec<Node<NodeId, Weight>> = vec![];

    open.resize(1024, Node::default());
    closed.resize(1024, Node::default());

    let mut astar_state = init(&mut open, ego_id, focus_id, 0.0);
    let mut steps = 0;
    let mut neighbor = None;
    let mut status = Status::PROGRESS;

    for _ in 0..10000 {
      // Max 10000 iterations
      steps += 1;
      status =
        iteration(&mut open, &mut closed, &mut astar_state, neighbor.clone());

      match status.clone() {
        Status::NEIGHBOR(request) => match graph.get_node_data(request.node) {
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
        },
        Status::OUT_OF_MEMORY => {
          open.resize(open.len() * 2, Node::default());
          closed.resize(closed.len() * 2, Node::default());
        },
        Status::SUCCESS | Status::FAIL => break,
        Status::PROGRESS => {},
      };
    }

    log_verbose!("Did {} A* iterations", steps);

    if status == Status::SUCCESS {
      log_verbose!("Path found");
      let mut ego_to_focus: Vec<NodeId> = vec![0; astar_state.num_closed];
      let n = path(&closed, &astar_state, &mut ego_to_focus);
      ego_to_focus.resize(n, 0);
      Ok(ego_to_focus)
    } else if status == Status::FAIL {
      log_error!("Path not found.");
      Err(())
    } else {
      log_error!("Too many iterations.");
      Err(())
    }
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

    let ego_to_focus = match self.perform_astar_search(
      &self.mr.graph,
      ego_id,
      focus_id,
    ) {
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
      self.add_edge_if_valid(im_graph, indices, ids, src, dst, weight);
    }
  }

  fn add_edge_if_valid(
    &self,
    im_graph: &mut DiGraph<NodeId, Weight>,
    indices: &mut HashMap<NodeId, NodeIndex>,
    ids: &mut HashMap<NodeIndex, NodeId>,
    src_id: NodeId,
    dst_id: NodeId,
    focus_dst_weight: Weight,
  ) {
    if let std::collections::hash_map::Entry::Vacant(e) = indices.entry(src_id) {
      let index = im_graph.add_node(src_id);
      e.insert(index);
      ids.insert(index, src_id);
    }
    if let std::collections::hash_map::Entry::Vacant(e) = indices.entry(dst_id) {
      let index = im_graph.add_node(dst_id);
      e.insert(index);
      ids.insert(index, dst_id);
    }
    if let (Some(focus_idx), Some(dst_idx)) =
      (indices.get(&src_id), indices.get(&dst_id))
    {
      im_graph.add_edge(*focus_idx, *dst_idx, focus_dst_weight);
    } else {
      log_error!("Got invalid node id");
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
        self.add_edge_if_valid(
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
          self.add_edge_if_valid(
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

  fn remove_self_references_from_im_graph(
    &self,
    im_graph: &mut DiGraph<NodeId, Weight>,
    indices: &HashMap<NodeId, NodeIndex>,
  ) {
    log_trace!();

    for (_, src_index) in indices.iter() {
      let mut edges_to_remove = Vec::new();
      for edge in im_graph.edges(*src_index) {
        if edge.target() == *src_index {
          edges_to_remove.push(edge.id());
        }
      }
      for edge_id in edges_to_remove {
        im_graph.remove_edge(edge_id);
      }
    }
  }

  fn extract_unique_edges_from_graph_data(
    &self,
    indices: &HashMap<NodeId, NodeIndex>,
    ids: &HashMap<NodeIndex, NodeId>,
    im_graph: &DiGraph<NodeId, Weight>,
  ) -> Vec<(NodeId, NodeId, Weight)> {
    log_trace!();

    let mut edge_ids = Vec::<(NodeId, NodeId, Weight)>::new();
    // Pre-allocate with a reasonable guess, though actual number of unique edges can vary.
    edge_ids.reserve_exact(indices.len() * 2);

    for src_index in indices.values() {
      for edge in im_graph.edges(*src_index) {
        if let (Some(&src_id), Some(&dst_id)) = // Dereference here
          (ids.get(src_index), ids.get(&edge.target()))
        {
          let w = *edge.weight();
          if w > -EPSILON && w < EPSILON {
            // Check for zero weight
            log_error!(
              "Got zero edge weight: {} -> {}",
              src_id,
              dst_id
            );
          } else {
            // Check for duplicate edges before pushing
            let mut found = false;
            for (x, y, _) in edge_ids.iter() {
              if src_id == *x && dst_id == *y {
                found = true;
                break;
              }
            }
            if !found {
              edge_ids.push((src_id, dst_id, w));
            }
          }
        } else {
          log_error!("Got invalid node index during edge extraction");
        }
      }
    }
    edge_ids
  }

  fn fetch_score(&self, ego: NodeId, dst: NodeId) -> (NodeScore, NodeCluster) {
    self.apply_score_clustering(
      ego,
      self.fetch_raw_score(ego, dst),
      self.nodes.id_to_info[ego].kind
    )
  }

  fn get_object_owner(&self, node: NodeId) -> Option<NodeId> {
    Some(node) // FIXME: Somehow this function was broken...
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
            Some(dst_owner_id) => {
              self.fetch_score_cached(dst_owner_id, ego_id)
            },
            None => (0.0, 0),
          };

        GraphResult {
          src: self.nodes.id_to_info[src_id].name.clone(),
          dst: self.nodes.id_to_info[dst_id].name.clone(),
          weight: weight_of_dst,
          score: score_value_of_dst,
          reverse_score: score_value_of_ego,
          cluster: score_cluster_of_dst,
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
      self.extract_unique_edges_from_graph_data(indices, ids, im_graph);
    self.sort_paginate_and_format_graph_edges(
      unique_edges,
      ego_id,
      index,
      count,
    )
  }

  pub fn read_graph(&self, data: OpReadGraph) -> Vec<GraphResult> {
    log_command!("{:?}", data);

    let ego_str = &data.ego;
    let focus_str = &data.focus;
    let positive_only = data.positive_only;
    let index = data.index;
    let count = data.count;

    let (ego_id, focus_id, mut im_graph, mut indices, mut ids) = match self
      .validate_read_graph_params_and_setup(ego_str, focus_str)
    {
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
        &node_infos, ego_id, focus_id, &mut indices, &mut ids, &mut im_graph,
      );
    }

    if force_read_graph_conn && !indices.contains_key(&ego_id) {
      self.add_edge_if_valid(&mut im_graph, &mut indices, &mut ids, ego_id, focus_id, 1.0);
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

    self.remove_self_references_from_im_graph(&mut im_graph, &indices);

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
          }
      };

      let edges: Vec<_> = match dir {
          NEIGHBORS_OUTBOUND => node_data.pos_edges.iter().collect(),
          NEIGHBORS_INBOUND => node_data.neg_edges.iter().collect(),
          NEIGHBORS_ALL => node_data.pos_edges.iter().chain(node_data.neg_edges.iter()).collect(),
          _ => {
            log_error!("Invalid direction: {}", dir);
            return vec![];
          },
      };

      edges.into_iter()
          .map(|(dst_id, &weight)| {
              let (_score, cluster) = self.fetch_score_cached(ego_id, *dst_id);
              (self.nodes.get_by_id(*dst_id).unwrap().clone(), weight, cluster)
          })
          .collect()
  }

  pub fn read_neighbors(&self, data: OpReadNeighbors) -> Vec<ScoreResult> {
    log_command!("{:?}", data);

    let kind_opt = data.kind;

    let dir = data.direction;

    if dir != NEIGHBORS_INBOUND && dir != NEIGHBORS_OUTBOUND && dir != NEIGHBORS_ALL {
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
        node_kind: None,
        hide_personal: data.hide_personal,
        score_lt: data.lt,
        score_lte: data.lte,
        score_gt: data.gt,
        score_gte: data.gte,
        index: data.index,
        count: data.count,
      },
      true,
    )
  }

  pub fn read_mutual_scores(&self, data: OpReadMutualScores) -> Vec<ScoreResult> {
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
    let mut v =
      Vec::<ScoreResult>::new();
    v.reserve_exact(ranks.len());

    for (node, score_value_of_dst, score_cluster_of_dst) in ranks {
      if score_value_of_dst > 0.0 && node.kind == NodeKind::User {
        let (score_value_of_ego, score_cluster_of_ego) =
          match self.get_object_owner(node.id) {
            Some(dst_owner_id) => {
              self.fetch_score_cached(dst_owner_id, ego_id)
            },
            None => (0.0, 0),
          };
        v.push(ScoreResult {
          ego: data.ego.clone(),
          target: node.name,
          score: score_value_of_dst,
          reverse_score: score_value_of_ego,
          cluster: score_cluster_of_dst,
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
      self.filter_and_sort_scores(scores, ego_info, filter_options);

    if prioritize_ego_owned_nodes {
      self.prioritize_ego_owned_items(&mut filtered_sorted_scores, ego_info);
    }

    self.paginate_and_format_items(
      filtered_sorted_scores,
      ego_info,
      filter_options.index,
      filter_options.count,
    )
  }

  fn filter_and_sort_scores(
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

  fn prioritize_ego_owned_items(
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
          && (*dst_kind_opt == NodeKind::User || *dst_kind_opt == NodeKind::Beacon)
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
      _ => {
        if self.settings.legacy_connections_mode {
          let src_id = self.nodes.register(&mut self.mr, src, NodeKind::User);
          let dst_id = self.nodes.register(&mut self.mr, dst, NodeKind::User);
          Ok((src_id, dst_id))
        } else {
          Err(AugGraphError::IncorrectNodeKinds(src, dst))
        }
      },
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
