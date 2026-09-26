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
use tokio::sync::{mpsc, watch};

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crate::processor_stats::ProcessorStats;
use crate::walk_tracker::WalkTracker;
use meritrank_core::NodeId;

/// An operation with its position in the dispatcher's global sequence.
#[derive(Clone)]
pub struct SeqOp {
  pub seq: u64,
  pub op:  Arc<AugGraphOp>,
}

/// Urgent operations end a batch: the worker publishes right after applying them.
fn is_urgent(op: &AugGraphOp) -> bool {
  matches!(op, AugGraphOp::Stamp(_) | AugGraphOp::WriteCalculate(_))
}

/// Sends operations into a subgraph's single queue.
#[derive(Clone)]
pub struct OpSender {
  tx:        mpsc::Sender<SeqOp>,
  /// Numbering for standalone use of one processor (`send`). A `MultiGraphProcessor` numbers
  /// operations with its dispatcher instead (`send_seq`); the two are never mixed.
  local_seq: Arc<tokio::sync::Mutex<u64>>,
}

impl OpSender {
  /// Numbers the operation locally and enqueues it. Returns its sequence number.
  pub async fn send(
    &self,
    op: AugGraphOp,
  ) -> Result<u64, mpsc::error::SendError<SeqOp>> {
    let mut last = self.local_seq.lock().await;
    let seq = *last + 1;
    self
      .tx
      .send(SeqOp {
        seq,
        op: Arc::new(op),
      })
      .await?;
    *last = seq;
    Ok(seq)
  }

  /// Enqueues an operation numbered by the caller, who guarantees increasing numbers.
  pub async fn send_seq(
    &self,
    seq: u64,
    op: Arc<AugGraphOp>,
  ) -> Result<(), mpsc::error::SendError<SeqOp>> {
    self.tx.send(SeqOp { seq, op }).await
  }
}

/// A subgraph: two replica copies of the graph, one published for reads, one written by the
/// worker thread, and the single queue that feeds them.
pub struct ConcurrentDataProcessor {
  #[allow(unused)]
  processing_thread: thread::JoinHandle<()>,
  pub op_sender:     OpSender,
  pub shared:        Arc<ArcSwap<RwLock<AugGraph>>>,
  /// Sequence number of the last operation visible in the published copy.
  pub published_seq: watch::Receiver<u64>,
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
  /// Every mutating operation takes this lock, gets the next sequence number and is enqueued into
  /// all of its target subgraphs before the lock is released, so every subgraph sees operations
  /// in one global order. Reads never take it.
  dispatcher:        tokio::sync::Mutex<u64>,
}

/// Where a dispatched operation goes.
enum Targets<'a> {
  One(&'a SubgraphName),
  /// These subgraphs, created if absent, in this order.
  List(Vec<SubgraphName>),
  /// Every existing subgraph, after creating `ensure` if it is absent.
  All { ensure: &'a SubgraphName },
}

/// SplitMix64 finaliser.
fn mix64(mut z: u64) -> u64 {
  z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
  z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
  z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
  z ^ (z >> 31)
}

/// Stable 64-bit key of a subgraph name (FNV-1a), for deriving its random streams.
fn stream_key(name: &str) -> u64 {
  name.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
    (h ^ b as u64).wrapping_mul(0x0100_0000_01B3)
  })
}

/// Seed of the random stream an operation uses: both copies apply it with the same stream, so
/// they stay identical, and a rerun of the same sequence reproduces every walk.
fn op_seed(
  seed: u64,
  stream: u64,
  seq: u64,
) -> u64 {
  mix64(mix64(seed ^ mix64(stream)) ^ seq)
}

