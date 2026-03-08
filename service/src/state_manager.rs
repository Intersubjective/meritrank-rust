use crate::aug_graph::*;
use crate::data::*;
use crate::node_registry::*;
use crate::settings::*;
use crate::utils::log::*;
use crate::vsids::Magnitude;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use parking_lot::RwLock;
use crate::data::Weight;
use tokio::{sync::mpsc, task::JoinSet};

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crate::processor_stats::ProcessorStats;
use crate::walk_tracker::WalkTracker;
use meritrank_core::NodeId;

/// Sends each op to both write channels (fan-out) for double-buffered eventual consistency.
#[derive(Clone)]
pub struct FanoutSender {
  tx_a: mpsc::Sender<AugGraphOp>,
  tx_b: mpsc::Sender<AugGraphOp>,
}

impl FanoutSender {
  pub async fn send(
    &self,
    op: AugGraphOp,
  ) -> Result<(), mpsc::error::SendError<AugGraphOp>> {
    let op2 = op.clone();
    self.tx_a.send(op).await?;
    self.tx_b.send(op2).await?;
    Ok(())
  }
}

pub struct ConcurrentDataProcessor {
  #[allow(unused)]
  processing_thread: thread::JoinHandle<()>,
  pub op_sender:     FanoutSender,
  pub shared:        Arc<ArcSwap<RwLock<AugGraph>>>,
  pub walk_tracker:  Option<WalkTracker>,
}

pub type GraphProcessor = ConcurrentDataProcessor;

pub struct MultiGraphProcessor {
  pub subgraphs_map: DashMap<SubgraphName, GraphProcessor>,
  settings:          Settings,
  loading:           AtomicBool,
  internal_stamp:    AtomicU64,
  publish_notify:    Arc<tokio::sync::Notify>,
  pub stats:         Option<Arc<ProcessorStats>>,
}

fn processing_loop(
  copy_a: Arc<RwLock<AugGraph>>,
  copy_b: Arc<RwLock<AugGraph>>,
  write_rx_a: mpsc::Receiver<AugGraphOp>,
  write_rx_b: mpsc::Receiver<AugGraphOp>,
  shared: Arc<ArcSwap<RwLock<AugGraph>>>,
  publish_notify: Arc<tokio::sync::Notify>,
  min_ops_before_swap: usize,
  stats: Option<Arc<ProcessorStats>>,
) {
  let mut front_arc = copy_a;
  let mut back_arc = copy_b;
  let mut front_rx = write_rx_a;
  let mut back_rx = write_rx_b;

  shared.store(Arc::clone(&front_arc));
  let mut back_guard = back_arc.write();

  let apply_one = |guard: &mut parking_lot::RwLockWriteGuard<'_, AugGraph>, op: &AugGraphOp, st: &Option<Arc<ProcessorStats>>, record_stats: bool| {
    let start = Instant::now();
    guard.apply_op(op);
    if record_stats {
      if let Some(s) = st {
        s.record_applied(start.elapsed());
      }
    }
  };

  loop {
    let mut applied = 0usize;
    while applied < min_ops_before_swap {
      let op = match back_rx.blocking_recv() {
        Some(o) => o,
        None => return,
      };
      apply_one(&mut back_guard, &op, &stats, true);
      applied += 1;
      while let Ok(op) = back_rx.try_recv() {
        apply_one(&mut back_guard, &op, &stats, true);
        applied += 1;
      }
    }

    drop(back_guard);
    shared.store(Arc::clone(&back_arc));
    publish_notify.notify_waiters();

    std::mem::swap(&mut front_arc, &mut back_arc);
    std::mem::swap(&mut front_rx, &mut back_rx);

    back_guard = back_arc.write();
    let mut drained = 0usize;
    while let Ok(op) = back_rx.try_recv() {
      apply_one(&mut back_guard, &op, &stats, false);
      drained += 1;
    }
    if drained >= min_ops_before_swap {
      drop(back_guard);
      shared.store(Arc::clone(&back_arc));
      publish_notify.notify_waiters();
      std::mem::swap(&mut front_arc, &mut back_arc);
      std::mem::swap(&mut front_rx, &mut back_rx);
      back_guard = back_arc.write();
    }
  }
}

