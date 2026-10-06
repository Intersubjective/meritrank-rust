use crate::aug_graph::*;
use crate::data::*;
use crate::settings::*;
use crate::utils::log::*;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use parking_lot::RwLock;
use crate::data::Weight;
use tokio::sync::{mpsc, watch};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crate::processor_stats::ProcessorStats;
use crate::residency::{Lease, Residency};
use meritrank_core::NodeId;

/// Reads whose working set exceeded the walk cache (pinned in portions; snapshots off).
static OVER_CAPACITY: AtomicU64 = AtomicU64::new(0);

/// How many reads so far needed more peer frames than the walk cache holds (each one logs, sparsely,
/// the over-capacity warning). With snapshots on, reads never do.
pub fn over_capacity_reads() -> u64 {
  OVER_CAPACITY.load(Ordering::Relaxed)
}

/// A read run on a published copy (shared with a blocking thread).
type ReadFn = Arc<dyn Fn(&AugGraph) -> Response + Send + Sync>;

/// Bytes of a sample waiting for admission (its footprint and counters, approximately).
fn sample_bytes(s: &meritrank_core::FrameSample) -> usize {
  s.visits.len() * 40 + 256
}

/// An operation with its position in the dispatcher's global sequence.
#[derive(Clone)]
pub struct SeqOp {
  pub seq: u64,
  pub op:  Arc<AugGraphOp>,
}

/// Urgent operations end a batch: the worker publishes right after applying them.
fn is_urgent(op: &AugGraphOp) -> bool {
  matches!(
    op,
    AugGraphOp::Stamp(_)
      | AugGraphOp::WriteCalculate(_)
      | AugGraphOp::Barrier
      | AugGraphOp::EnsureCalculated(_)
  )
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
  /// Which egos keep their walks (MERITRANK_WALKS_CACHE_SIZE; 0 = unlimited).
  pub residency:     Arc<Residency>,
  /// Reverse-score counters of the reads of this subgraph (D14).
  pub read_stats:    Arc<ReadStats>,
  /// Both buffer copies (replica checks in tests).
  copies:            [Arc<RwLock<AugGraph>>; 2],
  /// With `Settings::record_ops`: every operation applied, in order.
  recorded:          Option<Arc<std::sync::Mutex<Vec<(u64, AugGraphOp)>>>>,
}

/// Reverse-score counters of a subgraph's reads (D14).
#[derive(Default)]
pub struct ReadStats {
  pub reverse_from_frame:    AtomicU64,
  pub reverse_from_snapshot: AtomicU64,
  pub reverse_sampled:       AtomicU64,
  pub sample_walks:          AtomicU64,
  pub admissions_skipped:    AtomicU64,
}

pub type GraphProcessor = ConcurrentDataProcessor;

pub struct MultiGraphProcessor {
  pub subgraphs_map: DashMap<SubgraphName, GraphProcessor>,
  settings:          Settings,
  loading:           AtomicBool,
  publish_notify:    Arc<tokio::sync::Notify>,
  pub stats:         Option<Arc<ProcessorStats>>,
  /// Every mutating operation takes this lock, gets the next sequence number and is enqueued into
  /// all of its target subgraphs before the lock is released, so every subgraph sees operations
  /// in one global order. Reads never take it.
  dispatcher:        tokio::sync::Mutex<DispatchState>,
  /// Serializes whole replacements of the state (bulk load, reset) with each other.
  lifecycle:         tokio::sync::Mutex<()>,
  /// Reads that may sample frames at once (MERITRANK_SAMPLING_CONCURRENCY).
  sampling:          Arc<tokio::sync::Semaphore>,
  /// Bytes of samples dispatched for admission and not yet published.
  admit_inflight:    Arc<AtomicUsize>,
}

#[derive(Default)]
struct DispatchState {
  last_seq:         u64,
  /// Sequence number of the last operation enqueued into each subgraph: waiting for a subgraph's
  /// watermark to reach it means waiting until everything sent to it is published.
  last_by_subgraph: HashMap<SubgraphName, u64>,
}

/// Result of a dispatch: its sequence number and, per target subgraph, its identity (the
/// published-copy slot) and watermark.
struct Dispatched {
  ok:      bool,
  seq:     u64,
  watches: Vec<(SubgraphName, Arc<ArcSwap<RwLock<AugGraph>>>, watch::Receiver<u64>)>,
}

impl Dispatched {
  fn response(&self) -> Response {
    if self.ok {
      Response::Ok
    } else {
      Response::Fail
    }
  }

}

