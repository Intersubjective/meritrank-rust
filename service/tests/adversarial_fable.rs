//! Adversarial test suite (written by a Fable subagent): tries to BREAK the service with
//! malformed / extreme / malicious requests — crash a worker or the server, hang a request,
//! exhaust memory, corrupt state, or violate the spec (NEGATIVE_EDGES_FEATURE.md R1–R24,
//! SERVICE_CONSISTENCY_PLAN.md).
//!
//! Layout:
//!   * `fixed_*` — defects it found (unbounded frame allocation, out-of-range pagination panic,
//!                 u32 magnitude underflow killing a worker, NaN zero opinion, unbounded context
//!                 creation), now fixed: regression tests.
//!   * `ok_*`    — attacks that did not break anything: regression tests.
//!
//! Every test has a timeout, so a hang is a failure, never a stuck run.
//! Run: `cd service && cargo test --release --test adversarial_fable -- --nocapture`

use meritrank_service::data::*;
use meritrank_service::request_handler::run_server;
use meritrank_service::settings::Settings;
use meritrank_service::state_manager::MultiGraphProcessor;

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const T: Duration = Duration::from_secs(10);

fn fast_settings() -> Settings {
  Settings {
    num_walks: 100,
    zero_opinion_factor: 0.2,
    min_ops_before_swap: 1,
    ..Settings::default()
  }
}

fn processor() -> Arc<MultiGraphProcessor> {
  Arc::new(MultiGraphProcessor::new(fast_settings()))
}

/// A request with a hard timeout: a hang becomes a test failure, not a stuck run.
async fn req(p: &MultiGraphProcessor, subgraph: &str, data: ReqData) -> Response {
  let label = format!("{:?}", data);
  match timeout(T, p.process_request(&Request { subgraph: subgraph.into(), data })).await {
    Ok(r) => r,
    Err(_) => panic!("request HUNG for {:?}: {}", T, &label[..label.len().min(160)]),
  }
}

async fn write_edge(p: &MultiGraphProcessor, ctx: &str, src: &str, dst: &str, amount: f64, magnitude: u32) -> Response {
  req(p, ctx, ReqData::WriteEdge(OpWriteEdge { src: src.into(), dst: dst.into(), amount, magnitude })).await
}

async fn sync(p: &MultiGraphProcessor) -> Response {
  req(p, "", ReqData::Sync(0)).await
}

async fn read_scores(p: &MultiGraphProcessor, ctx: &str, ego: &str, opts: FilterOptions) -> Response {
  req(p, ctx, ReqData::ReadScores(OpReadScores { ego: ego.into(), score_options: opts })).await
}

fn read_edges_vec(r: Response) -> Vec<(String, String, f64)> {
  match r {
    Response::Edges(ResEdges { edges }) => edges.into_iter().map(|e| (e.src, e.dst, e.weight)).collect(),
    other => panic!("expected edges, got {:?}", other),
  }
}

/// Runs `process_request` on a spawned task so a panic in the handler is caught (JoinError)
/// instead of aborting the whole test. Returns Ok(Response) or Err(panic message).
async fn try_request(p: &Arc<MultiGraphProcessor>, r: Request) -> Result<Response, String> {
  let p = Arc::clone(p);
  let label = format!("{:?}", r.data);
  let handle = tokio::spawn(async move { p.process_request(&r).await });
  match timeout(T, handle).await {
    Ok(Ok(resp)) => Ok(resp),
    Ok(Err(join_err)) => Err(format!("handler PANICKED on {}: {}", &label[..label.len().min(120)], join_err)),
    Err(_) => Err(format!("handler HUNG on {}", &label[..label.len().min(120)])),
  }
}

/// This process's virtual memory size (VmSize) in bytes, from /proc/self/statm (field 0 = pages).
fn vm_size_bytes() -> u64 {
  let s = std::fs::read_to_string("/proc/self/statm").expect("read statm");
  let pages: u64 = s.split_whitespace().next().unwrap().parse().unwrap();
  pages * 4096
}

/// Binds an ephemeral port, releases it, returns the number (best-effort; race window is tiny).
fn free_port() -> u16 {
  let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  l.local_addr().unwrap().port()
}

async fn spawn_server(settings: Settings) -> (u16, CancellationToken, tokio::task::JoinHandle<()>) {
  let port = free_port();
  let mut s = settings;
  s.server_port = port;
  let token = CancellationToken::new();
  let token2 = token.clone();
  let proc = Arc::new(MultiGraphProcessor::new(s.clone()));
  let handle = tokio::spawn(async move {
    let _ = run_server(s, proc, token2).await;
  });
  // Wait until the listener is up.
  for _ in 0..200 {
    if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
      break;
    }
    tokio::time::sleep(Duration::from_millis(10)).await;
  }
  (port, token, handle)
}

