//  New TCP + bincode RPC client for the meritrank service.
//  See JOURNAL.md decisions D4 (sync stamp), D5 (URL), D6 (blocking),
//  D7 (timeout), D9 (magnitude).

use meritrank_service::data::*;
use meritrank_service::request_handler::max_frame_bytes;

use bincode::{config::standard, decode_from_slice, encode_to_vec};

use std::cell::RefCell;
use std::env::var;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{
  atomic::{AtomicU64, Ordering},
  mpsc, LazyLock,
};
use std::time::{Duration, Instant};

//  One cached connection per backend, tagged with the address it serves.
thread_local! {
  static CONN: RefCell<Option<(String, TcpStream)>> = RefCell::new(None);
  static ADDRS: RefCell<Option<(String, Vec<SocketAddr>)>> = RefCell::new(None);
}

//  D5 (JOURNAL): reuse MERITRANK_SERVICE_URL, default changes to port 8080.
//  The tcp:// prefix is stripped before connecting.
pub static SERVICE_URL: LazyLock<String> = LazyLock::new(|| {
  var("MERITRANK_SERVICE_URL")
    .unwrap_or_else(|_| "tcp://127.0.0.1:8080".to_string())
});

//  The whole-call budget: DNS, connect, write, the complete response read and
//  the one reconnect retry all fit inside it (D11, JOURNAL).
pub static RECV_TIMEOUT_MSEC: LazyLock<u64> = LazyLock::new(|| {
  var("MERITRANK_RECV_TIMEOUT_MSEC")
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(10000)
});

//  The longest single blocking socket wait. Between slices a call re-checks
//  its deadline and lets PostgreSQL process interrupts, so statement_timeout
//  and pg_cancel_backend() also bound a call that is waiting on the network.
const IO_SLICE: Duration = Duration::from_millis(50);

//  D4 (JOURNAL): monotonically-increasing stamp for Sync requests.
static SYNC_STAMP: AtomicU64 = AtomicU64::new(0);

//  Request frames put on the wire by this backend, retries included.
static NETWORK_ATTEMPTS: AtomicU64 = AtomicU64::new(0);

pub fn network_attempts() -> u64 {
  NETWORK_ATTEMPTS.load(Ordering::Relaxed)
}

#[cfg(not(test))]
fn check_interrupts() {
  pgrx::check_for_interrupts!();
}

#[cfg(test)]
fn check_interrupts() {}

fn strip_scheme(url: &str) -> &str {
  if let Some(rest) = url.strip_prefix("tcp://") {
    rest
  } else {
    url
  }
}

fn deadline_exceeded() -> io::Error {
  io::Error::new(io::ErrorKind::TimedOut, "meritrank: call deadline exceeded")
}

fn is_deadline(e: &io::Error) -> bool {
  e.kind() == io::ErrorKind::TimedOut
}

fn is_wait_timeout(e: &io::Error) -> bool {
  matches!(
    e.kind(),
    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
  )
}

//  The next blocking wait: at most IO_SLICE, never past the deadline.
fn next_slice(deadline: Instant) -> io::Result<Duration> {
  let remaining = deadline.saturating_duration_since(Instant::now());
  if remaining.is_zero() {
    return Err(deadline_exceeded());
  }
  Ok(remaining.min(IO_SLICE).max(Duration::from_millis(1)))
}

//  Name resolution can block indefinitely inside libc, so it runs on a helper
//  thread that the call stops waiting for at the deadline. Results are cached
//  per backend and dropped whenever connecting fails.
fn resolve(addr: &str, deadline: Instant) -> io::Result<Vec<SocketAddr>> {
  if let Ok(literal) = addr.parse::<SocketAddr>() {
    return Ok(vec![literal]);
  }
  let cached = ADDRS.with(|cell| {
    cell
      .borrow()
      .as_ref()
      .filter(|(a, _)| a == addr)
      .map(|(_, v)| v.clone())
  });
  if let Some(addrs) = cached {
    return Ok(addrs);
  }
  let (tx, rx) = mpsc::channel();
  let owned = addr.to_string();
  std::thread::spawn(move || {
    let _ = tx.send(owned.to_socket_addrs().map(|it| it.collect::<Vec<_>>()));
  });
  loop {
    match rx.recv_timeout(next_slice(deadline)?) {
      Ok(Ok(addrs)) if !addrs.is_empty() => {
        ADDRS.with(|cell| *cell.borrow_mut() = Some((addr.to_string(), addrs.clone())));
        return Ok(addrs);
      },
      Ok(Ok(_)) => {
        return Err(io::Error::new(io::ErrorKind::Other, "no addresses resolved"))
      },
      Ok(Err(e)) => return Err(e),
      Err(mpsc::RecvTimeoutError::Timeout) => check_interrupts(),
      Err(mpsc::RecvTimeoutError::Disconnected) => {
        return Err(io::Error::new(io::ErrorKind::Other, "name resolution failed"))
      },
    }
  }
}