fn processing_loop(
  copy_a: Arc<RwLock<AugGraph>>,
  copy_b: Arc<RwLock<AugGraph>>,
  mut rx: mpsc::Receiver<SeqOp>,
  shared: Arc<ArcSwap<RwLock<AugGraph>>>,
  publish_notify: Arc<tokio::sync::Notify>,
  published_tx: watch::Sender<u64>,
  max_batch: usize,
  stats: Option<Arc<ProcessorStats>>,
  stream: u64,
) {
  let mut front_arc = copy_a;
  let mut back_arc = copy_b;
  let max_batch = max_batch.max(1);
  shared.store(Arc::clone(&front_arc));

  let apply = |graph: &mut AugGraph, op: &SeqOp, record_stats: bool| {
    let start = Instant::now();
    let seed = op_seed(graph.settings.seed, stream, op.seq);
    graph.mr.reseed(seed);
    graph.apply_op(&op.op);
    if record_stats {
      if let Some(s) = &stats {
        s.record_applied(start.elapsed());
      }
    }
  };

  loop {
    // Wait for work without holding any lock.
    let first = match rx.blocking_recv() {
      Some(op) => op,
      None => return,
    };
    let mut batch = vec![first];
    while batch.len() < max_batch && !is_urgent(&batch[batch.len() - 1].op) {
      match rx.try_recv() {
        Ok(op) => batch.push(op),
        Err(_) => break,
      }
    }

    {
      let mut back = back_arc.write();
      for op in &batch {
        apply(&mut back, op, true);
      }
    }

    // The back copy now holds everything the published copy holds plus the batch, so publishing
    // it never moves the published state backwards.
    let last_seq = batch[batch.len() - 1].seq;
    shared.store(Arc::clone(&back_arc));
    published_tx.send_replace(last_seq);
    publish_notify.notify_waiters();
    std::mem::swap(&mut front_arc, &mut back_arc);

    // Bring the new back copy level with the published one by replaying the same batch with the
    // same random streams. Readers still on it finish first; nobody waits on it while idle.
    let mut back = back_arc.write();
    for op in &batch {
      apply(&mut back, op, false);
    }
  }
}

/// Runs `read` on the published copy. The copy is re-checked after taking its lock, so a read
/// never runs on a copy that stopped being published in between.
pub fn read_published<F, T>(
  shared: &ArcSwap<RwLock<AugGraph>>,
  read: F,
) -> T
where
  F: FnOnce(&AugGraph) -> T,
{
  loop {
    let arc = shared.load_full();
    let guard = arc.read();
    if Arc::ptr_eq(&shared.load(), &arc) {
      return read(&guard);
    }
  }
}

impl ConcurrentDataProcessor {
  /// A standalone processor (random streams keyed by the empty subgraph name).
  pub fn new(
    initial: AugGraph,
    queue_len: usize,
    max_batch: usize,
    publish_notify: Arc<tokio::sync::Notify>,
    stats: Option<Arc<ProcessorStats>>,
    walks_cache_size: usize,
  ) -> Self {
    Self::new_for_subgraph(
      "",
      initial,
      queue_len,
      max_batch,
      publish_notify,
      stats,
      walks_cache_size,
    )
  }

  pub fn new_for_subgraph(
    name: &str,
    initial: AugGraph,
    queue_len: usize,
    max_batch: usize,
    publish_notify: Arc<tokio::sync::Notify>,
    stats: Option<Arc<ProcessorStats>>,
    walks_cache_size: usize,
  ) -> Self {
    let copy_a = Arc::new(RwLock::new(initial.clone()));
    let copy_b = Arc::new(RwLock::new(initial));
    let shared = Arc::new(ArcSwap::new(Arc::clone(&copy_a)));

    let (tx, rx) = mpsc::channel(queue_len.max(1));
    let op_sender = OpSender {
      tx,
      local_seq: Arc::new(tokio::sync::Mutex::new(0)),
    };
    let (published_tx, published_seq) = watch::channel(0);

    let walk_tracker = if walks_cache_size > 0 {
      Some(WalkTracker::new(walks_cache_size as u64))
    } else {
      None
    };

    let shared_clone = Arc::clone(&shared);
    let notify_clone = Arc::clone(&publish_notify);
    let stream = stream_key(name);
    let loop_thread = thread::spawn(move || {
      processing_loop(
        copy_a,
        copy_b,
        rx,
        shared_clone,
        notify_clone,
        published_tx,
        max_batch,
        stats,
        stream,
      );
    });

    ConcurrentDataProcessor {
      processing_thread: loop_thread,
      op_sender,
      shared,
      published_seq,
      walk_tracker,
    }
  }

  /// Runs `read` on the published copy (see `read_published`).
  pub fn read<F, T>(
    &self,
    read: F,
  ) -> T
  where
    F: FnOnce(&AugGraph) -> T,
  {
    read_published(&self.shared, read)
  }

  #[allow(unused)]
  pub fn shutdown(self) -> thread::Result<()> {
    drop(self.op_sender);
    self.processing_thread.join()
  }
}

impl MultiGraphProcessor {
  pub fn new(settings: Settings) -> Self {
    Self::with_stats(settings, None)
  }