fn frame(req: &Request) -> Vec<u8> {
  let payload = bincode::encode_to_vec(req, bincode::config::standard()).unwrap();
  let mut buf = (payload.len() as u32).to_be_bytes().to_vec();
  buf.extend_from_slice(&payload);
  buf
}

// ===========================================================================
// CONFIRMED BUGS
// ===========================================================================

/// Was CRITICAL (fixed) — request_handler.rs `read_message` sized `vec![0u8; len]` from the
/// client's 4-byte length prefix with no bound: 4 bytes could request 4 GiB. Frames above
/// MERITRANK_MAX_FRAME_BYTES (default 256 MiB) are now refused before any allocation and the body
/// is read as it arrives. The server must drop such a connection at once, and keep serving.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixed_frame_length_prefix_is_capped() {
  let (port, token, _h) = spawn_server(fast_settings()).await;

  let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
  stream.write_all(&u32::MAX.to_be_bytes()).await.unwrap();
  stream.write_all(&[0u8, 0u8]).await.unwrap();
  stream.flush().await.unwrap();
  let mut byte = [0u8; 1];
  let closed = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut byte)).await;
  assert!(
    matches!(closed, Ok(Ok(0)) | Ok(Err(_))),
    "the server kept an oversized frame open: {:?}",
    closed
  );

  // Still serving.
  let mut ok = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
  ok.write_all(&frame(&Request {
    subgraph: String::new(),
    data:     ReqData::ReadNodeList,
  }))
  .await
  .unwrap();
  let mut len = [0u8; 4];
  tokio::time::timeout(Duration::from_secs(5), ok.read_exact(&mut len))
    .await
    .expect("server stopped serving")
    .unwrap();
  token.cancel();
}

/// HIGH — aug_graph/scores.rs:196-199. `paginate_and_format_items` computes
/// `let start = index as usize; let end = (index + count) as usize;` then slices
/// `items[start..end.min(items.len())]`. When `index` exceeds the number of results, `start > end`
/// and the slice panics ("slice index starts at N but ends at M"). Reachable from `mr_scores`
/// (and `mr_neighbors`) with a large `index`: a single read request panics the connection handler
/// task, which is never turned into `Response::Fail`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixed_read_scores_out_of_range_index() {
  let p = processor();
  write_edge(&p, "", "U1", "U2", 1.0, 0).await;
  sync(&p).await;
  // Warm the frame so there are real (few) results to page past.
  let _ = read_scores(&p, "", "U1", FilterOptions { count: 100, ..Default::default() }).await;

  let opts = FilterOptions { index: 1_000_000, count: 1, ..Default::default() };
  let r = try_request(&p, Request {
    subgraph: "".into(),
    data: ReqData::ReadScores(OpReadScores { ego: "U1".into(), score_options: opts }),
  })
  .await;

  match r {
    Ok(resp) => println!("bug_read_scores_out_of_range_index_panics: got {:?} (fixed)", resp),
    Err(msg) => panic!("{msg}\n-> aug_graph/scores.rs paginate_and_format_items slices items[start..] with start=index unchecked"),
  }
}

/// HIGH — same root cause via `mr_neighbors`: `read_neighbors` -> `apply_filters_and_pagination`
/// -> `paginate_and_format_items`, with the client-supplied `index`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixed_read_neighbors_out_of_range_index() {
  let p = processor();
  write_edge(&p, "", "U1", "U2", 1.0, 0).await;
  write_edge(&p, "", "U1", "U3", 1.0, 0).await;
  sync(&p).await;
  let _ = read_scores(&p, "", "U1", FilterOptions { count: 100, ..Default::default() }).await;

  let r = try_request(&p, Request {
    subgraph: "".into(),
    data: ReqData::ReadNeighbors(OpReadNeighbors {
      ego: "U1".into(),
      focus: "U1".into(),
      direction: NEIGHBORS_OUTBOUND,
      kind: None,
      hide_personal: false,
      lt: 1e9,
      lte: false,
      gt: -1e9,
      gte: false,
      index: 1_000_000,
      count: 1,
    }),
  })
  .await;

  match r {
    Ok(resp) => println!("bug_read_neighbors_out_of_range_index_panics: got {:?} (fixed)", resp),
    Err(msg) => panic!("{msg}"),
  }
}