fn forget_addrs() {
  ADDRS.with(|cell| *cell.borrow_mut() = None);
}

fn connect(addr: &str, deadline: Instant) -> io::Result<TcpStream> {
  let addrs = resolve(addr, deadline)?;
  loop {
    for sa in &addrs {
      match TcpStream::connect_timeout(sa, next_slice(deadline)?) {
        Ok(s) => {
          //  Small request/response frames: never wait for Nagle.
          s.set_nodelay(true)?;
          return Ok(s);
        },
        Err(e) if is_wait_timeout(&e) => {},
        Err(e) => {
          forget_addrs();
          return Err(e);
        },
      }
      check_interrupts();
    }
    if Instant::now() >= deadline {
      forget_addrs();
      return Err(deadline_exceeded());
    }
  }
}

fn write_all_by(stream: &mut TcpStream, mut buf: &[u8], deadline: Instant) -> io::Result<()> {
  while !buf.is_empty() {
    stream.set_write_timeout(Some(next_slice(deadline)?))?;
    match stream.write(buf) {
      Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
      Ok(n) => buf = &buf[n..],
      Err(e) if is_wait_timeout(&e) => {},
      Err(e) => return Err(e),
    }
    check_interrupts();
  }
  Ok(())
}

fn read_exact_by(stream: &mut TcpStream, buf: &mut [u8], deadline: Instant) -> io::Result<()> {
  let mut filled = 0;
  while filled < buf.len() {
    stream.set_read_timeout(Some(next_slice(deadline)?))?;
    match stream.read(&mut buf[filled..]) {
      Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
      Ok(n) => filled += n,
      Err(e) if is_wait_timeout(&e) => {},
      Err(e) => return Err(e),
    }
    check_interrupts();
  }
  Ok(())
}

fn encode_frame(req: &Request) -> io::Result<Vec<u8>> {
  let payload = encode_to_vec(req, standard())
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
  //  One write per frame (see rpc_sync::write_request_sync).
  let mut frame = Vec::with_capacity(4 + payload.len());
  frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
  frame.extend_from_slice(&payload);
  Ok(frame)
}