/// Where a dispatched operation goes.
enum Targets<'a> {
  One(&'a SubgraphName),
  /// Every existing subgraph, after creating `ensure` if it is absent.
  All { ensure: &'a SubgraphName },
  /// This subgraph if it exists (never creates one).
  Existing(&'a SubgraphName),
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
  recorded: Option<Arc<std::sync::Mutex<Vec<(u64, AugGraphOp)>>>>,
) {
  let mut front_arc = copy_a;
  let mut back_arc = copy_b;
  let max_batch = max_batch.max(1);
  shared.store(Arc::clone(&front_arc));

  let apply = |graph: &mut AugGraph, op: &SeqOp, record_stats: bool| {
    let start = Instant::now();
    if record_stats {
      if let Some(r) = &recorded {
        r.lock().unwrap().push((op.seq, (*op.op).clone()));
      }
    }
    // A panic must not kill the subgraph (its queue would never drain again and every later
    // sync would fail). Both copies replay the same operation with the same stream, so they
    // stay alike even then.
    let applied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      graph.apply_seq_op(op.seq, &op.op);
    }));
    if applied.is_err() {
      log_error!("Operation {} panicked while being applied: {:?}", op.seq, op.op);
    }
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

/// Rejects an edge write that must not reach a graph: empty names, self-edges and non-finite
/// weights. Walls are valid between any two nodes, in any context (D14: one node class, isolated
/// contexts).
fn validate_edge_write(
  src: &NodeName,
  dst: &NodeName,
  amount: Weight,
) -> Result<(), String> {
  if src.is_empty() || dst.is_empty() {
    return Err(format!("Empty node name: {:?} -> {:?}", src, dst));
  }
  if src == dst {
    return Err(format!("Self-reference is not allowed: {}", src));
  }
  if !amount.is_finite() {
    return Err(format!("Edge weight must be finite: {} -> {} = {}", src, dst, amount));
  }
  Ok(())
}

/// The peer of every row: a score's target, a graph edge's destination.
fn row_peer_keys(response: &Response) -> Vec<String> {
  match response {
    Response::Scores(ResScores { scores }) => scores.iter().map(|r| r.target.clone()).collect(),
    Response::Graph(ResGraph { graph }) => graph.iter().map(|r| r.dst.clone()).collect(),
    _ => vec![],
  }
}

/// Replaces, in `base`, every row whose peer (score target, graph edge destination) is in
/// `peers` by the same row of `portion`, keeping `base`'s order. A peer may have several rows
/// (e.g. a neighbour both inbound and outbound): they are matched in order.
fn merge_rows(
  base: Response,
  portion: Response,
  peers: &HashSet<String>,
) -> Response {
  use std::collections::VecDeque;
  match (base, portion) {
    (Response::Scores(ResScores { scores }), Response::Scores(ResScores { scores: part })) => {
      let mut queue: HashMap<String, VecDeque<ScoreResult>> = HashMap::new();
      for r in part_rows(part, peers, |r: &ScoreResult| &r.target) {
        queue.entry(r.target.clone()).or_default().push_back(r);
      }
      Response::Scores(ResScores {
        scores: scores
          .into_iter()
          .map(|r| queue.get_mut(&r.target).and_then(|q| q.pop_front()).unwrap_or(r))
          .collect(),
      })
    },
    (Response::Graph(ResGraph { graph }), Response::Graph(ResGraph { graph: part })) => {
      let mut queue: HashMap<(String, String), VecDeque<GraphResult>> = HashMap::new();
      for r in part_rows(part, peers, |r: &GraphResult| &r.dst) {
        queue.entry((r.src.clone(), r.dst.clone())).or_default().push_back(r);
      }
      Response::Graph(ResGraph {
        graph: graph
          .into_iter()
          .map(|r| {
            queue.get_mut(&(r.src.clone(), r.dst.clone())).and_then(|q| q.pop_front()).unwrap_or(r)
          })
          .collect(),
      })
    },
    (base, _) => base,
  }
}

fn part_rows<R>(
  rows: Vec<R>,
  peers: &HashSet<String>,
  peer: impl Fn(&R) -> &String,
) -> Vec<R> {
  rows.into_iter().filter(|r| peers.contains(peer(r))).collect()
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
    let mut initial = initial;
    initial.stream = stream_key(name);
    let recorded = initial
      .settings
      .record_ops
      .then(|| Arc::new(std::sync::Mutex::new(Vec::new())));
    let copy_a = Arc::new(RwLock::new(initial.clone()));
    let copy_b = Arc::new(RwLock::new(initial));
    let copies = [Arc::clone(&copy_a), Arc::clone(&copy_b)];
    let shared = Arc::new(ArcSwap::new(Arc::clone(&copy_a)));

    let (tx, rx) = mpsc::channel(queue_len.max(1));
    let op_sender = OpSender {
      tx,
      local_seq: Arc::new(tokio::sync::Mutex::new(0)),
    };
    let (published_tx, published_seq) = watch::channel(0);

    let residency = Residency::new(walks_cache_size);

    let shared_clone = Arc::clone(&shared);
    let notify_clone = Arc::clone(&publish_notify);
    let recorded_clone = recorded.clone();
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
        recorded_clone,
      );
    });

    ConcurrentDataProcessor {
      processing_thread: loop_thread,
      op_sender,
      shared,
      published_seq,
      residency,
      read_stats: Arc::new(ReadStats::default()),
      copies,
      recorded,
    }
  }

  /// Both buffer copies (test support: replica checks).
  pub fn copies(&self) -> [Arc<RwLock<AugGraph>>; 2] {
    [Arc::clone(&self.copies[0]), Arc::clone(&self.copies[1])]
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

/// Snapshot and reverse-score counters of a subgraph (D14).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapshotStats {
  /// Deterministic counters, from the published copy.
  pub graph:                 GraphCounters,
  pub snapshot_count:        usize,
  pub snapshot_bytes:        usize,
  pub snapshot_quota:        usize,
  /// Reverse scores taken by reads from resident frames, snapshots, and fresh samples.
  pub reverse_from_frame:    u64,
  pub reverse_from_snapshot: u64,
  pub reverse_sampled:       u64,
  pub sample_walks:          u64,
  /// Samples not offered for admission (the queue budget was exhausted).
  pub admissions_skipped:    u64,
}