/// Was MEDIUM (fixed) — vsids.rs subtracted two client-supplied u32 magnitudes; a smaller
/// magnitude after a rescale overflowed, panicking the subgraph worker in debug builds (its queue
/// then never drained). The exponent is now computed in i64. An edge written with a magnitude far
/// older than the node's current scale (1.03^-1195 here) falls below VSIDS's pruning threshold and
/// is dropped by design; what matters is that the worker survives and the service keeps working.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixed_decreasing_magnitude_keeps_worker_alive() {
  let p = processor();
  // First write with a magnitude big enough to force a VSIDS rescale (1.03^1200 > 1e15),
  // which stores mag_scale = 1200 for U1.
  assert!(matches!(write_edge(&p, "", "U1", "U2", 1.0, 1200).await, Response::Ok));
  // Second write on the same src with a smaller magnitude: 5 - 1200 underflows u32.
  let _ = write_edge(&p, "", "U1", "U3", 1.0, 5).await;

  // If the worker is alive, this sync completes and returns Ok; if it panicked, the subgraph is
  // dead. (The timeout in `req` turns a dead-worker hang into a failure.)
  let s = sync(&p).await;
  assert!(
    matches!(s, Response::Ok),
    "mr_sync did not return Ok after a decreasing-magnitude write: worker thread likely panicked \
     (vsids.rs u32 subtraction underflow). Got {:?}",
    s
  );

  // The worker still applies later writes.
  assert!(matches!(write_edge(&p, "", "U1", "U4", 1.0, 1300).await, Response::Ok));
  assert!(matches!(sync(&p).await, Response::Ok));
  let edges = read_edges_vec(req(&p, "", ReqData::ReadEdges).await);
  assert!(
    edges.iter().any(|(s, d, _)| s == "U1" && d == "U4"),
    "a write after the decreasing magnitude was not applied: {:?}",
    edges
  );
}

/// LOW/MEDIUM — apply_op WriteZeroOpinion (aug_graph/absorb.rs:43-59). `MERITRANK_DISCREDIT_LAMBDA`,
/// `blame_decay` and `alpha` are validated finite at start-up (R23), but `mr_write_zero_opinion`
/// accepts an arbitrary `f64` with no finiteness check and stores it verbatim. A NaN/inf zero
/// opinion poisons served scores: `score*(1-k)+k*zero` becomes NaN/inf. `mr_scores` happens to hide
/// it (the score-range filter drops NaN), but `mr_node_score` returns the raw score with no filter,
/// so a NaN/inf reaches the client. A score read must never emit a non-finite number.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixed_nan_zero_opinion_rejected() {
  let p = processor();
  write_edge(&p, "", "U1", "U2", 1.0, 0).await;
  sync(&p).await;
  let _ = read_scores(&p, "", "U1", FilterOptions { count: 100, ..Default::default() }).await;
  // NaN zero opinion for U2.
  req(&p, "", ReqData::WriteZeroOpinion(OpWriteZeroOpinion { node: "U2".into(), score: f64::NAN })).await;
  sync(&p).await;

  let resp = req(&p, "", ReqData::ReadNodeScore(OpReadNodeScore { ego: "U1".into(), target: "U2".into() })).await;
  let scores = match resp {
    Response::Scores(ResScores { scores }) => scores,
    other => panic!("expected scores, got {:?}", other),
  };
  let bad: Vec<_> = scores.iter().filter(|s| !s.score.is_finite()).map(|s| (s.target.clone(), s.score)).collect();
  assert!(
    bad.is_empty(),
    "non-finite score served by mr_node_score after a NaN zero opinion: {:?} (WriteZeroOpinion does \
     not validate finiteness)",
    bad
  );
}

// ===========================================================================
// ATTACKS THAT DID NOT BREAK ANYTHING (regression tests, must stay green)
// ===========================================================================

/// R2: a negative weight (wall) in a NAMED context is rejected, and nothing is mutated.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_negative_weight_in_named_context_rejected() {
  let p = processor();
  let r = write_edge(&p, "X", "U1", "U2", -0.5, 0).await;
  assert!(matches!(r, Response::Fail), "expected Fail for wall in named context, got {:?}", r);
  sync(&p).await;
  // The rejected write created nothing in the null context.
  let edges = read_edges_vec(req(&p, "", ReqData::ReadEdges).await);
  assert!(edges.is_empty(), "rejected wall still mutated the graph: {:?}", edges);
}