//  Sends one frame and reads one complete response frame by the deadline.
fn exchange(stream: &mut TcpStream, frame: &[u8], deadline: Instant) -> io::Result<Response> {
  NETWORK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
  write_all_by(stream, frame, deadline)?;
  let mut len_buf = [0u8; 4];
  read_exact_by(stream, &mut len_buf, deadline)?;
  let len = u32::from_be_bytes(len_buf) as usize;
  let max = max_frame_bytes();
  if len > max {
    return Err(io::Error::new(
      io::ErrorKind::InvalidData,
      format!("frame of {} bytes exceeds the limit of {}", len, max),
    ));
  }
  let mut buf = vec![0u8; len];
  read_exact_by(stream, &mut buf, deadline)?;
  decode_from_slice(&buf, standard())
    .map(|(v, _)| v)
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

//  The cached connection is taken out for the duration of a call and put back
//  only after a complete exchange, so a call that fails, times out or is
//  interrupted never leaves a half-read stream behind.
fn take_cached(addr: &str) -> Option<TcpStream> {
  CONN.with(|cell| {
    let mut slot = cell.borrow_mut();
    match slot.take() {
      Some((a, s)) if a == addr => Some(s),
      _ => None,
    }
  })
}

fn put_back(addr: &str, stream: TcpStream) {
  CONN.with(|cell| *cell.borrow_mut() = Some((addr.to_string(), stream)));
}

fn call_by(addr: &str, frame: &[u8], deadline: Instant) -> io::Result<Response> {
  //  A cached connection may have been closed by the peer while idle; one
  //  failure on it is retried on a fresh connection within the same deadline.
  //  A deadline miss is never retried, and a fresh connection is never retried.
  if let Some(mut stream) = take_cached(addr) {
    match exchange(&mut stream, frame, deadline) {
      Ok(resp) => {
        put_back(addr, stream);
        return Ok(resp);
      },
      Err(e) if is_deadline(&e) => return Err(e),
      Err(_) => {},
    }
  }
  let mut stream = connect(addr, deadline)?;
  let resp = exchange(&mut stream, frame, deadline)?;
  put_back(addr, stream);
  Ok(resp)
}

fn tcp_call(
  subgraph: &str,
  data: ReqData,
  timeout_msec: Option<u64>,
) -> Result<Response, Box<dyn Error + 'static>> {
  let budget_msec = timeout_msec.unwrap_or(*RECV_TIMEOUT_MSEC);
  let deadline = Instant::now() + Duration::from_millis(budget_msec);
  let req = Request {
    subgraph: subgraph.to_string(),
    data,
  };
  let frame = encode_frame(&req)?;
  call_by(strip_scheme(&SERVICE_URL), &frame, deadline).map_err(|e| {
    if is_deadline(&e) {
      format!("meritrank: no complete response within {} ms", budget_msec).into()
    } else {
      e.into()
    }
  })
}