impl ConcurrentDataProcessor {
  pub fn new(
    initial: AugGraph,
    queue_len: usize,
    min_ops_before_swap: usize,
    publish_notify: Arc<tokio::sync::Notify>,
    stats: Option<Arc<ProcessorStats>>,
    walks_cache_size: usize,
  ) -> Self {
    let copy_a = Arc::new(RwLock::new(initial.clone()));
    let copy_b = Arc::new(RwLock::new(initial));
    let shared = Arc::new(ArcSwap::new(Arc::clone(&copy_a)));

    let (tx_a, write_rx_a) = mpsc::channel(queue_len);
    let (tx_b, write_rx_b) = mpsc::channel(queue_len);
    let op_sender = FanoutSender { tx_a, tx_b };

    let walk_tracker = if walks_cache_size > 0 {
      Some(WalkTracker::new(walks_cache_size as u64))
    } else {
      None
    };

    let shared_clone = Arc::clone(&shared);
    let notify_clone = Arc::clone(&publish_notify);
    let loop_thread = thread::spawn(move || {
      processing_loop(
        copy_a,
        copy_b,
        write_rx_a,
        write_rx_b,
        shared_clone,
        notify_clone,
        min_ops_before_swap,
        stats,
      );
    });

    ConcurrentDataProcessor {
      processing_thread: loop_thread,
      op_sender,
      shared,
      walk_tracker,
    }
  }

  #[allow(unused)]
  pub fn shutdown(self) -> thread::Result<()> {
    drop(self.op_sender);
    self.processing_thread.join()
  }
}

impl MultiGraphProcessor {
  pub fn new(settings: Settings) -> Self {
    let mgp = MultiGraphProcessor {
      subgraphs_map:   DashMap::new(),
      settings,
      loading:         AtomicBool::new(false),
      internal_stamp:  AtomicU64::new(0),
      publish_notify:  Arc::new(tokio::sync::Notify::new()),
      stats:           None,
    };
    mgp.insert_subgraph_if_does_not_exist(&String::new());
    mgp
  }

  pub fn new_with_stats(
    settings: Settings,
    stats: Arc<ProcessorStats>,
  ) -> Self {
    let mgp = MultiGraphProcessor {
      subgraphs_map:   DashMap::new(),
      settings,
      loading:         AtomicBool::new(false),
      internal_stamp:  AtomicU64::new(0),
      publish_notify:  Arc::new(tokio::sync::Notify::new()),
      stats:           Some(stats),
    };
    mgp.insert_subgraph_if_does_not_exist(&String::new());
    mgp
  }

  fn next_stamp(&self) -> u64 {
    self.internal_stamp.fetch_add(1, Ordering::SeqCst) + 1
  }

  pub fn get_tx_channel(
    &self,
    subgraph_name: &SubgraphName,
  ) -> FanoutSender {
    self.insert_subgraph_if_does_not_exist(subgraph_name)
  }

  async fn send_op(
    &self,
    subgraph_name: &SubgraphName,
    op: AugGraphOp,
  ) -> Response {
    log_trace!();

    if let Some(s) = &self.stats {
      s.record_enqueue();
    }
    if self.get_tx_channel(subgraph_name).send(op).await.is_ok() {
      Response::Ok
    } else {
      Response::Fail
    }
  }

  pub fn process_read<F>(
    &self,
    subgraph_name: &SubgraphName,
    read_function: F,
  ) -> Response
  where
    F: FnOnce(&AugGraph) -> Response,
  {
    log_trace!();

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

    let arc = subgraph.shared.load_full();
    let rw_guard = arc.read();
    let aug_graph: &AugGraph = &*rw_guard;
    log_trace!(
      "Successfully accessed AugGraph for subgraph: {:?}",
      subgraph_name
    );

    let response = read_function(aug_graph);
    log_verbose!("Executed read function for subgraph: {:?}", subgraph_name);

    response
  }

  pub async fn sync_future(
    &self,
    stamp: u64,
  ) {
    log_trace!();

    for ref_multi in self.subgraphs_map.iter() {
      let subgraph = ref_multi.value();
      let _ = subgraph.op_sender.clone().send(AugGraphOp::Stamp(stamp)).await;
    }

    loop {
      let notified = self.publish_notify.notified();

      let mut done = true;
      for ref_multi in self.subgraphs_map.iter() {
        let sub = ref_multi.value();
        let arc = sub.shared.load_full();
        let s = arc.read().stamp;
        if s < stamp {
          done = false;
          break;
        }
      }

      if done {
        break;
      }

      notified.await;
    }
  }