/// R3: a negative self-edge is rejected.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_negative_self_edge_rejected() {
  let p = processor();
  assert!(matches!(write_edge(&p, "", "U1", "U1", -0.5, 0).await, Response::Fail));
  // A positive self-edge is also rejected (self-reference).
  assert!(matches!(write_edge(&p, "", "U1", "U1", 1.0, 0).await, Response::Fail));
}

/// Non-finite edge weights (NaN, +inf, -inf, and huge finite) are handled: NaN/inf rejected,
/// huge finite accepted without a panic or a dead worker.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_extreme_edge_weights() {
  let p = processor();
  assert!(matches!(write_edge(&p, "", "U1", "U2", f64::NAN, 0).await, Response::Fail));
  assert!(matches!(write_edge(&p, "", "U1", "U2", f64::INFINITY, 0).await, Response::Fail));
  assert!(matches!(write_edge(&p, "", "U1", "U2", f64::NEG_INFINITY, 0).await, Response::Fail));
  // Huge finite weights and an extreme magnitude must not crash the worker.
  assert!(matches!(write_edge(&p, "", "U3", "U4", 1e308, 0).await, Response::Ok));
  assert!(matches!(write_edge(&p, "", "U5", "U6", 1.0, u32::MAX).await, Response::Ok));
  assert!(matches!(sync(&p).await, Response::Ok), "worker died after extreme weights/magnitude");
}

/// A large `count` with `index = 0` (the default FilterOptions has count = u32::MAX) must return a
/// page, not overflow or panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_huge_count_index_zero() {
  let p = processor();
  write_edge(&p, "", "U1", "U2", 1.0, 0).await;
  sync(&p).await;
  let _ = read_scores(&p, "", "U1", FilterOptions { count: 100, ..Default::default() }).await;
  let r = try_request(&p, Request {
    subgraph: "".into(),
    data: ReqData::ReadScores(OpReadScores {
      ego: "U1".into(),
      score_options: FilterOptions { index: 0, count: u32::MAX, ..Default::default() },
    }),
  })
  .await;
  assert!(r.is_ok(), "{}", r.err().unwrap_or_default());
}

/// `mr_neighbors` with an out-of-range direction returns an empty result, no crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_neighbors_bad_direction() {
  let p = processor();
  write_edge(&p, "", "U1", "U2", 1.0, 0).await;
  sync(&p).await;
  let r = req(&p, "", ReqData::ReadNeighbors(OpReadNeighbors {
    ego: "U1".into(), focus: "U2".into(), direction: 999, kind: None, hide_personal: false,
    lt: 1e9, lte: false, gt: -1e9, gte: false, index: 0, count: 10,
  })).await;
  match r {
    Response::Scores(ResScores { scores }) => assert!(scores.is_empty()),
    other => panic!("expected empty scores, got {:?}", other),
  }
}

/// A read against an unknown subgraph returns Fail (no panic).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_read_unknown_subgraph() {
  let p = processor();
  let r = read_scores(&p, "does-not-exist", "U1", FilterOptions::default()).await;
  assert!(matches!(r, Response::Fail | Response::Scores(_)), "got {:?}", r);
}

/// Using a non-user (object) node as the ego yields no scores, gracefully.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_object_node_as_ego() {
  let p = processor();
  write_edge(&p, "", "U1", "C1", 1.0, 0).await; // registers C1 (Comment) owned by U1
  sync(&p).await;
  let r = read_scores(&p, "", "C1", FilterOptions::default()).await;
  match r {
    Response::Scores(ResScores { scores }) => assert!(scores.is_empty()),
    other => panic!("expected empty scores for object ego, got {:?}", other),
  }
}

/// Unbounded distinct context names create unbounded subgraphs (each spawns an OS worker thread and
/// two full graph copies), with no configured cap — a resource-exhaustion vector. Demonstrated at a
/// safe scale: the map grows one-for-one with attacker-chosen names.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_unbounded_context_creation_resource_growth() {
  let p = processor();
  const N: usize = 64;
  for i in 0..N {
    // (User -> Beacon) edge in a fresh context creates that context's subgraph.
    write_edge(&p, &format!("ctx{i}"), "U1", "B1", 1.0, 0).await;
  }
  sync(&p).await;
  let n = p.subgraphs_map.len();
  println!(
    "ok_unbounded_context_creation_resource_growth: {} subgraphs from {} attacker-named contexts \
     (each = 1 OS thread + 2 graph copies; no cap)",
    n, N
  );
  assert!(n >= N, "expected the subgraph count to grow with context names, got {}", n);
}