fn expect_ok(resp: Response) -> Result<&'static str, Box<dyn Error + 'static>> {
  match resp {
    Response::Ok => Ok("Ok"),
    Response::Fail => Err("Service returned Fail".into()),
    Response::NotImplemented => Err("meritrank: operation not implemented".into()),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

// ================================================================
//
//    Write operations
//
// ================================================================

pub fn new_reset() -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call("", ReqData::WriteReset, Some(*RECV_TIMEOUT_MSEC))?;
  expect_ok(resp)
}

pub fn new_create_context(
  context: &str
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp =
    tcp_call(context, ReqData::WriteCreateContext, Some(*RECV_TIMEOUT_MSEC))?;
  expect_ok(resp)
}

pub fn new_put_edge(
  src: &str,
  dst: &str,
  weight: f64,
  context: &str,
  index: i64,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  //  D9 (JOURNAL): negative index maps to magnitude 0.
  let magnitude = if index < 0 { 0u32 } else { index as u32 };
  let resp = tcp_call(
    context,
    ReqData::WriteEdge(OpWriteEdge {
      src:       src.to_string(),
      dst:       dst.to_string(),
      amount:    weight,
      magnitude,
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )?;
  expect_ok(resp)
}

pub fn new_bulk_load_edges(
  edges: Vec<BulkEdge>,
  timeout_msec: Option<u64>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let timeout = timeout_msec.unwrap_or(120_000);
  let resp = tcp_call(
    "",
    ReqData::WriteBulkEdges(OpWriteBulkEdges { edges }),
    Some(timeout),
  )?;
  expect_ok(resp)
}

pub fn new_delete_edge(
  src: &str,
  dst: &str,
  context: &str,
  index: i64,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call(
    context,
    ReqData::WriteDeleteEdge(OpWriteDeleteEdge {
      src:   src.to_string(),
      dst:   dst.to_string(),
      index,
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )?;
  expect_ok(resp)
}

pub fn new_delete_node(
  node: &str,
  context: &str,
  index: i64,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call(
    context,
    ReqData::WriteDeleteNode(OpWriteDeleteNode {
      node:  node.to_string(),
      index,
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )?;
  expect_ok(resp)
}

pub fn new_set_zero_opinion(
  node: &str,
  score: f64,
  context: &str,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call(
    context,
    ReqData::WriteZeroOpinion(OpWriteZeroOpinion {
      node:  node.to_string(),
      score,
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )?;
  expect_ok(resp)
}

pub fn new_sync(
  timeout_msec: Option<u64>
) -> Result<&'static str, Box<dyn Error + 'static>> {
  //  D4 (JOURNAL): increment the per-process stamp and send it.
  let stamp = SYNC_STAMP.fetch_add(1, Ordering::SeqCst) + 1;
  let resp = tcp_call("", ReqData::Sync(stamp), timeout_msec)?;
  expect_ok(resp)
}

pub fn new_reset_stats(
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call("", ReqData::ResetStats, Some(*RECV_TIMEOUT_MSEC))?;
  expect_ok(resp)
}

pub fn new_get_stats(
) -> Result<ResStats, Box<dyn Error + 'static>> {
  let resp = tcp_call("", ReqData::GetStats, Some(*RECV_TIMEOUT_MSEC))?;
  match resp {
    Response::Stats(s) => Ok(s),
    Response::Fail => Err("Service returned Fail".into()),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_zerorec(
  _timeout_msec: Option<u64>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  Err("recalculate_zero has been removed".into())
}

pub fn new_recalculate_clustering(
  timeout_msec: Option<u64>
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call(
    "",
    ReqData::WriteRecalculateClustering,
    timeout_msec,
  )?;
  expect_ok(resp)
}

//  D10 (JOURNAL): server returns NotImplemented for new edges filter; connector reports as error.
pub fn new_set_new_edges_filter(
  src: &str,
  filter: Vec<u8>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call(
    "",
    ReqData::WriteNewEdgesFilter(OpWriteNewEdgesFilter {
      src:    src.to_string(),
      filter,
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )?;
  match resp {
    Response::NotImplemented => Err("meritrank: set_new_edges_filter not implemented".into()),
    _ => expect_ok(resp),
  }
}

// ================================================================
//
//    Read operations
//
// ================================================================

pub fn new_node_list(
  context: &str
) -> Result<Vec<(String,)>, Box<dyn Error + 'static>> {
  match tcp_call(context, ReqData::ReadNodeList, Some(*RECV_TIMEOUT_MSEC))? {
    Response::NodeList(r) => Ok(r.nodes),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_node_score(
  ego: &str,
  target: &str,
  context: &str,
) -> Result<Vec<(String, String, f64, f64, i32, i32)>, Box<dyn Error + 'static>> {
  match tcp_call(
    context,
    ReqData::ReadNodeScore(OpReadNodeScore {
      ego:    ego.to_string(),
      target: target.to_string(),
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::Scores(r) => Ok(scores_to_tuples(r.scores)),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_scores(
  ego: &str,
  hide_personal: bool,
  context: &str,
  // Kept for the SQL signature; ignored (D14: one node class).
  _kind: &str,
  lt: Option<f64>,
  lte: Option<f64>,
  gt: Option<f64>,
  gte: Option<f64>,
  index: u32,
  count: u32,
) -> Result<Vec<(String, String, f64, f64, i32, i32)>, Box<dyn Error + 'static>> {
  //  D8 (JOURNAL): map None bounds to f64::MAX/MIN with appropriate lte/gte flags.
  let (score_lt, score_lte, score_gt, score_gte) = map_bounds(lt, lte, gt, gte)?;
  match tcp_call(
    context,
    ReqData::ReadScores(OpReadScores {
      ego:           ego.to_string(),
      score_options: FilterOptions {
        // The service has one node class (D14): the kind filter is ignored.
        node_kind: None,
        hide_personal,
        score_lt,
        score_lte,
        score_gt,
        score_gte,
        index,
        count,
      },
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::Scores(r) => Ok(scores_to_tuples(r.scores)),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_graph(
  ego: &str,
  focus: &str,
  context: &str,
  positive_only: bool,
  index: u64,
  count: u64,
) -> Result<Vec<(String, String, f64, f64, f64, i32, i32)>, Box<dyn Error + 'static>> {
  match tcp_call(
    context,
    ReqData::ReadGraph(OpReadGraph {
      ego: ego.to_string(),
      focus: focus.to_string(),
      positive_only,
      index,
      count,
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::Graph(r) => Ok(graph_to_tuples(r.graph)),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_neighbors(
  ego: &str,
  focus: &str,
  direction: i64,
  hide_personal: bool,
  context: &str,
  // Kept for the SQL signature; ignored (D14: one node class).
  _kind: &str,
  lt: Option<f64>,
  lte: Option<f64>,
  gt: Option<f64>,
  gte: Option<f64>,
  index: u32,
  count: u32,
) -> Result<Vec<(String, String, f64, f64, i32, i32)>, Box<dyn Error + 'static>> {
  let (score_lt, score_lte, score_gt, score_gte) = map_bounds(lt, lte, gt, gte)?;
  match tcp_call(
    context,
    ReqData::ReadNeighbors(OpReadNeighbors {
      ego:           ego.to_string(),
      focus:         focus.to_string(),
      direction,
      // The service has one node class (D14): the kind filter is ignored.
      kind:          None,
      hide_personal,
      lt:            score_lt,
      lte:           score_lte,
      gt:            score_gt,
      gte:           score_gte,
      index,
      count,
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::Scores(r) => Ok(scores_to_tuples(r.scores)),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_edgelist(
  context: &str
) -> Result<Vec<(String, String, f64)>, Box<dyn Error + 'static>> {
  match tcp_call(context, ReqData::ReadEdges, Some(*RECV_TIMEOUT_MSEC))? {
    Response::Edges(r) => Ok(
      r.edges
        .into_iter()
        .map(|e| (e.src, e.dst, e.weight))
        .collect(),
    ),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_connected(
  ego: &str,
  context: &str,
) -> Result<Vec<(String, String)>, Box<dyn Error + 'static>> {
  match tcp_call(
    context,
    ReqData::ReadConnected(OpReadConnected {
      node: ego.to_string(),
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::Connections(r) => Ok(r.connections.into_iter().map(|c| (c.src, c.dst)).collect()),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_mutual_scores(
  ego: &str,
  context: &str,
) -> Result<Vec<(String, String, f64, f64, i32, i32)>, Box<dyn Error + 'static>> {
  match tcp_call(
    context,
    ReqData::ReadMutualScores(OpReadMutualScores {
      ego: ego.to_string(),
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::Scores(r) => Ok(scores_to_tuples(r.scores)),
    Response::Fail => Ok(vec![]),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_get_new_edges_filter(
  src: &str
) -> Result<Vec<u8>, Box<dyn Error + 'static>> {
  match tcp_call(
    "",
    ReqData::ReadNewEdgesFilter(OpReadNewEdgesFilter {
      src: src.to_string(),
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::NewEdgesFilter(r) => Ok(r.bytes),
    Response::NotImplemented => Err("meritrank: get_new_edges_filter not implemented".into()),
    Response::Fail => Err("Service returned Fail".into()),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

pub fn new_fetch_new_edges(
  src: &str,
  prefix: &str,
) -> Result<Vec<(String, String, f64, f64, i32, i32)>, Box<dyn Error + 'static>> {
  match tcp_call(
    "",
    ReqData::WriteFetchNewEdges(OpWriteFetchNewEdges {
      src:    src.to_string(),
      prefix: prefix.to_string(),
    }),
    Some(*RECV_TIMEOUT_MSEC),
  )? {
    Response::NewEdges(r) => Ok(
      r.new_edges
        .into_iter()
        .map(|e| {
          (
            src.to_string(),
            e.node,
            e.score,
            e.score_reversed,
            e.cluster as i32,
            e.cluster_reversed as i32,
          )
        })
        .collect(),
    ),
    Response::NotImplemented => Err("meritrank: fetch_new_edges not implemented".into()),
    Response::Fail => Err("Service returned Fail".into()),
    other => Err(format!("Unexpected response: {:?}", other).into()),
  }
}

// ================================================================
//
//    Helpers
//
// ================================================================

fn scores_to_tuples(
  scores: Vec<ScoreResult>
) -> Vec<(String, String, f64, f64, i32, i32)> {
  scores
    .into_iter()
    .map(|s| {
      (
        s.ego,
        s.target,
        s.score,
        s.reverse_score,
        s.cluster as i32,
        s.reverse_cluster as i32,
      )
    })
    .collect()
}

fn graph_to_tuples(
  graph: Vec<GraphResult>
) -> Vec<(String, String, f64, f64, f64, i32, i32)> {
  graph
    .into_iter()
    .map(|g| {
      (
        g.src,
        g.dst,
        g.weight,
        g.score,
        g.reverse_score,
        g.cluster as i32,
        g.reverse_cluster as i32,
      )
    })
    .collect()
}


//  D8 (JOURNAL): map Option<f64> bounds to (value, flag) pairs.
fn map_bounds(
  lt: Option<f64>,
  lte: Option<f64>,
  gt: Option<f64>,
  gte: Option<f64>,
) -> Result<(f64, bool, f64, bool), Box<dyn Error + 'static>> {
  if lt.is_some() && lte.is_some() {
    return Err("either lt or lte is allowed!".into());
  }
  if gt.is_some() && gte.is_some() {
    return Err("either gt or gte is allowed!".into());
  }
  Ok((
    lt.unwrap_or_else(|| lte.unwrap_or(f64::MAX)),
    lte.is_some(),
    gt.unwrap_or_else(|| gte.unwrap_or(f64::MIN)),
    gte.is_some(),
  ))
}

// ================================================================
//
//    Unit tests (no server required)
//
// ================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strip_scheme_removes_tcp_prefix() {
    assert_eq!(strip_scheme("tcp://127.0.0.1:8080"), "127.0.0.1:8080");
    assert_eq!(strip_scheme("127.0.0.1:8080"), "127.0.0.1:8080");
  }

  #[test]
  fn map_bounds_defaults() {
    let (lt, lte, gt, gte) = map_bounds(None, None, None, None).unwrap();
    assert_eq!(lt, f64::MAX);
    assert!(!lte);
    assert_eq!(gt, f64::MIN);
    assert!(!gte);
  }

  #[test]
  fn map_bounds_explicit_lt_gt() {
    let (lt, lte, gt, gte) = map_bounds(Some(1.0), None, Some(0.1), None).unwrap();
    assert_eq!(lt, 1.0);
    assert!(!lte);
    assert_eq!(gt, 0.1);
    assert!(!gte);
  }

  #[test]
  fn map_bounds_lte_gte() {
    let (lt, lte, gt, gte) = map_bounds(None, Some(1.0), None, Some(0.1)).unwrap();
    assert_eq!(lt, 1.0);
    assert!(lte);
    assert_eq!(gt, 0.1);
    assert!(gte);
  }

  #[test]
  fn map_bounds_both_lt_and_lte_is_error() {
    assert!(map_bounds(Some(1.0), Some(1.0), None, None).is_err());
  }

  #[test]
  fn sync_stamp_increments() {
    let before = SYNC_STAMP.load(Ordering::SeqCst);
    //  new_sync connects to a (possibly absent) server; we expect either Ok
    //  (if a server happens to be running) or an Err (connection refused).
    //  Either way, the stamp must have been incremented.
    let _ = new_sync(Some(50));
    let after = SYNC_STAMP.load(Ordering::SeqCst);
    assert!(after > before);
  }

  //  ---- deadline tests against local fake peers -------------------------

  use std::net::TcpListener;
  use std::thread;

  fn ok_frame() -> Vec<u8> {
    let payload = encode_to_vec(&Response::Ok, standard()).unwrap();
    let mut f = (payload.len() as u32).to_be_bytes().to_vec();
    f.extend_from_slice(&payload);
    f
  }

  fn read_request(s: &mut TcpStream) -> bool {
    let mut len = [0u8; 4];
    if s.read_exact(&mut len).is_err() {
      return false;
    }
    let mut body = vec![0u8; u32::from_be_bytes(len) as usize];
    s.read_exact(&mut body).is_ok()
  }

  fn sync_frame() -> Vec<u8> {
    encode_frame(&Request {
      subgraph: String::new(),
      data:     ReqData::Sync(1),
    })
    .unwrap()
  }

  fn within(deadline_ms: u64) -> Instant {
    Instant::now() + Duration::from_millis(deadline_ms)
  }

  #[test]
  fn blackhole_peer_fails_at_the_deadline_without_retrying() {
    //  Accepts and reads, never answers.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    thread::spawn(move || {
      for s in listener.incoming() {
        let mut s = s.unwrap();
        thread::spawn(move || {
          let mut sink = [0u8; 1024];
          while s.read(&mut sink).map(|n| n > 0).unwrap_or(false) {}
        });
      }
    });
    let started = Instant::now();
    let err = call_by(&addr, &sync_frame(), within(300)).unwrap_err();
    let took = started.elapsed();
    assert!(is_deadline(&err), "{err:?}");
    assert!(took >= Duration::from_millis(280), "{took:?}");
    assert!(took < Duration::from_millis(600), "{took:?}");
  }

  #[test]
  fn trickled_response_is_bounded_by_the_whole_call_deadline() {
    //  Sends the response one byte every 100 ms: each read makes progress, so
    //  only an absolute deadline (not a per-read timeout) can stop it.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    thread::spawn(move || {
      let (mut s, _) = listener.accept().unwrap();
      read_request(&mut s);
      let mut frame = vec![0u8, 0, 1, 0];
      frame.extend(std::iter::repeat(0u8).take(256));
      for b in frame {
        if s.write_all(&[b]).is_err() {
          return;
        }
        thread::sleep(Duration::from_millis(100));
      }
    });
    let started = Instant::now();
    let err = call_by(&addr, &sync_frame(), within(400)).unwrap_err();
    assert!(is_deadline(&err), "{err:?}");
    assert!(started.elapsed() < Duration::from_millis(700));
  }

  #[test]
  fn stalled_write_is_bounded_by_the_deadline() {
    //  Accepts but never reads; a large frame fills both socket buffers.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    thread::spawn(move || {
      let (_s, _) = listener.accept().unwrap();
      thread::sleep(Duration::from_secs(5));
    });
    let big = vec![7u8; 64 * 1024 * 1024];
    let started = Instant::now();
    let err = call_by(&addr, &big, within(300)).unwrap_err();
    assert!(is_deadline(&err), "{err:?}");
    assert!(started.elapsed() < Duration::from_millis(700));
  }

  #[test]
  fn unreachable_peer_connect_is_bounded_by_the_deadline() {
    //  A non-routable address: SYNs go unanswered.
    let started = Instant::now();
    let err = call_by("10.255.255.1:9", &sync_frame(), within(300)).unwrap_err();
    assert!(started.elapsed() < Duration::from_millis(700), "{err:?}");
  }

  #[test]
  fn healthy_peer_answers_and_connection_is_reused() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let accepted = std::sync::Arc::new(AtomicU64::new(0));
    let counter = accepted.clone();
    thread::spawn(move || {
      for s in listener.incoming() {
        counter.fetch_add(1, Ordering::SeqCst);
        let mut s = s.unwrap();
        thread::spawn(move || {
          while read_request(&mut s) {
            if s.write_all(&ok_frame()).is_err() {
              return;
            }
          }
        });
      }
    });
    let before = network_attempts();
    for _ in 0..3 {
      assert!(matches!(call_by(&addr, &sync_frame(), within(1000)).unwrap(), Response::Ok));
    }
    assert_eq!(network_attempts() - before, 3);
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn recovers_on_a_fresh_connection_after_the_peer_drops_an_idle_one() {
    //  First connection answers once and is closed by the peer; the next call
    //  fails on the stale stream and succeeds on one retry within its deadline.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    thread::spawn(move || {
      let mut first = true;
      for s in listener.incoming() {
        let mut s = s.unwrap();
        let once = first;
        first = false;
        thread::spawn(move || {
          while read_request(&mut s) {
            if s.write_all(&ok_frame()).is_err() || once {
              return;
            }
          }
        });
      }
    });
    assert!(matches!(call_by(&addr, &sync_frame(), within(1000)).unwrap(), Response::Ok));
    thread::sleep(Duration::from_millis(50));
    assert!(matches!(call_by(&addr, &sync_frame(), within(1000)).unwrap(), Response::Ok));
  }

  #[test]
  fn a_failed_call_leaves_no_cached_connection_behind() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    thread::spawn(move || {
      let (_s, _) = listener.accept().unwrap();
      thread::sleep(Duration::from_secs(2));
    });
    assert!(call_by(&addr, &sync_frame(), within(200)).is_err());
    assert!(take_cached(&addr).is_none());
  }

  #[test]
  fn magnitude_mapping() {
    assert_eq!(if -1i64 < 0 { 0u32 } else { (-1i64) as u32 }, 0);
    assert_eq!(if 3i64 < 0 { 0u32 } else { 3i64 as u32 }, 3);
  }
}