  /// If the ego has no walks in this subgraph, send WriteCalculate and sync so the next read sees scores.
  async fn ensure_calculated(
    &self,
    subgraph: &SubgraphName,
    ego: &NodeName,
  ) {
    let needs_calc = self.process_read(subgraph, |aug_graph| {
      match aug_graph.nodes.get_by_name(ego) {
        Some(info) if !aug_graph.mr.get_personal_hits().contains_key(&info.id) => Response::Fail,
        _ => Response::Ok,
      }
    });
    if matches!(needs_calc, Response::Fail) {
      let _ = self
        .send_op(
          subgraph,
          AugGraphOp::WriteCalculate(OpWriteCalculate {
            ego: ego.clone(),
          }),
        )
        .await;
      let stamp = self.next_stamp();
      let _ = self.send_op(subgraph, AugGraphOp::Stamp(stamp)).await;
      self.sync_future(stamp).await;
    }
  }

  pub async fn process_request(
    &self,
    req: &Request,
  ) -> Response {
    //  NOTE: Duplicated logic with `process_request_blocking`.
    //
    //  FIXME: No need to clone here, but borrow checker!!!

    log_trace!();

    if self.loading.load(Ordering::SeqCst) {
      if !matches!(&req.data, ReqData::WriteBulkEdges(_)) {
        return Response::Fail;
      }
    }

    let data = req.data.clone();

    if let Some(ego) = req.data.read_ego() {
      self.ensure_calculated(&req.subgraph, ego).await;
      // Mutual scores need reverse_score (target's score for ego), so ensure all user nodes are calculated.
      if let ReqData::ReadMutualScores(_) = &req.data {
        let list = self.process_read(&req.subgraph, |aug_graph| {
          Response::NodeList(ResNodeList {
            nodes: aug_graph
              .nodes
              .id_to_info
              .iter()
              .map(|info| (info.name.clone(),))
              .collect(),
          })
        });
        if let Response::NodeList(ResNodeList { nodes }) = list {
          for (name,) in nodes {
            if node_kind_from_prefix(&name) == Some(NodeKind::User) && name != *ego {
              self.ensure_calculated(&req.subgraph, &name).await;
            }
          }
        }
      }
      self.touch_ego_in_tracker(&req.subgraph, ego).await;
    }

    match data {
      ReqData::ResetStats => {
        if let Some(s) = &self.stats {
          s.reset();
        }
        Response::Ok
      },
      ReqData::GetStats => {
        let snap = self
          .stats
          .as_ref()
          .map(|s| s.snapshot())
          .unwrap_or(crate::processor_stats::StatsSnapshot {
            pending:    0,
            median_us:  0,
            p95_us:     0,
            p99_us:     0,
            min_us:     0,
            max_us:     0,
            count:      0,
          });
        Response::Stats(ResStats {
          pending:   snap.pending,
          median_us: snap.median_us,
          p95_us:    snap.p95_us,
          p99_us:    snap.p99_us,
          min_us:    snap.min_us,
          max_us:    snap.max_us,
          count:     snap.count,
        })
      },
      ReqData::Stamp(value) => {
        self.send_op(&req.subgraph, AugGraphOp::Stamp(value)).await
      },
      ReqData::WriteEdge(data) => {
        self.process_write_edge(&req.subgraph, &data).await
      },
      ReqData::WriteBulkEdges(data) => {
        self.loading.store(true, Ordering::SeqCst);

        self.subgraphs_map.clear();
        self.insert_subgraph_if_does_not_exist(&String::new());

        let mut contexts: HashSet<SubgraphName> = HashSet::new();
        for edge in &data.edges {
          if !edge.context.is_empty() {
            contexts.insert(edge.context.clone());
          }
        }
        for ctx in &contexts {
          self.insert_subgraph_if_does_not_exist(ctx);
        }

        let mut user_user_edges: Vec<OpWriteEdge> = vec![];
        let mut context_non_user_edges: HashMap<SubgraphName, Vec<OpWriteEdge>> =
          HashMap::new();

        for edge in data.edges {
          let op = OpWriteEdge {
            src:       edge.src,
            dst:       edge.dst,
            amount:    edge.amount,
            magnitude: edge.magnitude,
          };
          let src_kind = node_kind_from_prefix(&op.src);
          let dst_kind = node_kind_from_prefix(&op.dst);

          if matches!(
            (src_kind, dst_kind),
            (Some(NodeKind::User), Some(NodeKind::User))
          ) {
            user_user_edges.push(op);
          } else {
            context_non_user_edges
              .entry(edge.context)
              .or_default()
              .push(op);
          }
        }

        let mut aggregate_edges = user_user_edges.clone();
        for edges in context_non_user_edges.values() {
          aggregate_edges.extend(edges.iter().cloned());
        }
        let _ = self
          .send_op(&String::new(), AugGraphOp::BulkLoadEdges(aggregate_edges))
          .await;

        for ctx in &contexts {
          let mut ctx_edges = user_user_edges.clone();
          if let Some(specific) = context_non_user_edges.get(ctx) {
            ctx_edges.extend(specific.iter().cloned());
          }
          let _ = self.send_op(ctx, AugGraphOp::BulkLoadEdges(ctx_edges)).await;
        }

        let stamp = self.next_stamp();
        for ref_multi in self.subgraphs_map.iter() {
          let name = ref_multi.key().clone();
          let _ = self.send_op(&name, AugGraphOp::Stamp(stamp)).await;
        }
        self.sync_future(stamp).await;

        self.loading.store(false, Ordering::SeqCst);
        Response::Ok
      },
      ReqData::WriteCalculate(data) => {
        self
          .send_op(
            &req.subgraph,
            AugGraphOp::WriteCalculate(OpWriteCalculate {
              ego: data.ego.clone(),
            }),
          )
          .await
      },
      ReqData::WriteCreateContext => {
        let is_new = !self.subgraphs_map.contains_key(&req.subgraph);
        self.insert_subgraph_if_does_not_exist(&req.subgraph);
        if is_new {
          self.seed_context_from_aggregate(&req.subgraph).await;
        }
        Response::Ok
      },
      ReqData::WriteDeleteEdge(data) => {
        self
          .process_write_edge(
            &req.subgraph,
            &OpWriteEdge {
              src:       data.src,
              dst:       data.dst,
              amount:    0.0,
              magnitude: data.index as u32,
            },
          )
          .await
      },
      ReqData::WriteDeleteNode(data) => {
        let op = AugGraphOp::DeleteNode(data.node.clone());
        let resp = self.send_op(&req.subgraph, op.clone()).await;
        if matches!(resp, Response::Ok) {
          let default_ctx = String::new();
          let _ = self.send_op(&default_ctx, op).await;
        }
        resp
      },
      ReqData::WriteZeroOpinion(data) => {
        self
          .send_op(&req.subgraph, AugGraphOp::WriteZeroOpinion(data.clone()))
          .await
      },
      ReqData::WriteReset => {
        self.subgraphs_map.clear();
        self.insert_subgraph_if_does_not_exist(&String::new());
        Response::Ok
      },
      ReqData::WriteRecalculateClustering => {
        self
          .send_op(&req.subgraph, AugGraphOp::WriteRecalculateClustering)
          .await
      },
      ReqData::WriteFetchNewEdges(_) => {
        self.process_read(&req.subgraph, |_| Response::NotImplemented)
      },
      ReqData::WriteNewEdgesFilter(_) => {
        self.process_read(&req.subgraph, |_| Response::NotImplemented)
      },
      ReqData::ReadNewEdgesFilter(_) => {
        self.process_read(&req.subgraph, |_| Response::NotImplemented)
      },
      ReqData::ReadScores(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_scores(data),
          })
        })
      },
      ReqData::ReadNodeScore(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_node_score(data),
          })
        })
      },
      ReqData::ReadGraph(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Graph(ResGraph {
            graph: aug_graph.read_graph(data),
          })
        })
      },
      ReqData::ReadNeighbors(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_neighbors(data),
          })
        })
      },
      ReqData::ReadNodeList => self.process_read(&req.subgraph, |aug_graph| {
        Response::NodeList(ResNodeList {
          nodes: aug_graph
            .nodes
            .id_to_info
            .iter()
            .map(|info| (info.name.clone(),))
            .collect(),
        })
      }),
      ReqData::ReadEdges => self.process_read(&req.subgraph, |aug_graph| {
        let mut edges = vec![];
        edges.reserve(aug_graph.nodes.id_to_info.len() * 2);

        for (src_id, info) in aug_graph.nodes.id_to_info.iter().enumerate() {
          if let Some(data) = aug_graph.mr.graph.get_node_data(src_id) {
            let src_name = &info.name;

            for (dst_id, weight) in data.get_outgoing_edges() {
              match aug_graph.nodes.get_by_id(dst_id) {
                Some(x) => edges.push(EdgeResult {
                  src: src_name.to_string(),
                  dst: x.name.clone(),
                  weight,
                }),
                None => log_error!("Node does not exist: {}", dst_id),
              }
            }
          };
        }

        Response::Edges(ResEdges {
          edges,
        })
      }),
      ReqData::ReadConnected(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          match aug_graph.nodes.get_by_name(&data.node) {
            Some(src) => Response::Connections(ResConnections {
              connections: aug_graph
                .mr
                .graph
                .get_node_data(src.id)
                .unwrap()
                .get_outgoing_edges()
                .map(|(dst_id, _)| ConnectionResult {
                  src: data.node.clone(),
                  dst: aug_graph.nodes.get_by_id(dst_id).unwrap().name.clone(),
                })
                .collect(),
            }),
            None => {
              log_error!("Node not found: {:?}", data.node);
              Response::Fail
            },
          }
        })
      },
      ReqData::ReadMutualScores(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_mutual_scores(data),
          })
        })
      },
      ReqData::Sync(stamp) => {
        self.sync_future(stamp).await;
        Response::Ok
      },
    }
  }

  async fn process_write_edge(
    &self,
    subgraph_name: &SubgraphName,
    data: &OpWriteEdge,
  ) -> Response {
    //  NOTE: Duplicated logic with `process_write_edge_blocking`.

    log_trace!("{:?} {:?}", subgraph_name, data);

    if data.src == data.dst {
      log_error!("Self-reference is not allowed.");
      return Response::Fail;
    }

    let src_kind_opt = node_kind_from_prefix(&data.src);
    let dst_kind_opt = node_kind_from_prefix(&data.dst);

    let response = match (src_kind_opt, dst_kind_opt) {
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
        //  TODO
        Response::Ok
      },
      (Some(NodeKind::PollVariant), Some(NodeKind::Poll)) => {
        //  TODO
        Response::Ok
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
        let op = AugGraphOp::WriteEdge(OpWriteEdge {
          src:       data.src.clone(),
          dst:       data.dst.clone(),
          amount:    data.amount,
          magnitude: data.magnitude,
        });
        let resp = self.send_op(subgraph_name, op.clone()).await;
        if matches!(resp, Response::Ok) {
          let default_ctx = String::new();
          let _ = self.send_op(&default_ctx, op).await;
        }
        resp
      },
    };
    response
  }

  /// Seeds the given (new) context with user-user edges from the "" aggregate. Does not update tracking or "".
  async fn seed_context_from_aggregate(
    &self,
    subgraph_name: &SubgraphName,
  ) {
    let default_ctx = String::new();
    let response = self.process_read(&default_ctx, |aug_graph| {
      let mut edges = vec![];
      edges.reserve(aug_graph.nodes.id_to_info.len() * 2);
      for (src_id, info) in aug_graph.nodes.id_to_info.iter().enumerate() {
        if let Some(data) = aug_graph.mr.graph.get_node_data(src_id) {
          let src_name = &info.name;
          for (dst_id, weight) in data.get_outgoing_edges() {
            match aug_graph.nodes.get_by_id(dst_id) {
              Some(x) => edges.push(EdgeResult {
                src: src_name.to_string(),
                dst: x.name.clone(),
                weight,
              }),
              None => log_error!("Node does not exist: {}", dst_id),
            }
          }
        }
      }
      Response::Edges(ResEdges { edges })
    });

    if let Response::Edges(ResEdges { edges }) = response {
      let tx = self.get_tx_channel(subgraph_name);
      for edge in edges {
        if node_kind_from_prefix(&edge.src) == Some(NodeKind::User)
          && node_kind_from_prefix(&edge.dst) == Some(NodeKind::User)
          && edge.weight != 0.0
        {
          let _ = tx
            .send(AugGraphOp::WriteEdge(OpWriteEdge {
              src:       edge.src,
              dst:       edge.dst,
              amount:    edge.weight,
              magnitude: 0,
            }))
            .await;
        }
      }
    }
  }

  /// Records ego usage in the walk tracker and sends ClearEgo for any evicted egos.
  async fn touch_ego_in_tracker(
    &self,
    subgraph_name: &SubgraphName,
    ego: &NodeName,
  ) {
    let ego_id_opt = RefCell::new(None);
    self.process_read(subgraph_name, |aug_graph| {
      if let Some(info) = aug_graph.nodes.get_by_name(ego) {
        *ego_id_opt.borrow_mut() = Some(info.id);
      }
      Response::Ok
    });
    let ego_id = match *ego_id_opt.borrow() {
      Some(id) => id,
      None => return,
    };

    let evicted_ids: Vec<NodeId> = if let Some(entry) = self.subgraphs_map.get(subgraph_name) {
      if let Some(ref tracker) = entry.walk_tracker {
        tracker.touch(ego_id);
        tracker.drain_evicted()
      } else {
        vec![]
      }
    } else {
      vec![]
    };

    for evicted_id in evicted_ids {
      let _ = self
        .send_op(subgraph_name, AugGraphOp::ClearEgo(evicted_id))
        .await;
    }
  }

  pub fn insert_subgraph_if_does_not_exist(
    &self,
    subgraph_name: &SubgraphName,
  ) -> FanoutSender {
    log_trace!();

    self
      .subgraphs_map
      .entry(subgraph_name.clone())
      .or_insert_with(|| {
        log_trace!("Create subgraph");
        GraphProcessor::new(
          AugGraph::new(self.settings.clone()),
          self.settings.subgraph_queue_capacity,
          self.settings.min_ops_before_swap,
          self.publish_notify.clone(),
          self.stats.clone(),
          self.settings.walks_cache_size,
        )
      })
      .op_sender
      .clone()
  }

  async fn process_user_to_user_edge(
    &self,
    subgraph_name: &SubgraphName,
    src: &NodeName,
    dst: &NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) -> Response {
    //  NOTE: Duplicated logic with `process_user_to_user_edge_blocking`.

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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::data::{EdgeResult, FilterOptions, OpReadScores, ResEdges, ResScores};
  use crate::data::Weight;
  use std::sync::atomic::Ordering;
  use std::time::Duration;

  fn default_processor() -> MultiGraphProcessor {
    MultiGraphProcessor::new(Settings::default())
  }

  async fn sync(proc: &MultiGraphProcessor) {
    let _ = proc.process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::Sync(1),
    }).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
  }

  fn edges_from_response(response: Response) -> Vec<(String, String, Weight)> {
    match response {
      Response::Edges(ResEdges { edges }) => edges
        .into_iter()
        .map(|e: EdgeResult| (e.src, e.dst, e.weight))
        .collect(),
      _ => vec![],
    }
  }

  #[tokio::test]
  async fn context_aggregate_null_context_last_write_wins() {
    // Verbatim aggregate: "" receives each edge write as-is; last write wins for same (src, dst).
    let proc = default_processor();
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "B1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "B1".into(),
        dst:       "U2".into(),
        amount:    2.0,
        magnitude: 0,
      }),
    }).await;
    sync(&proc).await;
    let response = proc.process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::ReadEdges,
    }).await;
    let edges = edges_from_response(response);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].0, "B1");
    assert_eq!(edges[0].1, "U2");
    assert!((edges[0].2 - 2.0).abs() < 1e-6, "expected weight ~2.0 (last write wins), got {}", edges[0].2);
  }

  #[tokio::test]
  async fn context_aggregate_null_context_contains_all_users() {
    let proc = default_processor();
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "U1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "U1".into(),
        dst:       "U3".into(),
        amount:    2.0,
        magnitude: 0,
      }),
    }).await;
    sync(&proc).await;
    let response = proc.process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::ReadEdges,
    }).await;
    let edges = edges_from_response(response);
    let expected = vec![
      ("U1".to_string(), "U2".to_string(), 1.0),
      ("U1".to_string(), "U3".to_string(), 2.0),
    ];
    assert_eq!(edges.len(), expected.len());
    for exp in &expected {
      assert!(edges.iter().any(|e| e.0 == exp.0 && e.1 == exp.1 && (e.2 - exp.2).abs() < 1e-9));
    }
  }

  #[tokio::test]
  async fn context_aggregate_delete_contexted_edge() {
    // Verbatim: deleting from X sends WriteEdge(0) to ""; edge is removed or zeroed in "".
    let proc = default_processor();
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "B1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "B1".into(),
        dst:       "U2".into(),
        amount:    2.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteDeleteEdge(OpWriteDeleteEdge {
        src:   "B1".into(),
        dst:   "U2".into(),
        index: -1,
      }),
    }).await;
    sync(&proc).await;
    let response = proc.process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::ReadEdges,
    }).await;
    let edges = edges_from_response(response);
    // After verbatim delete, "" has WriteEdge(0); graph may omit zero-weight edges from ReadEdges.
    assert!(edges.is_empty() || (edges.len() == 1 && (edges[0].2 - 0.0).abs() < 1e-6),
      "expected no edges or single edge with weight 0, got {} edges", edges.len());
  }

  #[tokio::test]
  async fn context_aggregate_null_context_invariant() {
    // Verbatim: delete from X (sends 0 to ""), then re-add 1.0 from X; "" ends with 1.0.
    let proc = default_processor();
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "B1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "B1".into(),
        dst:       "U2".into(),
        amount:    2.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteDeleteEdge(OpWriteDeleteEdge {
        src:   "B1".into(),
        dst:   "U2".into(),
        index: -1,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "B1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
      }),
    }).await;
    sync(&proc).await;
    let response = proc.process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::ReadEdges,
    }).await;
    let edges = edges_from_response(response);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].0, "B1");
    assert_eq!(edges[0].1, "U2");
    assert!((edges[0].2 - 1.0).abs() < 1e-6, "expected weight ~1.0 (verbatim), got {}", edges[0].2);
  }

  #[tokio::test]
  async fn context_aggregate_user_edges_dup() {
    let proc = default_processor();
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "U1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "U1".into(),
        dst:       "U3".into(),
        amount:    2.0,
        magnitude: 0,
      }),
    }).await;
    sync(&proc).await; // ensure "" has edges before we seed Y from it
    let _ = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::WriteCreateContext,
    }).await;
    sync(&proc).await;
    let response = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::ReadEdges,
    }).await;
    let edges = edges_from_response(response);
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().any(|e| e.0 == "U1" && e.1 == "U2" && (e.2 - 1.0).abs() < 1e-9));
    assert!(edges.iter().any(|e| e.0 == "U1" && e.1 == "U3" && (e.2 - 2.0).abs() < 1e-9));
  }

  #[tokio::test]
  async fn context_aggregate_non_user_edges_no_dup() {
    let proc = default_processor();
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "U1".into(),
        dst:       "C2".into(),
        amount:    1.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "U1".into(),
        dst:       "C3".into(),
        amount:    2.0,
        magnitude: 0,
      }),
    }).await;
    let _ = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::WriteCreateContext,
    }).await;
    sync(&proc).await;
    let response = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::ReadEdges,
    }).await;
    let edges = edges_from_response(response);
    assert_eq!(edges.len(), 0);
  }

  #[tokio::test]
  async fn bulk_load_single_context() {
    let proc = default_processor();
    let edges = vec![
      BulkEdge {
        src:       "U1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
        context:   String::new(),
      },
      BulkEdge {
        src:       "U1".into(),
        dst:       "U3".into(),
        amount:    2.0,
        magnitude: 0,
        context:   String::new(),
      },
    ];
    let resp = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::WriteBulkEdges(OpWriteBulkEdges { edges }),
      })
      .await;
    assert!(matches!(resp, Response::Ok));
    let response = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::ReadEdges,
      })
      .await;
    let loaded = edges_from_response(response);
    assert_eq!(loaded.len(), 2);
    let scores_resp = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::ReadScores(OpReadScores {
          ego:           "U1".into(),
          score_options: FilterOptions::default(),
        }),
      })
      .await;
    match scores_resp {
      Response::Scores(ResScores { scores }) => assert!(!scores.is_empty()),
      _ => panic!("expected scores"),
    }
  }

  #[tokio::test]
  async fn bulk_load_multi_context() {
    let proc = default_processor();
    let edges = vec![
      BulkEdge {
        src:       "U1".into(),
        dst:       "U2".into(),
        amount:    1.0,
        magnitude: 0,
        context:   String::new(),
      },
      BulkEdge {
        src:       "U1".into(),
        dst:       "B1".into(),
        amount:    3.0,
        magnitude: 0,
        context:   "X".into(),
      },
    ];
    let _ = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::WriteBulkEdges(OpWriteBulkEdges { edges }),
      })
      .await;
    let agg = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::ReadEdges,
      })
      .await;
    let agg_edges = edges_from_response(agg);
    assert_eq!(agg_edges.len(), 2);
    let ctx_x = proc
      .process_request(&Request {
        subgraph: "X".into(),
        data:     ReqData::ReadEdges,
      })
      .await;
    let x_edges = edges_from_response(ctx_x);
    assert_eq!(x_edges.len(), 2);
  }

  #[tokio::test]
  async fn bulk_load_lazy_calc_on_read() {
    let proc = default_processor();
    let edges = vec![BulkEdge {
      src:       "U1".into(),
      dst:       "U2".into(),
      amount:    1.0,
      magnitude: 0,
      context:   String::new(),
    }];
    let _ = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::WriteBulkEdges(OpWriteBulkEdges { edges }),
      })
      .await;
    let scores_resp = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::ReadScores(OpReadScores {
          ego:           "U1".into(),
          score_options: FilterOptions::default(),
        }),
      })
      .await;
    match scores_resp {
      Response::Scores(ResScores { scores }) => {
        assert!(!scores.is_empty());
        assert!(scores.iter().any(|s| s.target == "U2" && s.score > 0.0));
      },
      _ => panic!("expected scores"),
    }
  }

  #[tokio::test]
  async fn bulk_load_blocks_reads() {
    let proc = default_processor();
    proc.loading.store(true, Ordering::SeqCst);
    let response = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::ReadEdges,
      })
      .await;
    proc.loading.store(false, Ordering::SeqCst);
    assert!(matches!(response, Response::Fail));
  }

  #[tokio::test]
  async fn normal_write_no_auto_calc() {
    let proc = default_processor();
    let _ = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::WriteEdge(OpWriteEdge {
          src:       "U1".into(),
          dst:       "U2".into(),
          amount:    1.0,
          magnitude: 0,
        }),
      })
      .await;
    sync(&proc).await;
    let mut scores_resp = proc
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::ReadScores(OpReadScores {
          ego:           "U1".into(),
          score_options: FilterOptions::default(),
        }),
      })
      .await;
    for _ in 0..9 {
      if let Response::Scores(ResScores { scores }) = &scores_resp {
        if !scores.is_empty() {
          return;
        }
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
      scores_resp = proc
        .process_request(&Request {
          subgraph: String::new(),
          data:     ReqData::ReadScores(OpReadScores {
            ego:           "U1".into(),
            score_options: FilterOptions::default(),
          }),
        })
        .await;
    }
    match scores_resp {
      Response::Scores(ResScores { scores }) => assert!(!scores.is_empty(), "expected scores from lazy calc"),
      other => panic!("expected scores, got {:?}", other),
    }
  }

  #[tokio::test]
  async fn nonblocking() {
    let notify = Arc::new(tokio::sync::Notify::new());
      let proc = GraphProcessor::new(
        AugGraph::new(Settings::default()),
        10,
        1,
        Arc::clone(&notify),
        None,
        0,
      );
    let _ = proc.op_sender.send(AugGraphOp::Stamp(1)).await;
    let _ = proc.op_sender.send(AugGraphOp::Stamp(2)).await;
    let _ = proc.op_sender.send(AugGraphOp::Stamp(3)).await;

    for _ in 0..20 {
      let n = notify.notified();
      let s = proc.shared.load().read().stamp;
      if s >= 3 {
        break;
      }
      n.await;
    }
    assert_eq!(proc.shared.load().read().stamp, 3);
    proc.shutdown().ok();
  }
}