/// Wire robustness: a garbage / undecodable frame closes only that connection; the server stays up
/// and a fresh connection still works.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_garbage_frame_does_not_kill_server() {
  let (port, token, _h) = spawn_server(fast_settings()).await;

  // Frame with a valid length but garbage bincode payload.
  {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let garbage = [0xFFu8; 16];
    s.write_all(&(garbage.len() as u32).to_be_bytes()).await.unwrap();
    s.write_all(&garbage).await.unwrap();
    s.flush().await.unwrap();
    // Server should drop this connection (read side eventually closes).
    let mut buf = [0u8; 1];
    let _ = timeout(Duration::from_secs(1), s.read(&mut buf)).await;
  }

  // A fresh, well-formed connection still gets served.
  let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
  let request = Request { subgraph: "".into(), data: ReqData::ReadNodeList };
  s.write_all(&frame(&request)).await.unwrap();
  s.flush().await.unwrap();

  let mut len_buf = [0u8; 4];
  timeout(T, s.read_exact(&mut len_buf)).await.expect("server did not respond after a garbage frame").unwrap();
  let len = u32::from_be_bytes(len_buf) as usize;
  assert!(len > 0 && len < 1_000_000);
  let mut body = vec![0u8; len];
  s.read_exact(&mut body).await.unwrap();
  let (resp, _): (Response, _) = bincode::decode_from_slice(&body, bincode::config::standard()).unwrap();
  assert!(matches!(resp, Response::NodeList(_)), "got {:?}", resp);

  token.cancel();
}

/// A zero-length frame (len = 0) is handled without hanging or crashing the server.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_zero_length_frame() {
  let (port, token, _h) = spawn_server(fast_settings()).await;
  {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    s.write_all(&0u32.to_be_bytes()).await.unwrap();
    s.flush().await.unwrap();
    let mut buf = [0u8; 1];
    let _ = timeout(Duration::from_secs(1), s.read(&mut buf)).await;
  }
  // Server still alive.
  let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
  s.write_all(&frame(&Request { subgraph: "".into(), data: ReqData::ReadNodeList })).await.unwrap();
  s.flush().await.unwrap();
  let mut len_buf = [0u8; 4];
  timeout(T, s.read_exact(&mut len_buf)).await.expect("server hung after zero-length frame").unwrap();
  token.cancel();
}

/// Bulk load with one invalid wall (wall in a named context) is rejected as a whole (R20), and the
/// graph is left empty — no partial mutation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ok_bulk_load_with_invalid_wall_rejected_atomically() {
  let p = processor();
  let edges = vec![
    BulkEdge { src: "U1".into(), dst: "U2".into(), amount: 1.0, magnitude: 0, context: "".into() },
    // Invalid: negative weight in a named context.
    BulkEdge { src: "U3".into(), dst: "U4".into(), amount: -0.5, magnitude: 0, context: "X".into() },
  ];
  let r = req(&p, "", ReqData::WriteBulkEdges(OpWriteBulkEdges { edges })).await;
  assert!(matches!(r, Response::Fail), "invalid bulk batch should be rejected, got {:?}", r);
  sync(&p).await;
  let after = read_edges_vec(req(&p, "", ReqData::ReadEdges).await);
  assert!(after.is_empty(), "rejected bulk batch mutated the graph: {:?}", after);
}

/// Was MEDIUM (fixed) — any write naming a fresh context created a subgraph (a worker thread and
/// two graph copies) without limit. MERITRANK_MAX_CONTEXTS now caps them: explicit and implicit
/// creation beyond it fail, existing contexts keep working.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixed_context_creation_is_capped() {
  let p = Arc::new(MultiGraphProcessor::new(Settings {
    max_contexts: 3,
    ..fast_settings()
  }));
  for c in ["A", "B", "C"] {
    assert!(matches!(req(&p, c, ReqData::WriteCreateContext).await, Response::Ok));
  }
  assert!(matches!(req(&p, "D", ReqData::WriteCreateContext).await, Response::Fail));
  assert!(matches!(write_edge(&p, "E", "B1", "U1", 1.0, 0).await, Response::Fail));
  assert_eq!(p.subgraphs_map.len(), 4, "null context + 3");
  assert!(matches!(write_edge(&p, "A", "B1", "U1", 1.0, 0).await, Response::Ok));
  assert!(matches!(write_edge(&p, "", "U1", "U2", 1.0, 0).await, Response::Ok));
  assert!(matches!(sync(&p).await, Response::Ok));
}