impl MultiGraphProcessor {
  /// Snapshot counters of a subgraph.
  pub fn snapshot_stats(
    &self,
    subgraph: &str,
  ) -> Option<SnapshotStats> {
    let p = self.subgraphs_map.get(subgraph)?;
    let (graph, snapshot_count, snapshot_bytes, snapshot_quota) = p.read(|g| {
      (g.counters.clone(), g.snapshots.len(), g.snapshots.bytes(), g.snapshots.quota())
    });
    let r = &p.read_stats;
    Some(SnapshotStats {
      graph,
      snapshot_count,
      snapshot_bytes,
      snapshot_quota,
      reverse_from_frame: r.reverse_from_frame.load(Ordering::Relaxed),
      reverse_from_snapshot: r.reverse_from_snapshot.load(Ordering::Relaxed),
      reverse_sampled: r.reverse_sampled.load(Ordering::Relaxed),
      sample_walks: r.sample_walks.load(Ordering::Relaxed),
      admissions_skipped: r.admissions_skipped.load(Ordering::Relaxed),
    })
  }

  /// Runs `read` on a subgraph's published copy.
  pub fn read_subgraph<T>(
    &self,
    subgraph: &str,
    read: impl FnOnce(&AugGraph) -> T,
  ) -> Option<T> {
    let shared = Arc::clone(&self.subgraphs_map.get(subgraph)?.shared);
    Some(read_published(&shared, read))
  }