  pub fn new_with_stats(
    settings: Settings,
    stats: Arc<ProcessorStats>,
  ) -> Self {
    Self::with_stats(settings, Some(stats))
  }

  fn with_stats(
    settings: Settings,
    stats: Option<Arc<ProcessorStats>>,
  ) -> Self {
    let mgp = MultiGraphProcessor {
      subgraphs_map:   DashMap::new(),
      settings,
      loading:         AtomicBool::new(false),
      internal_stamp:  AtomicU64::new(0),
      publish_notify:  Arc::new(tokio::sync::Notify::new()),
      stats,
      dispatcher:      tokio::sync::Mutex::new(0),
    };
    mgp.insert_subgraph_if_does_not_exist(&String::new());
    mgp
  }

  fn next_stamp(&self) -> u64 {
    self.internal_stamp.fetch_add(1, Ordering::SeqCst) + 1
  }

  /// Numbers `op` and enqueues it into every target subgraph, under the dispatcher lock.
  async fn dispatch(
    &self,
    targets: Targets<'_>,
    op: AugGraphOp,
  ) -> Response {
    let op = Arc::new(op);
    let mut last_seq = self.dispatcher.lock().await;

    let names: Vec<SubgraphName> = match targets {
      Targets::One(name) => {
        self.insert_subgraph_if_does_not_exist(name);
        vec![name.clone()]
      },
      Targets::List(names) => {
        for name in &names {
          self.insert_subgraph_if_does_not_exist(name);
        }
        names
      },
      Targets::All { ensure } => {
        self.insert_subgraph_if_does_not_exist(ensure);
        let mut names: Vec<SubgraphName> =
          self.subgraphs_map.iter().map(|r| r.key().clone()).collect();
        names.sort();
        names
      },
    };
    // Clone the senders out of the map: no map guard may be held across an await.
    let senders: Vec<OpSender> = names
      .iter()
      .filter_map(|n| self.subgraphs_map.get(n).map(|p| p.op_sender.clone()))
      .collect();

    *last_seq += 1;
    let seq = *last_seq;
    let mut ok = true;
    for sender in senders {
      if let Some(s) = &self.stats {
        s.record_enqueue();
      }
      if sender.send_seq(seq, Arc::clone(&op)).await.is_err() {
        ok = false;
      }
    }
    if ok {
      Response::Ok
    } else {
      Response::Fail
    }
  }

  async fn send_op(
    &self,
    subgraph_name: &SubgraphName,
    op: AugGraphOp,
  ) -> Response {
    log_trace!();
    self.dispatch(Targets::One(subgraph_name), op).await
  }

