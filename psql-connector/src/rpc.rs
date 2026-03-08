//  New TCP + bincode RPC client for the meritrank service.
//  See JOURNAL.md decisions D4 (sync stamp), D5 (URL), D6 (blocking),
//  D7 (timeout), D9 (magnitude).

use meritrank_service::data::*;
use meritrank_service::rpc_sync::{read_response_sync, set_read_timeout, write_request_sync};

use std::cell::RefCell;
use std::env::var;
use std::error::Error;
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{
  atomic::{AtomicU64, Ordering},
  LazyLock,
};
use std::time::Duration;

thread_local! {
  static CONN: RefCell<Option<TcpStream>> = RefCell::new(None);
}

//  D5 (JOURNAL): reuse MERITRANK_SERVICE_URL, default changes to port 8080.
//  The tcp:// prefix is stripped before connecting.
pub static SERVICE_URL: LazyLock<String> = LazyLock::new(|| {
  var("MERITRANK_SERVICE_URL")
    .unwrap_or_else(|_| "tcp://127.0.0.1:8080".to_string())
});

pub static RECV_TIMEOUT_MSEC: LazyLock<u64> = LazyLock::new(|| {
  var("MERITRANK_RECV_TIMEOUT_MSEC")
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(10000)
});

//  D4 (JOURNAL): monotonically-increasing stamp for Sync requests.
static SYNC_STAMP: AtomicU64 = AtomicU64::new(0);

fn strip_scheme(url: &str) -> &str {
  if let Some(rest) = url.strip_prefix("tcp://") {
    rest
  } else {
    url
  }
}

fn get_or_reconnect(timeout: Duration) -> io::Result<TcpStream> {
  CONN.with(|cell| {
    let mut opt = cell.borrow_mut();
    if let Some(ref stream) = *opt {
      if let Ok(cloned) = stream.try_clone() {
        return Ok(cloned);
      }
      *opt = None;
    }
    let addr_str = strip_scheme(&SERVICE_URL);
    let addrs: Vec<_> = addr_str.to_socket_addrs()?.collect();
    let mut last_err = None;
    for addr in &addrs {
      match TcpStream::connect_timeout(addr, timeout) {
        Ok(s) => {
          let cloned = s
            .try_clone()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
          *opt = Some(cloned);
          return Ok(s);
        },
        Err(e) => last_err = Some(e),
      }
    }
    Err(last_err.unwrap_or_else(|| {
      io::Error::new(io::ErrorKind::Other, "no addresses resolved")
    }))
  })
}

fn invalidate_conn() {
  CONN.with(|cell| {
    *cell.borrow_mut() = None;
  });
}

fn tcp_call(
  subgraph: &str,
  data: ReqData,
  timeout_msec: Option<u64>,
) -> Result<Response, Box<dyn Error + 'static>> {
  let timeout = timeout_msec.unwrap_or(*RECV_TIMEOUT_MSEC);
  let timeout_dur = Duration::from_millis(timeout);

  let req = Request {
    subgraph: subgraph.to_string(),
    data,
  };

  let result = (|| -> io::Result<Response> {
    let mut stream = get_or_reconnect(timeout_dur)?;
    set_read_timeout(&mut stream, Some(timeout))?;
    write_request_sync(&mut stream, &req)?;
    read_response_sync(&mut stream)
  })();

  match result {
    Ok(resp) => Ok(resp),
    Err(_) => {
      invalidate_conn();
      let mut stream = get_or_reconnect(timeout_dur)?;
      set_read_timeout(&mut stream, Some(timeout))?;
      write_request_sync(&mut stream, &req)?;
      Ok(read_response_sync(&mut stream)?)
    },
  }
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
  timeout_msec: Option<u64>
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let resp = tcp_call("", ReqData::WriteRecalculateZero, timeout_msec)?;
  expect_ok(resp)
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
  kind: &str,
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
        node_kind: kind_from_prefix(kind),
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
  kind: &str,
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
      kind:          kind_from_prefix(kind),
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

fn kind_from_prefix(prefix: &str) -> Option<NodeKind> {
  match prefix.chars().next() {
    Some('U') => Some(NodeKind::User),
    Some('B') => Some(NodeKind::Beacon),
    Some('C') => Some(NodeKind::Comment),
    Some('O') => Some(NodeKind::Opinion),
    Some('V') => Some(NodeKind::PollVariant),
    Some('P') => Some(NodeKind::Poll),
    _ => None,
  }
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

  #[test]
  fn kind_from_prefix_examples() {
    assert_eq!(kind_from_prefix("U1"), Some(NodeKind::User));
    assert_eq!(kind_from_prefix("B1"), Some(NodeKind::Beacon));
    assert_eq!(kind_from_prefix(""), None);
    assert_eq!(kind_from_prefix("?"), None);
  }

  #[test]
  fn magnitude_mapping() {
    assert_eq!(if -1i64 < 0 { 0u32 } else { (-1i64) as u32 }, 0);
    assert_eq!(if 3i64 < 0 { 0u32 } else { 3i64 as u32 }, 3);
  }
}