  /// With `Settings::record_ops`: every operation the subgraph's worker applied, in order.
  pub fn recorded_ops(
    &self,
    subgraph: &str,
  ) -> Vec<(u64, AugGraphOp)> {
    self
      .subgraphs_map
      .get(subgraph)
      .and_then(|p| p.recorded.as_ref().map(|r| r.lock().unwrap().clone()))
      .unwrap_or_default()
  }

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
    let settings_concurrency = settings.sampling_concurrency.max(1);
    let mgp = MultiGraphProcessor {
      subgraphs_map:   DashMap::new(),
      settings,
      loading:         AtomicBool::new(false),
      publish_notify:  Arc::new(tokio::sync::Notify::new()),
      stats,
      dispatcher:      tokio::sync::Mutex::new(DispatchState::default()),
      lifecycle:       tokio::sync::Mutex::new(()),
      sampling:        Arc::new(tokio::sync::Semaphore::new(settings_concurrency)),
      admit_inflight:  Arc::new(AtomicUsize::new(0)),
    };
    mgp.insert_subgraph_if_does_not_exist(&String::new());
    mgp
  }

  /// Numbers `op` and enqueues it into every target subgraph, under the dispatcher lock.
  async fn dispatch(
    &self,
    targets: Targets<'_>,
    op: AugGraphOp,
  ) -> Response {
    let mut state = self.dispatcher.lock().await;
    self.dispatch_locked(&mut state, targets, op).await.response()
  }

  /// Waits until every target subgraph has published the dispatched operation. A subgraph that
  /// was retired meanwhile (reset, bulk load) no longer matters; one whose worker died while it is
  /// still the current subgraph is a failure (the operation will never be published).
  async fn wait_dispatched(
    &self,
    dispatched: Dispatched,
  ) -> bool {
    let mut ok = dispatched.ok;
    for (name, shared, mut watch) in dispatched.watches {
      if watch.wait_for(|v| *v >= dispatched.seq).await.is_err() {
        let current = self
          .subgraphs_map
          .get(&name)
          .map_or(false, |p| Arc::ptr_eq(&p.shared, &shared));
        if current {
          log_error!("Subgraph {:?} stopped before publishing operation {}", name, dispatched.seq);
          ok = false;
        }
      }
    }
    ok
  }

  /// `dispatch` for a caller that already holds the dispatcher lock.
  async fn dispatch_locked(
    &self,
    state: &mut DispatchState,
    targets: Targets<'_>,
    op: AugGraphOp,
  ) -> Dispatched {
    let op = Arc::new(op);

    let refused = Dispatched {
      ok:      false,
      seq:     state.last_seq,
      watches: vec![],
    };
    let names: Vec<SubgraphName> = match targets {
      Targets::One(name) => {
        if !self.create_subgraph_locked(state, name).await {
          return refused;
        }
        vec![name.clone()]
      },
      Targets::All { ensure } => {
        if !self.create_subgraph_locked(state, ensure).await {
          return refused;
        }
        let mut names: Vec<SubgraphName> =
          self.subgraphs_map.iter().map(|r| r.key().clone()).collect();
        names.sort();
        names
      },
      Targets::Existing(name) => {
        if !self.subgraphs_map.contains_key(name) {
          return refused;
        }
        vec![name.clone()]
      },
    };
    // Clone senders and watermarks out of the map: no map guard may be held across an await.
    let targets: Vec<(
      SubgraphName,
      OpSender,
      Arc<ArcSwap<RwLock<AugGraph>>>,
      watch::Receiver<u64>,
    )> = names
      .into_iter()
      .filter_map(|n| {
        let p = self.subgraphs_map.get(&n)?;
        let t = (p.op_sender.clone(), Arc::clone(&p.shared), p.published_seq.clone());
        Some((n, t.0, t.1, t.2))
      })
      .collect();

    state.last_seq += 1;
    let seq = state.last_seq;
    let mut ok = true;
    let mut watches = vec![];
    for (name, sender, shared, watch) in targets {
      if let Some(s) = &self.stats {
        s.record_enqueue();
      }
      if sender.send_seq(seq, Arc::clone(&op)).await.is_err() {
        ok = false;
      }
      state.last_by_subgraph.insert(name.clone(), seq);
      watches.push((name, shared, watch));
    }
    Dispatched { ok, seq, watches }
  }

  /// Creates a subgraph if it is absent, under the dispatcher lock. A new context starts empty
  /// (D14: contexts are isolated). Returns false, creating nothing, when the subgraph is absent
  /// and the number of contexts has reached MERITRANK_MAX_CONTEXTS (every subgraph costs a thread
  /// and two graph copies).
  async fn create_subgraph_locked(
    &self,
    state: &mut DispatchState,
    name: &SubgraphName,
  ) -> bool {
    if self.subgraphs_map.contains_key(name) {
      return true;
    }
    if !name.is_empty() && self.subgraphs_map.len() > self.settings.max_contexts {
      log_error!(
        "Context {:?} not created: MERITRANK_MAX_CONTEXTS ({}) reached",
        name,
        self.settings.max_contexts
      );
      return false;
    }
    self.insert_subgraph_if_does_not_exist(name);
    self.rebalance_snapshot_quotas_locked(state).await;
    true
  }

  /// Splits the process-wide snapshot budget evenly over the subgraphs (both copies of each): a
  /// sequenced operation, so both copies of a subgraph evict the same snapshots.
  async fn rebalance_snapshot_quotas_locked(
    &self,
    state: &mut DispatchState,
  ) {
    if !self.settings.snapshots_enabled() {
      return;
    }
    let quota = self.settings.snapshot_bytes_per_copy(self.subgraphs_map.len());
    let mut names: Vec<SubgraphName> = self.subgraphs_map.iter().map(|r| r.key().clone()).collect();
    names.sort();
    for name in names {
      let sender = match self.subgraphs_map.get(&name) {
        Some(p) => p.op_sender.clone(),
        None => continue,
      };
      state.last_seq += 1;
      let seq = state.last_seq;
      if sender.send_seq(seq, Arc::new(AugGraphOp::SetSnapshotQuota(quota))).await.is_ok() {
        state.last_by_subgraph.insert(name, seq);
      }
    }
  }

  /// Clears every subgraph and recreates the null context, under the dispatcher lock.
  async fn clear_subgraphs(&self) {
    let mut state = self.dispatcher.lock().await;
    self.subgraphs_map.clear();
    state.last_by_subgraph.clear();
    self.insert_subgraph_if_does_not_exist(&String::new());
    self.rebalance_snapshot_quotas_locked(&mut state).await;
  }

  async fn send_op(
    &self,
    subgraph_name: &SubgraphName,
    op: AugGraphOp,
  ) -> Response {
    log_trace!();
    self.dispatch(Targets::One(subgraph_name), op).await
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

  /// Sync barrier: every operation dispatched before the call is published in every subgraph
  /// when it returns. The caller's stamp is not needed (kept in the protocol for compatibility).
  pub async fn sync_future(&self) -> bool {
    log_trace!();
    let dispatched = {
      let mut state = self.dispatcher.lock().await;
      self
        .dispatch_locked(
          &mut state,
          Targets::All {
            ensure: &String::new(),
          },
          AugGraphOp::Barrier,
        )
        .await
    };
    self.wait_dispatched(dispatched).await
  }

  /// Makes the egos' frames resident in the subgraph and, with `pin`, keeps them so until the
  /// returned lease is dropped. Absent frames are calculated; evictions to stay within capacity
  /// are decided and dispatched under the dispatcher lock, so they are ordered with every other
  /// calculation. Waits until the frames are published. Non-user and unknown ids are ignored.
  ///
  /// Robustness: node ids are resolved against one subgraph and re-checked under the lock (a
  /// reset or bulk load may have replaced it: then start over); pins are owned by a lease from the
  /// moment they are taken, so a cancelled caller releases them; and before planning, frames the
  /// cache does not know about (from an interrupted plan) are adopted so they can be evicted.
  async fn acquire(
    &self,
    subgraph: &SubgraphName,
    egos: &[NodeId],
    pin: bool,
  ) -> Option<Lease> {
    for _attempt in 0..3 {
      let (residency, mut watch, shared) = match self.subgraphs_map.get(subgraph) {
        Some(p) => (
          Arc::clone(&p.residency),
          p.published_seq.clone(),
          Arc::clone(&p.shared),
        ),
        None => return None,
      };
      let mut egos: Vec<NodeId> = read_published(&shared, |g| {
        egos
          .iter()
          .copied()
          .filter(|id| g.nodes.get_by_id(*id).is_some())
          .collect()
      });
      egos.sort_unstable();
      egos.dedup();
      if egos.is_empty() {
        return None;
      }

      if pin {
        if let Some(ready) = residency.try_pin_resident(&egos) {
          let lease = Lease::new(residency, egos);
          let _ = watch.wait_for(|v| *v >= ready).await;
          return Some(lease);
        }
      }

      let mut state = self.dispatcher.lock().await;
      let same = self
        .subgraphs_map
        .get(subgraph)
        .map_or(false, |p| Arc::ptr_eq(&p.residency, &residency));
      if !same {
        continue; // replaced meanwhile: the ids belong to the old graph
      }
      if residency.capacity() > 0 {
        let calculated = read_published(&shared, |g| g.mr.calculated_egos());
        residency.adopt(&calculated);
      }
      let plan = residency.plan(&egos, pin);
      let lease = pin.then(|| Lease::new(Arc::clone(&residency), egos.clone()));
      for id in &plan.evict {
        self
          .dispatch_locked(&mut state, Targets::One(subgraph), AugGraphOp::ClearEgo(*id))
          .await;
      }
      let mut wait_for = plan.ready_seq;
      if !plan.calculate.is_empty() {
        let d = self
          .dispatch_locked(
            &mut state,
            Targets::One(subgraph),
            AugGraphOp::EnsureCalculated(plan.calculate.clone()),
          )
          .await;
        residency.set_ready(&plan.calculate, d.seq);
        wait_for = wait_for.max(d.seq);
      }
      drop(state);
      let _ = watch.wait_for(|v| *v >= wait_for).await;
      return lease;
    }
    None
  }

  /// An explicit (re)calculation: the `WriteCalculate` itself is the calculation. The ego is
  /// registered as resident (evicting others if needed) in the same dispatcher section, and the
  /// call returns once the operation is queued, like any write.
  async fn explicit_calculate(
    &self,
    subgraph: &SubgraphName,
    ego: &NodeName,
  ) -> Response {
    let op = AugGraphOp::WriteCalculate(OpWriteCalculate { ego: ego.clone() });
    let known = self.subgraphs_map.get(subgraph).and_then(|p| {
      let id = read_published(&p.shared, |g| {
        g.nodes.get_by_name(ego).map(|i| i.id)
      })?;
      Some((Arc::clone(&p.residency), id))
    });
    let (residency, id) = match known {
      // Unknown ego: the operation registers the node; once that is published, register the
      // frame as resident too (evicting others if needed), or it would never be evicted.
      None => {
        let dispatched = {
          let mut state = self.dispatcher.lock().await;
          self.dispatch_locked(&mut state, Targets::One(subgraph), op).await
        };
        let response = dispatched.response();
        self.wait_dispatched(dispatched).await;
        let id = self.subgraphs_map.get(subgraph).and_then(|p| {
          read_published(&p.shared, |g| g.nodes.get_by_name(ego).map(|i| i.id))
        });
        if let Some(id) = id {
          self.acquire(subgraph, &[id], false).await;
        }
        return response;
      },
      Some(x) => x,
    };
    let mut state = self.dispatcher.lock().await;
    let same = self
      .subgraphs_map
      .get(subgraph)
      .map_or(false, |p| Arc::ptr_eq(&p.residency, &residency));
    if !same {
      // Replaced by a reset or bulk load since the lookup: the id belongs to the old graph.
      return self.dispatch_locked(&mut state, Targets::One(subgraph), op).await.response();
    }
    let plan = residency.plan(&[id], false);
    for victim in &plan.evict {
      self
        .dispatch_locked(&mut state, Targets::One(subgraph), AugGraphOp::ClearEgo(*victim))
        .await;
    }
    let dispatched = self.dispatch_locked(&mut state, Targets::One(subgraph), op).await;
    residency.set_ready(&plan.calculate, dispatched.seq);
    dispatched.response()
  }

  /// A read in an ego's frame (D14). The ego's frame is pinned; the read runs under one read
  /// guard of the published copy, on a blocking thread, and takes every reverse score from the
  /// peer's resident frame, its snapshot, or a fresh sample of it. Samples are then offered for
  /// admission. Without snapshots (or with an unbounded walk cache) peers' frames are pinned
  /// instead (`ego_read_frames`). Returns, with the response, the (epoch, applied_seq) of the
  /// copy it was read from.
  async fn ego_read(
    &self,
    subgraph: &SubgraphName,
    ego: &NodeName,
    run: ReadFn,
  ) -> (Response, Option<(u64, u64)>) {
    if !self.settings.snapshots_enabled() {
      return (self.ego_read_frames(subgraph, ego, run).await, None);
    }
    for _attempt in 0..3 {
      let (shared, read_stats, published) = match self.subgraphs_map.get(subgraph) {
        Some(p) => (Arc::clone(&p.shared), Arc::clone(&p.read_stats), p.published_seq.clone()),
        None => return (Response::Fail, None),
      };
      let ego_id = read_published(&shared, |g| g.nodes.get_by_name(ego).map(|i| i.id));
      let lease = match ego_id {
        Some(id) => self.acquire(subgraph, &[id], true).await,
        None => None,
      };
      // A reset or bulk load may have replaced the subgraph meanwhile: start over by name.
      let same = self
        .subgraphs_map
        .get(subgraph)
        .map_or(false, |p| Arc::ptr_eq(&p.shared, &shared));
      if !same {
        drop(lease);
        continue;
      }
      // Take a sampling slot before the graph guard: never wait for one while holding it.
      let permit = match Arc::clone(&self.sampling).acquire_owned().await {
        Ok(p) => p,
        Err(_) => return (Response::Fail, None),
      };
      let read = {
        let shared = Arc::clone(&shared);
        let run = Arc::clone(&run);
        tokio::task::spawn_blocking(move || {
          read_published(&shared, |g| {
            let (response, report) = read_scope(true, || run(g));
            (response, report, g.epoch, g.applied_seq, g.zero_revision, ego_id)
          })
        })
        .await
      };
      drop(permit);
      drop(lease);
      let (response, report, epoch, applied_seq, zero_revision, ego_id) = match read {
        Ok(x) => x,
        Err(e) => {
          log_error!("Read failed: {}", e);
          return (Response::Fail, None);
        },
      };

      let peers_from_frames =
        report.frames.iter().filter(|f| Some(**f) != ego_id).count() as u64;
      read_stats.reverse_from_frame.fetch_add(peers_from_frames, Ordering::Relaxed);
      read_stats.reverse_from_snapshot.fetch_add(report.from_snapshot, Ordering::Relaxed);
      read_stats.reverse_sampled.fetch_add(report.sampled.len() as u64, Ordering::Relaxed);
      read_stats.sample_walks.fetch_add(report.sample_walks, Ordering::Relaxed);
      self
        .offer_samples(
          subgraph,
          report.sampled,
          report.bounds,
          (epoch, applied_seq, zero_revision),
          &read_stats,
          published,
        )
        .await;
      return (response, Some((epoch, applied_seq)));
    }
    (Response::Fail, None)
  }

  /// Dispatches a read's samples for admission, within the admission byte budget.
  async fn offer_samples(
    &self,
    subgraph: &SubgraphName,
    sampled: Vec<Arc<meritrank_core::FrameSample>>,
    bounds: Vec<Option<Vec<f64>>>,
    (epoch, base_seq, zero_revision): (u64, u64, u64),
    read_stats: &ReadStats,
    mut published: watch::Receiver<u64>,
  ) {
    if sampled.is_empty() {
      return;
    }
    // Offer as many as the budget holds (in read order); the rest are sampled again by a later
    // read and offered then.
    let budget = self.settings.admit_queue_bytes();
    let mut bytes = 0usize;
    let mut samples: Vec<meritrank_core::FrameSample> = vec![];
    let mut kept_bounds: Vec<Option<Vec<f64>>> = vec![];
    let total = sampled.len();
    for (s, b) in sampled.into_iter().zip(bounds) {
      let size = sample_bytes(&s);
      let before = self.admit_inflight.fetch_add(size, Ordering::SeqCst);
      if before + size > budget {
        self.admit_inflight.fetch_sub(size, Ordering::SeqCst);
        break;
      }
      bytes += size;
      samples.push(Arc::try_unwrap(s).unwrap_or_else(|a| (*a).clone()));
      kept_bounds.push(b);
    }
    let skipped = (total - samples.len()) as u64;
    if skipped > 0 {
      read_stats.admissions_skipped.fetch_add(skipped, Ordering::Relaxed);
    }
    if samples.is_empty() {
      return;
    }
    let op = AugGraphOp::AdmitSnapshots(AdmitBatch {
      epoch,
      base_seq,
      samples: Arc::new(samples),
      bounds: Arc::new(kept_bounds),
      zero_revision,
    });
    let dispatched = {
      let mut state = self.dispatcher.lock().await;
      self.dispatch_locked(&mut state, Targets::Existing(subgraph), op).await
    };
    let inflight = Arc::clone(&self.admit_inflight);
    if !dispatched.ok {
      inflight.fetch_sub(bytes, Ordering::SeqCst);
      return;
    }
    let seq = dispatched.seq;
    // The budget is released once the admission is published (or the subgraph is gone).
    tokio::spawn(async move {
      let _ = published.wait_for(|v| *v >= seq).await;
      inflight.fetch_sub(bytes, Ordering::SeqCst);
    });
  }

  /// A read in an ego's frame, in two phases: pin the ego and read, recording which other frames
  /// the read needed (reverse scores); pin those, calculating absent ones, and read again. When
  /// the peers do not fit the capacity, they are pinned in portions and each row (one per peer:
  /// the target of a score, the destination of a graph edge) is taken from its portion's read.
  async fn ego_read_frames(
    &self,
    subgraph: &SubgraphName,
    ego: &NodeName,
    run: ReadFn,
  ) -> Response {
    let run = |g: &AugGraph| run(g);
    let (residency, shared) = match self.subgraphs_map.get(subgraph) {
      Some(p) => (Arc::clone(&p.residency), Arc::clone(&p.shared)),
      None => return Response::Fail,
    };
    let ego_id = match read_published(&shared, |g| g.nodes.get_by_name(ego).map(|i| i.id)) {
      Some(id) => id,
      None => return read_published(&shared, &run),
    };
    let _ego = self.acquire(subgraph, &[ego_id], true).await;

    let (first, frames) = record_frames(|| read_published(&shared, &run));
    let peers: Vec<NodeId> = frames.into_iter().filter(|id| *id != ego_id).collect();
    if peers.is_empty() {
      return first;
    }
    let capacity = residency.capacity();
    if capacity == 0 || peers.len() <= capacity {
      // The graph may change between the passes; if the final read needs frames the first did
      // not (a different page, a new peer), pin those too and read again.
      let mut acquired: BTreeSet<NodeId> = peers.iter().copied().collect();
      let mut leases = vec![self.acquire(subgraph, &peers, true).await];
      for _attempt in 0..3 {
        let (response, frames) = record_frames(|| read_published(&shared, &run));
        let missing: Vec<NodeId> = frames
          .into_iter()
          .filter(|id| *id != ego_id && !acquired.contains(id))
          .collect();
        if missing.is_empty() {
          return response;
        }
        acquired.extend(missing.iter().copied());
        leases.push(self.acquire(subgraph, &missing, true).await);
      }
      return read_published(&shared, &run);
    }

    // The working set of one read exceeds the walk cache: frames will be recalculated on every
    // such read. Logged sparsely (1st, 2nd, 4th, 8th … time).
    let seen = OVER_CAPACITY.fetch_add(1, Ordering::Relaxed) + 1;
    if seen.is_power_of_two() {
      log_warning!(
        "A read needs {} peer frames but MERITRANK_WALKS_CACHE_SIZE is {}: frames are recalculated \
         on every such read ({} so far); raise the cache size to cover the working set.",
        peers.len(),
        capacity,
        seen
      );
    }
    let keys = row_peer_keys(&first);
    let mut merged = first;
    for portion in peers.chunks(capacity) {
      let _lease = self.acquire(subgraph, portion, true).await;
      // A row belongs to this portion when the frame its reverse score needs — its target's —
      // is in the portion.
      let members: HashSet<NodeId> = portion.iter().copied().collect();
      let names: HashSet<String> = read_published(&shared, |g| {
        keys
          .iter()
          .filter(|k| g.nodes.get_by_name(k).map_or(false, |i| members.contains(&i.id)))
          .cloned()
          .collect()
      });
      let portion_read = read_published(&shared, &run);
      merged = merge_rows(merged, portion_read, &names);
    }
    merged
  }

  pub async fn process_request(
    &self,
    req: &Request,
  ) -> Response {
    self.process_request_traced(req).await.0
  }

  /// `process_request`, also returning, for an ego read, the (epoch, applied_seq) of the copy the
  /// response was built from.
  pub async fn process_request_traced(
    &self,
    req: &Request,
  ) -> (Response, Option<(u64, u64)>) {
    if self.loading.load(Ordering::SeqCst) && !matches!(&req.data, ReqData::WriteBulkEdges(_)) {
      return (Response::Fail, None);
    }
    let ego_read = |run: ReadFn| async move {
      let ego = req.data.read_ego().cloned().unwrap_or_default();
      self.ego_read(&req.subgraph, &ego, run).await
    };
    match &req.data {
      ReqData::ReadScores(data) => {
        let data = data.clone();
        return ego_read(Arc::new(move |g: &AugGraph| {
          Response::Scores(ResScores { scores: g.read_scores(data.clone()) })
        }))
        .await;
      },
      ReqData::ReadNodeScore(data) => {
        let data = data.clone();
        return ego_read(Arc::new(move |g: &AugGraph| {
          Response::Scores(ResScores { scores: g.read_node_score(data.clone()) })
        }))
        .await;
      },
      ReqData::ReadGraph(data) => {
        let data = data.clone();
        return ego_read(Arc::new(move |g: &AugGraph| {
          Response::Graph(ResGraph { graph: g.read_graph(data.clone()) })
        }))
        .await;
      },
      ReqData::ReadNeighbors(data) => {
        let data = data.clone();
        return ego_read(Arc::new(move |g: &AugGraph| {
          Response::Scores(ResScores { scores: g.read_neighbors(data.clone()) })
        }))
        .await;
      },
      ReqData::ReadMutualScores(data) => {
        let data = data.clone();
        return ego_read(Arc::new(move |g: &AugGraph| {
          Response::Scores(ResScores { scores: g.read_mutual_scores(data.clone()) })
        }))
        .await;
      },
      _ => {},
    }
    (self.process_other(req).await, None)
  }

  async fn process_other(
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
        // Validate the whole batch before touching anything (R20).
        let contexts: BTreeSet<&SubgraphName> =
          data.edges.iter().map(|e| &e.context).filter(|c| !c.is_empty()).collect();
        if contexts.len() > self.settings.max_contexts {
          log_error!(
            "Bulk load rejected: {} contexts exceed MERITRANK_MAX_CONTEXTS ({})",
            contexts.len(),
            self.settings.max_contexts
          );
          return Response::Fail;
        }
        for edge in &data.edges {
          if let Err(e) = validate_edge_write(&edge.src, &edge.dst, edge.amount) {
            log_error!("Bulk load rejected: {}", e);
            return Response::Fail;
          }
        }
        let _lifecycle = self.lifecycle.lock().await;
        self.loading.store(true, Ordering::SeqCst);

        // Every edge goes to its own context only (D14: isolated contexts), in input order.
        // Ordered collections: the load must not depend on hash iteration order.
        let mut by_context: BTreeMap<SubgraphName, Vec<OpWriteEdge>> = BTreeMap::new();
        by_context.insert(String::new(), vec![]);
        for edge in data.edges {
          by_context.entry(edge.context).or_default().push(OpWriteEdge {
            src:       edge.src,
            dst:       edge.dst,
            amount:    edge.amount,
            magnitude: edge.magnitude,
          });
        }

        // Replace the state and enqueue the load in one dispatcher section: no concurrent write
        // can land between the clearing and the load.
        {
          let mut state = self.dispatcher.lock().await;
          self.subgraphs_map.clear();
          state.last_by_subgraph.clear();
          for ctx in by_context.keys() {
            self.insert_subgraph_if_does_not_exist(ctx);
          }
          self.rebalance_snapshot_quotas_locked(&mut state).await;
          for (ctx, edges) in by_context {
            self
              .dispatch_locked(&mut state, Targets::One(&ctx), AugGraphOp::BulkLoadEdges(edges))
              .await;
          }
        }

        let synced = self.sync_future().await;

        self.loading.store(false, Ordering::SeqCst);
        if synced {
          Response::Ok
        } else {
          Response::Fail
        }
      },
      ReqData::WriteCalculate(data) => {
        self.explicit_calculate(&req.subgraph, &data.ego).await
      },
      ReqData::WriteCreateContext => {
        let mut state = self.dispatcher.lock().await;
        if self.create_subgraph_locked(&mut state, &req.subgraph).await {
          Response::Ok
        } else {
          Response::Fail
        }
      },
      ReqData::WriteDeleteEdge(data) => {
        self
          .process_write_edge(
            &req.subgraph,
            &OpWriteEdge {
              src:       data.src,
              dst:       data.dst,
              amount:    0.0,
              // A deletion needs no magnitude (and `index` defaults to -1).
              magnitude: 0,
            },
          )
          .await
      },
      ReqData::WriteDeleteNode(data) => {
        self
          .dispatch(Targets::One(&req.subgraph), AugGraphOp::DeleteNode(data.node.clone()))
          .await
      },
      ReqData::WriteZeroOpinion(data) => {
        if !data.score.is_finite() {
          log_error!("Zero opinion must be finite: {} = {}", data.node, data.score);
          return Response::Fail;
        }
        self
          .send_op(&req.subgraph, AugGraphOp::WriteZeroOpinion(data.clone()))
          .await
      },
      ReqData::WriteReset => {
        let _lifecycle = self.lifecycle.lock().await;
        self.clear_subgraphs().await;
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
      ReqData::ReadScores(_)
      | ReqData::ReadNodeScore(_)
      | ReqData::ReadGraph(_)
      | ReqData::ReadNeighbors(_)
      | ReqData::ReadMutualScores(_) => {
        // Handled by `process_request_traced`.
        Response::Fail
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
      ReqData::Sync(_stamp) => {
        if self.sync_future().await {
          Response::Ok
        } else {
          Response::Fail
        }
      },
    }
  }

  /// An edge write reaches only the context it names (D14: isolated contexts).
  async fn process_write_edge(
    &self,
    subgraph_name: &SubgraphName,
    data: &OpWriteEdge,
  ) -> Response {
    log_trace!("{:?} {:?}", subgraph_name, data);

    if let Err(e) = validate_edge_write(&data.src, &data.dst, data.amount) {
      log_error!("{}", e);
      return Response::Fail;
    }
    self.dispatch(Targets::One(subgraph_name), AugGraphOp::WriteEdge(data.clone())).await
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
          AugGraph::with_stream(self.settings.clone(), subgraph_name),
          self.settings.subgraph_queue_capacity,
          self.settings.min_ops_before_swap,
          self.publish_notify.clone(),
          self.stats.clone(),
          self.settings.walks_cache_size,
        )
      });
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

  async fn write_in(proc: &MultiGraphProcessor, ctx: &str, src: &str, dst: &str, amount: Weight) {
    let _ = proc.process_request(&Request {
      subgraph: ctx.into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       src.into(),
        dst:       dst.into(),
        amount,
        magnitude: 0,
      }),
    }).await;
  }

  async fn edges_in(proc: &MultiGraphProcessor, ctx: &str) -> Vec<(String, String, Weight)> {
    let mut edges = edges_from_response(proc.process_request(&Request {
      subgraph: ctx.into(),
      data:     ReqData::ReadEdges,
    }).await);
    edges.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    edges
  }

  /// Contexts are isolated (D14): a write reaches only the context it names; nothing is
  /// aggregated into the null context.
  #[tokio::test]
  async fn contexts_are_isolated_null_gets_nothing() {
    let proc = default_processor();
    write_in(&proc, "X", "B1", "U2", 1.0).await;
    write_in(&proc, "Y", "B1", "U2", 2.0).await;
    sync(&proc).await;
    assert!(edges_in(&proc, "").await.is_empty());
    assert_eq!(edges_in(&proc, "X").await, vec![("B1".into(), "U2".into(), 1.0)]);
    assert_eq!(edges_in(&proc, "Y").await, vec![("B1".into(), "U2".into(), 2.0)]);
  }

  #[tokio::test]
  async fn user_edges_stay_in_their_context() {
    let proc = default_processor();
    write_in(&proc, "X", "U1", "U2", 1.0).await;
    write_in(&proc, "Y", "U1", "U3", 2.0).await;
    write_in(&proc, "", "U4", "U5", 3.0).await;
    sync(&proc).await;
    assert_eq!(edges_in(&proc, "").await, vec![("U4".into(), "U5".into(), 3.0)]);
    assert_eq!(edges_in(&proc, "X").await, vec![("U1".into(), "U2".into(), 1.0)]);
    assert_eq!(edges_in(&proc, "Y").await, vec![("U1".into(), "U3".into(), 2.0)]);
  }

  #[tokio::test]
  async fn delete_reaches_only_its_context() {
    let proc = default_processor();
    write_in(&proc, "X", "B1", "U2", 1.0).await;
    write_in(&proc, "Y", "B1", "U2", 2.0).await;
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteDeleteEdge(OpWriteDeleteEdge {
        src:   "B1".into(),
        dst:   "U2".into(),
        index: -1,
      }),
    }).await;
    sync(&proc).await;
    assert!(edges_in(&proc, "X").await.is_empty());
    assert_eq!(edges_in(&proc, "Y").await, vec![("B1".into(), "U2".into(), 2.0)]);
    assert!(edges_in(&proc, "").await.is_empty());
  }

  #[tokio::test]
  async fn delete_node_reaches_only_its_context() {
    let proc = default_processor();
    write_in(&proc, "X", "U1", "U2", 1.0).await;
    write_in(&proc, "", "U1", "U2", 1.0).await;
    let _ = proc.process_request(&Request {
      subgraph: "X".into(),
      data:     ReqData::WriteDeleteNode(OpWriteDeleteNode {
        node:  "U1".into(),
        index: -1,
      }),
    }).await;
    sync(&proc).await;
    assert!(edges_in(&proc, "X").await.is_empty());
    assert_eq!(edges_in(&proc, "").await, vec![("U1".into(), "U2".into(), 1.0)]);
  }

  /// A new context starts empty: it inherits nothing from the null context.
  #[tokio::test]
  async fn new_context_starts_empty() {
    let proc = default_processor();
    write_in(&proc, "", "U1", "U2", 1.0).await;
    write_in(&proc, "X", "U1", "U3", 2.0).await;
    sync(&proc).await;
    let _ = proc.process_request(&Request {
      subgraph: "Y".into(),
      data:     ReqData::WriteCreateContext,
    }).await;
    sync(&proc).await;
    assert!(edges_in(&proc, "Y").await.is_empty());
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
    assert_eq!(agg_edges.len(), 1, "the null context holds only its own edges");
    let ctx_x = proc
      .process_request(&Request {
        subgraph: "X".into(),
        data:     ReqData::ReadEdges,
      })
      .await;
    let x_edges = edges_from_response(ctx_x);
    assert_eq!(x_edges.len(), 1, "X holds only its own edges");
    assert_eq!(x_edges[0].1, "B1");
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