  /// `subgraph` and the null context, once each.
  fn with_null_context(subgraph_name: &SubgraphName) -> Vec<SubgraphName> {
    if subgraph_name.is_empty() {
      vec![String::new()]
    } else {
      vec![subgraph_name.clone(), String::new()]
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

    let shared = match self.subgraphs_map.get(subgraph_name) {
      Some(subgraph) => Arc::clone(&subgraph.shared),
      None => {
        log_warning!("Subgraph not found for name: {:?}", subgraph_name);
        return Response::Fail;
      },
    };
    read_published(&shared, read_function)
  }

  pub async fn sync_future(
    &self,
    stamp: u64,
  ) {
    log_trace!();

    let _ = self
      .dispatch(
        Targets::All {
          ensure: &String::new(),
        },
        AugGraphOp::Stamp(stamp),
      )
      .await;

    loop {
      let notified = self.publish_notify.notified();

      let done = self
        .subgraphs_map
        .iter()
        .all(|r| read_published(&r.value().shared, |g| g.stamp) >= stamp);
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
        Some(info) if !aug_graph.mr.is_calculated(info.id) => Response::Fail,
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
      self.sync_future(stamp).await;
    }
  }

  pub async fn process_request(
    &self,
    req: &Request,
  ) -> Response {
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

        {
          let _ordered = self.dispatcher.lock().await;
          self.subgraphs_map.clear();
          self.insert_subgraph_if_does_not_exist(&String::new());
        }

        // Ordered collections: the load must not depend on hash iteration order.
        let mut contexts: BTreeSet<SubgraphName> = BTreeSet::new();
        for edge in &data.edges {
          if !edge.context.is_empty() {
            contexts.insert(edge.context.clone());
          }
        }
        for ctx in &contexts {
          self.insert_subgraph_if_does_not_exist(ctx);
        }

        let mut user_user_edges: Vec<OpWriteEdge> = vec![];
        let mut context_non_user_edges: BTreeMap<SubgraphName, Vec<OpWriteEdge>> =
          BTreeMap::new();

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
        let was_new = {
          let _ordered = self.dispatcher.lock().await;
          let exists = self.subgraphs_map.contains_key(&req.subgraph);
          self.insert_subgraph_if_does_not_exist(&req.subgraph);
          !exists
        };
        if was_new {
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
        self
          .dispatch(
            Targets::List(Self::with_null_context(&req.subgraph)),
            AugGraphOp::DeleteNode(data.node.clone()),
          )
          .await
      },
      ReqData::WriteZeroOpinion(data) => {
        self
          .send_op(&req.subgraph, AugGraphOp::WriteZeroOpinion(data.clone()))
          .await
      },
      ReqData::WriteReset => {
        let _ordered = self.dispatcher.lock().await;
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
        self
          .dispatch(Targets::List(Self::with_null_context(subgraph_name)), op)
          .await
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
      for edge in edges {
        if node_kind_from_prefix(&edge.src) == Some(NodeKind::User)
          && node_kind_from_prefix(&edge.dst) == Some(NodeKind::User)
          && edge.weight != 0.0
        {
          let _ = self
            .send_op(
              subgraph_name,
              AugGraphOp::WriteEdge(OpWriteEdge {
                src:       edge.src,
                dst:       edge.dst,
                amount:    edge.weight,
                magnitude: 0,
              }),
            )
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
    let shared = match self.subgraphs_map.get(subgraph_name) {
      Some(entry) => Arc::clone(&entry.shared),
      None => return,
    };
    let ego_id =
      match read_published(&shared, |g| g.nodes.get_by_name(ego).map(|i| i.id)) {
        Some(id) => id,
        None => return,
      };

    let evicted_ids: Vec<NodeId> = {
      match self.subgraphs_map.get(subgraph_name) {
        Some(entry) => {
          if let Some(ref tracker) = entry.walk_tracker {
            tracker.touch(ego_id);
            tracker.drain_evicted()
          } else {
            vec![]
          }
        },
        None => vec![],
      }
    };

    for evicted_id in evicted_ids {
      let _ = self
        .send_op(subgraph_name, AugGraphOp::ClearEgo(evicted_id))
        .await;
    }
  }

  /// Creates the subgraph if it is absent. Callers that must order the creation against
  /// operations hold the dispatcher lock.
  pub fn insert_subgraph_if_does_not_exist(
    &self,
    subgraph_name: &SubgraphName,
  ) {
    log_trace!();

    if self.subgraphs_map.contains_key(subgraph_name) {
      return;
    }
    self
      .subgraphs_map
      .entry(subgraph_name.clone())
      .or_insert_with(|| {
        log_trace!("Create subgraph");
        GraphProcessor::new_for_subgraph(
          subgraph_name,
          AugGraph::new(self.settings.clone()),
          self.settings.subgraph_queue_capacity,
          self.settings.min_ops_before_swap,
          self.publish_notify.clone(),
          self.stats.clone(),
          self.settings.walks_cache_size,
        )
      });
  }

  /// A User→User edge takes part in every context: it is enqueued into all subgraphs as one
  /// dispatched operation.
  async fn process_user_to_user_edge(
    &self,
    subgraph_name: &SubgraphName,
    src: &NodeName,
    dst: &NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) -> Response {
    log_trace!();
    let response = self
      .dispatch(
        Targets::All {
          ensure: subgraph_name,
        },
        AugGraphOp::WriteEdge(OpWriteEdge {
          src: src.clone(),
          dst: dst.clone(),
          amount,
          magnitude,
        }),
      )
      .await;
    if !matches!(response, Response::Ok) {
      log_error!("Failed to send WriteEdge operation to a subgraph");
    }
    response
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::data::{EdgeResult, FilterOptions, OpReadScores, ResEdges, ResScores};
  use crate::data::Weight;
  use std::sync::atomic::Ordering;

  fn default_processor() -> MultiGraphProcessor {
    MultiGraphProcessor::new(Settings::default())
  }

  /// Waits for the processor to apply a sync point (process_request(Sync) already awaits).
  async fn sync(proc: &MultiGraphProcessor) {
    let _ = proc.process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::Sync(1),
    }).await;
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
    for _ in 0..100 {
      if let Response::Scores(ResScores { scores }) = &scores_resp {
        if !scores.is_empty() {
          return;
        }
      }
      tokio::task::yield_now().await;
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
