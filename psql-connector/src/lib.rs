use core::result::Result;
use lazy_static::lazy_static;
use meritrank_service::protocol::*;
use nng::options::{Options, RecvTimeout};
use nng::*;
use pgrx::iter::TableIterator;
use pgrx::*;
use serde::de::Deserialize;
use std::env::var;
use std::error::Error;
use std::time::Duration;

#[cfg(any(test, feature = "pg_test"))]
pub mod testing;

pg_module_magic!();

lazy_static! {
  static ref SERVICE_URL: String =
    var("MERITRANK_SERVICE_URL").unwrap_or("tcp://127.0.0.1:10234".to_string());
  static ref RECV_TIMEOUT_MSEC: u64 = var("MERITRANK_RECV_TIMEOUT_MSEC")
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(10000);
}

const VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
  Some(x) => x,
  None => "dev",
};

//  ================================================================
//
//    SQL
//
//  ================================================================

extension_sql!(
  r#"
DROP FUNCTION IF EXISTS mr_node_score;
DROP FUNCTION IF EXISTS mr_scores;
DROP FUNCTION IF EXISTS mr_graph;
DROP FUNCTION IF EXISTS mr_mutual_scores;
DROP FUNCTION IF EXISTS mr_fetch_new_edges;
DROP FUNCTION IF EXISTS mr_put_edge;
DROP FUNCTION IF EXISTS mr_delete_edge;
DROP FUNCTION IF EXISTS mr_delete_node;
DROP FUNCTION IF EXISTS mr_log_level;
DROP FUNCTION IF EXISTS mr_sync;
"#,
  name = "bootstrap_raw",
  bootstrap,
);

//  ================================================================
//
//    Utils
//
//  ================================================================

fn request_raw(
  payload: Vec<u8>,
  timeout_msec: Option<u64>,
) -> Result<Message, Box<dyn Error + 'static>> {
  let client = Socket::new(Protocol::Req0)?;
  match timeout_msec {
    Some(t) => client.set_opt::<RecvTimeout>(Some(Duration::from_millis(t)))?,
    _ => {},
  }
  client.dial(&SERVICE_URL)?;
  client
    .send(Message::from(payload.as_slice()))
    .map_err(|(_, err)| err)?;
  return Ok(client.recv()?);
}

fn request<T>(
  payload: Vec<u8>,
  timeout_msec: Option<u64>,
) -> Result<T, Box<dyn Error + 'static>>
where
  T: Clone + for<'a> Deserialize<'a>,
{
  let msg = request_raw(payload, timeout_msec)?;
  let slice: &[u8] = msg.as_slice();
  match decode_response(slice) {
    Ok(x) => Ok(x),
    Err(s) => Err(s.into()),
  }
}

fn service_wrapped() -> Result<String, Box<dyn Error + 'static>> {
  let payload =
    rmp_serde::to_vec(&(CMD_VERSION, "", true, rmp_serde::to_vec(&())?))?;

  let response = request_raw(payload, Some(*RECV_TIMEOUT_MSEC))?;
  let s = rmp_serde::from_slice(response.as_slice())?;
  return Ok(s);
}

//  ================================================================
//
//    Immutable functions
//
//  ================================================================

#[pg_extern(immutable)]
fn mr_service_url() -> &'static str {
  &SERVICE_URL
}

#[pg_extern(immutable)]
fn mr_connector() -> &'static str {
  VERSION
}

#[pg_extern(immutable)]
fn mr_service() -> String {
  match service_wrapped() {
    Err(e) => format!("{}", e),
    Ok(s) => s,
  }
}

#[pg_extern(immutable)]
fn mr_node_score(
  src: Option<&str>,
  dst: Option<&str>,
  context: default!(Option<&str>, "''"),
) -> Result<
  TableIterator<
    'static,
    (
      name!(src, String),
      name!(dst, String),
      name!(score_value_of_dst, f64),
      name!(score_value_of_src, f64),
      name!(score_cluster_of_dst, i32),
      name!(score_cluster_of_src, i32),
    ),
  >,
  Box<dyn Error + 'static>,
> {
  let context = context.unwrap_or("");
  let ego = src.expect("src should not be null");
  let target = dst.expect("dst should not be null");

  let args = rmp_serde::to_vec(&(ego, target))?;

  let payload = encode_request(&Command {
    id:       CMD_NODE_SCORE.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  args,
  })?;

  let response: Vec<(String, String, f64, f64, i32, i32)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

fn scores_payload(
  context: Option<&str>,
  src: Option<&str>,
  hide_personal: Option<bool>,
  kind: Option<&str>,
  lt: Option<f64>,
  lte: Option<f64>,
  gt: Option<f64>,
  gte: Option<f64>,
  index: Option<i64>,
  count: Option<i64>,
) -> Result<Vec<u8>, Box<dyn Error + 'static>> {
  let context = context.unwrap_or("");
  let ego = src.expect("ego should not be null");
  let hide_personal = hide_personal.unwrap_or(false);
  let k = kind.unwrap_or("");
  let index = index.unwrap_or(0) as u32;
  let count = count.unwrap_or(i32::MAX as i64) as u32;
  if lt.is_some() && lte.is_some() {
    return Err(Box::from("either lt or lte is allowed!"));
  }
  if gt.is_some() && gte.is_some() {
    return Err(Box::from("either gt or gte is allowed!"));
  }

  let args = rmp_serde::to_vec(&(
    ego,
    k,
    hide_personal,
    lt.unwrap_or(lte.unwrap_or(i32::MAX.into())),
    lte.is_some(),
    gt.unwrap_or(gte.unwrap_or(i32::MIN.into())),
    gte.is_some(),
    index,
    count,
  ))?;

  let payload = encode_request(&Command {
    id:       CMD_SCORES.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  args,
  });

  payload.map_err(|e| e.into())
}

#[pg_extern(immutable)]
fn mr_scores(
  src: Option<&str>,
  hide_personal: default!(Option<bool>, "false"),
  context: default!(Option<&str>, "''"),
  kind: default!(Option<&str>, "''"),
  lt: default!(Option<f64>, "null"),
  lte: default!(Option<f64>, "null"),
  gt: default!(Option<f64>, "null"),
  gte: default!(Option<f64>, "null"),
  index: default!(Option<i64>, "0"),
  count: default!(Option<i64>, "16"),
) -> Result<
  TableIterator<
    'static,
    (
      name!(src, String),
      name!(dst, String),
      name!(score_value_of_dst, f64),
      name!(score_value_of_src, f64),
      name!(score_cluster_of_dst, i32),
      name!(score_cluster_of_src, i32),
    ),
  >,
  Box<dyn Error + 'static>,
> {
  let payload = scores_payload(
    context,
    src,
    hide_personal,
    kind,
    lt,
    lte,
    gt,
    gte,
    index,
    count,
  )?;

  let response: Vec<(String, String, f64, f64, i32, i32)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_graph(
  ego: Option<&str>,
  focus: Option<&str>,
  context: default!(Option<&str>, "''"),
  positive_only: default!(Option<bool>, "false"),
  index: default!(Option<i64>, "0"),
  count: default!(Option<i64>, "16"),
) -> Result<
  TableIterator<
    'static,
    (
      name!(src, String),
      name!(dst, String),
      name!(weight, f64),
      name!(score_value_of_dst, f64),
      name!(score_value_of_ego, f64),
      name!(score_cluster_of_dst, i32),
      name!(score_cluster_of_ego, i32),
    ),
  >,
  Box<dyn Error + 'static>,
> {
  let context = context.unwrap_or("");
  let ego = ego.expect("ego should not be null");
  let focus = focus.expect("focus should not be null");
  let positive_only = positive_only.unwrap_or(false);
  let index = index.unwrap_or(0) as u32;
  let count = count.unwrap_or(i32::MAX as i64) as u32;

  let args = rmp_serde::to_vec(&(ego, focus, positive_only, index, count))?;

  let payload = encode_request(&Command {
    id:       CMD_GRAPH.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  args,
  })?;

  let response: Vec<(String, String, f64, f64, f64, i32, i32)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_neighbors(
  ego: Option<&str>,
  focus: Option<&str>,
  direction: Option<i64>,
  hide_personal: default!(Option<bool>, "false"),
  context: default!(Option<&str>, "''"),
  kind: default!(Option<&str>, "''"),
  lt: default!(Option<f64>, "null"),
  lte: default!(Option<f64>, "null"),
  gt: default!(Option<f64>, "null"),
  gte: default!(Option<f64>, "null"),
  index: default!(Option<i64>, "0"),
  count: default!(Option<i64>, "16"),
) -> Result<
  TableIterator<
    'static,
    (
      name!(src, String),
      name!(dst, String),
      name!(score_value_of_dst, f64),
      name!(score_value_of_src, f64),
      name!(score_cluster_of_dst, i32),
      name!(score_cluster_of_src, i32),
    ),
  >,
  Box<dyn Error + 'static>,
> {
  let ego = ego.expect("ego should not be null");
  let focus = focus.expect("focus should not be null");
  let direction = direction
    .expect("direction should be 0 (all), 1 (outbound) or 2 (inbound)");
  let hide_personal = hide_personal.unwrap_or(false);
  let context = context.unwrap_or("");
  let kind = kind.unwrap_or("");
  let index = index.unwrap_or(0) as u32;
  let count = count.unwrap_or(i32::MAX as i64) as u32;

  if direction != 0 && direction != 1 && direction != 2 {
    return Err(Box::from(
      "direction should be 0 (all), 1 (outbound) or 2 (inbound)",
    ));
  }
  if lt.is_some() && lte.is_some() {
    return Err(Box::from("either lt or lte is allowed!"));
  }
  if gt.is_some() && gte.is_some() {
    return Err(Box::from("either gt or gte is allowed!"));
  }

  let args = rmp_serde::to_vec(&(
    ego,
    focus,
    direction,
    kind,
    hide_personal,
    lt.unwrap_or(lte.unwrap_or(i32::MAX.into())),
    lte.is_some(),
    gt.unwrap_or(gte.unwrap_or(i32::MIN.into())),
    gte.is_some(),
    index,
    count,
  ))?;

  let payload = encode_request(&Command {
    id:       CMD_NEIGHBORS.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  args,
  })?;

  let response: Vec<(String, String, f64, f64, i32, i32)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_nodelist(
  context: default!(Option<&str>, "''")
) -> Result<
  TableIterator<'static, (name!(node, String),)>,
  Box<dyn Error + 'static>,
> {
  let context = context.unwrap_or("");

  let payload = encode_request(&Command {
    id:       CMD_NODE_LIST.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  rmp_serde::to_vec(&())?,
  })?;

  let response: Vec<(String,)> = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_edgelist(
  context: default!(Option<&str>, "''")
) -> Result<
  TableIterator<
    'static,
    (name!(src, String), name!(dst, String), name!(weight, f64)),
  >,
  Box<dyn Error + 'static>,
> {
  let context = context.unwrap_or("");

  let payload = encode_request(&Command {
    id:       CMD_EDGES.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  rmp_serde::to_vec(&())?,
  })?;

  let response: Vec<(String, String, f64)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_connected(
  src: Option<&str>,
  context: default!(Option<&str>, "''"),
) -> Result<
  TableIterator<'static, (name!(src, String), name!(dst, String))>,
  Box<dyn Error + 'static>,
> {
  let context = context.unwrap_or("");
  let ego = src.expect("src should not be null");

  let args = rmp_serde::to_vec(&(ego))?;

  let payload = encode_request(&Command {
    id:       CMD_CONNECTED.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  args,
  })?;

  let response: Vec<(String, String)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_mutual_scores(
  src: Option<&str>,
  context: default!(Option<&str>, "''"),
) -> Result<
  TableIterator<
    'static,
    (
      name!(src, String),
      name!(dst, String),
      name!(score_value_of_dst, f64),
      name!(score_value_of_src, f64),
      name!(score_cluster_of_dst, i32),
      name!(score_cluster_of_src, i32),
    ),
  >,
  Box<dyn Error + 'static>,
> {
  let ego = src.expect("src should not be null");
  let context = context.unwrap_or("");

  let args = rmp_serde::to_vec(&(ego))?;

  let payload = encode_request(&Command {
    id:       CMD_MUTUAL_SCORES.to_string(),
    context:  context.to_string(),
    blocking: true,
    payload:  args,
  })?;

  let response: Vec<(String, String, f64, f64, i32, i32)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::new(response))
}

#[pg_extern]
fn mr_get_new_edges_filter(
  src: Option<&str>
) -> Result<Vec<u8>, Box<dyn Error + 'static>> {
  let src = src.expect("src should not be null");

  let args = rmp_serde::to_vec(&(src))?;

  let payload = encode_request(&Command {
    id:       CMD_READ_NEW_EDGES_FILTER.to_string(),
    context:  "".to_string(),
    blocking: true,
    payload:  args,
  })?;

  let response = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok(response);
}

#[pg_extern(immutable)]
fn mr_sync(
  timeout_msec: default!(Option<i64>, "6000000")
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let timeout_msec = match timeout_msec {
    Some(x) => Some(x as u64),
    _ => None,
  };

  let payload = encode_request(&Command {
    id:       CMD_SYNC.to_string(),
    context:  "".to_string(),
    blocking: true,
    payload:  rmp_serde::to_vec(&())?,
  })?;

  let _: () = request(payload, timeout_msec)?;
  return Ok("Ok");
}

//  ================================================================
//
//    Mutable functions
//
//  ================================================================

#[pg_extern]
fn mr_log_level(
  log_level: default!(Option<i64>, "1")
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let log_level = log_level.unwrap_or(0) as u32;

  let payload = encode_request(&Command {
    id:       CMD_LOG_LEVEL.to_string(),
    context:  "".to_string(),
    blocking: true,
    payload:  rmp_serde::to_vec(&(log_level))?,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_create_context(
  context: Option<&str>
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let context = context.unwrap_or("");

  let payload = encode_request(&Command {
    id:       CMD_CREATE_CONTEXT.to_string(),
    context:  context.to_string(),
    blocking: false,
    payload:  rmp_serde::to_vec(&())?,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_put_edge(
  src: Option<&str>,
  dst: Option<&str>,
  weight: Option<f64>,
  context: default!(Option<&str>, "''"),
  index: default!(Option<i64>, "-1"),
) -> Result<
  TableIterator<
    'static,
    (name!(src, String), name!(dst, String), name!(weight, f64)),
  >,
  Box<dyn Error + 'static>,
> {
  let context = context.unwrap_or("");
  let src = src.expect("src should not be null");
  let dest = dst.expect("dst should not be null");
  let weight = weight.expect("weight should not be null");
  let index = index.expect("index should not be null");

  let args = rmp_serde::to_vec(&(src, dest, weight, index))?;

  let payload = encode_request(&Command {
    id:       CMD_PUT_EDGE.to_string(),
    context:  context.to_string(),
    blocking: false,
    payload:  args,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  Ok(TableIterator::once((
    src.to_string(),
    dest.to_string(),
    weight,
  )))
}

#[pg_extern]
fn mr_delete_edge(
  src: Option<&str>,
  dst: Option<&str>,
  context: default!(Option<&str>, "''"),
  index: default!(Option<i64>, "-1"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let context = context.unwrap_or("");
  let ego = src.expect("src should not be null");
  let dst = dst.expect("dst should not be null");
  let index = index.expect("index should not be null");

  let args = rmp_serde::to_vec(&(ego, dst, index))?;

  let payload = encode_request(&Command {
    id:       CMD_DELETE_EDGE.to_string(),
    context:  context.to_string(),
    blocking: false,
    payload:  args,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_delete_node(
  src: Option<&str>,
  context: default!(Option<&str>, "''"),
  index: default!(Option<i64>, "-1"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let context = context.unwrap_or("");
  let ego = src.expect("src should not be null");
  let index = index.expect("index should not be null");

  let args = rmp_serde::to_vec(&(ego, index))?;

  let payload = encode_request(&Command {
    id:       CMD_DELETE_NODE.to_string(),
    context:  context.to_string(),
    blocking: false,
    payload:  args,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_set_zero_opinion(
  node: Option<&str>,
  score: Option<f64>,
  context: default!(Option<&str>, "''"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let context = context.unwrap_or("");
  let node = node.expect("node should not be null");
  let score = score.expect("score should not be null");

  let args = rmp_serde::to_vec(&(node, score))?;

  let payload = encode_request(&Command {
    id:       CMD_SET_ZERO_OPINION.to_string(),
    context:  context.to_string(),
    blocking: false,
    payload:  args,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_set_new_edges_filter(
  src: Option<&str>,
  filter: Option<Vec<u8>>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let src = src.expect("src should not be null");
  let filter = filter.expect("filter should not be null");

  let args = rmp_serde::to_vec(&(src, filter))?;

  let payload = encode_request(&Command {
    id:       CMD_WRITE_NEW_EDGES_FILTER.to_string(),
    context:  "".to_string(),
    blocking: false,
    payload:  args,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_fetch_new_edges(
  src: Option<&str>,
  prefix: default!(Option<&str>, "''"),
) -> Result<
  TableIterator<
    'static,
    (
      name!(src, String),
      name!(dst, String),
      name!(score_value_of_dst, f64),
      name!(score_value_of_src, f64),
      name!(score_cluster_of_dst, i32),
      name!(score_cluster_of_src, i32),
    ),
  >,
  Box<dyn Error + 'static>,
> {
  let src = src.expect("src should not be null");
  let prefix = prefix.unwrap_or("");

  let args = rmp_serde::to_vec(&(src, prefix))?;

  let payload = encode_request(&Command {
    id:       CMD_FETCH_NEW_EDGES.to_string(),
    context:  "".to_string(),
    blocking: true,
    payload:  args,
  })?;

  let response: Vec<(String, f64, f64, i32, i32)> =
    request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  let edges: Vec<(String, String, f64, f64, i32, i32)> = response
    .iter()
    .map(
      |(
        dst,
        score_value_of_dst,
        score_value_of_src,
        score_cluster_of_dst,
        score_cluster_of_src,
      )| {
        (
          src.to_string(),
          dst.clone(),
          *score_value_of_dst,
          *score_value_of_src,
          *score_cluster_of_dst,
          *score_cluster_of_src,
        )
      },
    )
    .collect();
  Ok(TableIterator::new(edges))
}

#[pg_extern]
fn mr_reset() -> Result<&'static str, Box<dyn Error + 'static>> {
  let payload = encode_request(&Command {
    id:       CMD_RESET.to_string(),
    context:  "".to_string(),
    blocking: false,
    payload:  rmp_serde::to_vec(&())?,
  })?;

  let _: () = request(payload, Some(*RECV_TIMEOUT_MSEC))?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_zerorec(
  blocking: default!(Option<bool>, "true"),
  timeout_msec: default!(Option<i64>, "6000000"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let blocking = blocking.unwrap_or(true);
  let timeout_msec = match timeout_msec {
    Some(x) => Some(x as u64),
    _ => None,
  };

  let payload = encode_request(&Command {
    id: CMD_RECALCULATE_ZERO.to_string(),
    context: "".to_string(),
    blocking,
    payload: rmp_serde::to_vec(&())?,
  })?;

  let _: () = request(payload, timeout_msec)?;
  return Ok("Ok");
}

#[pg_extern]
fn mr_recalculate_clustering(
  blocking: default!(Option<bool>, "true"),
  timeout_msec: default!(Option<i64>, "6000000"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let blocking = blocking.unwrap_or(true);
  let timeout_msec = match timeout_msec {
    Some(x) => Some(x as u64),
    _ => None,
  };

  let payload = encode_request(&Command {
    id: CMD_RECALCULATE_CLUSTERING.to_string(),
    context: "".to_string(),
    blocking,
    payload: rmp_serde::to_vec(&())?,
  })?;

  let _: () = request(payload, timeout_msec)?;
  return Ok("Ok");
}

//  ================================================================
//
//    Tests
//
//  ================================================================

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
  use super::testing::*;
  use meritrank_service::protocol::*;
  use pgrx::prelude::*;
  use std::time::SystemTime;

  #[pg_test]
  fn sync_deadlock() {
    for _ in 0..3000 {
      let _ = crate::mr_reset().unwrap();
      let _ =
        crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), None, Some(-1))
          .unwrap();
      let _ =
        crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
          .unwrap();
      let _ =
        crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), None, Some(-1))
          .unwrap();
      let _ = crate::mr_sync(Some(1000)).unwrap();
    }
  }

  #[pg_test]
  fn zerorec_graph_all() {
    let _ = crate::mr_reset().unwrap();

    put_testing_edges();

    let _ = crate::mr_zerorec(Some(true), None).unwrap();

    let res = crate::mr_graph(
      Some("Uadeb43da4abb"),
      Some("B7f628ad203b5"),
      None,
      Some(false),
      None,
      None,
    )
    .unwrap();

    let n = res.count();

    assert!(n > 1);
    assert!(n < 5);
  }

  #[pg_test]
  fn recalculate_clustering() {
    let _ = crate::mr_reset().unwrap();

    put_testing_edges();

    let _ = crate::mr_recalculate_clustering(Some(true), None).unwrap();
  }

  #[pg_test]
  fn zerorec_graph_positive_only() {
    let _ = crate::mr_reset().unwrap();

    put_testing_edges();

    let _ = crate::mr_zerorec(Some(true), None).unwrap();

    let res = crate::mr_graph(
      Some("Uadeb43da4abb"),
      Some("B7f628ad203b5"),
      None,
      Some(true),
      None,
      None,
    )
    .unwrap();

    let n = res.count();

    assert!(n > 1);
    assert!(n < 5);
  }

  #[pg_test]
  fn zerorec_reset_perf() {
    let _ = crate::mr_reset().unwrap();

    put_testing_edges();
    let _ = crate::mr_zerorec(Some(true), None).unwrap();
    let _ = crate::mr_reset().unwrap();
    put_testing_edges();
    let _ = crate::mr_create_context(Some("X")).unwrap();
    let _ = crate::mr_create_context(Some("Y")).unwrap();
    let _ = crate::mr_create_context(Some("Z")).unwrap();
    let _ = crate::mr_zerorec(Some(true), None).unwrap();

    let begin = SystemTime::now();
    let get_time =
      || SystemTime::now().duration_since(begin).unwrap().as_millis();

    let _ = crate::mr_graph(
      Some("Uadeb43da4abb"),
      Some("U000000000000"),
      None,
      Some(true),
      None,
      None,
    )
    .unwrap();

    assert!(get_time() < 200);
  }

  #[pg_test]
  fn zerorec_scores() {
    let _ = crate::mr_reset().unwrap();

    put_testing_edges();

    let _ = crate::mr_zerorec(Some(true), None).unwrap();

    let res = crate::mr_scores(
      Some("Uadeb43da4abb"),
      Some(true),
      Some(""),
      Some("B"),
      None,
      None,
      Some(0.0),
      None,
      Some(0),
      Some(i32::MAX as i64),
    )
    .unwrap();

    let n = res.count();

    assert!(n > 5);
    assert!(n < 80);
  }

  #[pg_test]
  fn service() {
    let ver = crate::mr_service();

    //  check if ver is in form "X.Y.Z"

    let nums: Vec<&str> = ver.split(".").collect();

    assert_eq!(nums.len(), 3);
    let _ = nums[0].parse::<u32>().unwrap();
    let _ = nums[1].parse::<u32>().unwrap();
  }

  #[pg_test]
  fn edge_uncontexted() {
    let _ = crate::mr_reset().unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
        .unwrap();

    let n = res
      .map(|x| {
        let (ego, target, score) = x;
        assert_eq!(ego, "U1");
        assert_eq!(target, "U2");
        assert_eq!(score, 1.0);
      })
      .count();

    assert_eq!(n, 1);
  }

  #[pg_test]
  fn edge_contexted() {
    let _ = crate::mr_reset().unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_put_edge(
      Some("U1"),
      Some("U2"),
      Some(1.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();

    let n = res
      .map(|x| {
        let (ego, target, score) = x;
        assert_eq!(ego, "U1");
        assert_eq!(target, "U2");
        assert_eq!(score, 1.0);
      })
      .count();

    assert_eq!(n, 1);
  }

  #[pg_test]
  fn create_context() {
    let _ = crate::mr_reset().unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_create_context(Some("X"));
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_edgelist(Some("X")).unwrap();

    let n = res
      .map(|x| {
        let (ego, target, score) = x;
        assert_eq!(ego, "U1");
        assert_eq!(target, "U2");
        assert!(score > 0.99);
        assert!(score < 1.01);
      })
      .count();

    assert_eq!(n, 1);
  }

  #[pg_test]
  fn null_context_is_sum() {
    let _ = crate::mr_reset().unwrap();

    let _ = crate::mr_put_edge(
      Some("B1"),
      Some("B2"),
      Some(1.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("B1"),
      Some("B2"),
      Some(2.0),
      Some("Y"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_edgelist(None).unwrap();

    let n = res
      .map(|x| {
        let (ego, target, score) = x;
        assert_eq!(ego, "B1");
        assert_eq!(target, "B2");
        assert!(score > 2.99);
        assert!(score < 3.01);
      })
      .count();

    assert_eq!(n, 1);
  }

  #[pg_test]
  fn delete_contexted_edge() {
    let _ = crate::mr_reset().unwrap();

    let _ = crate::mr_put_edge(
      Some("B1"),
      Some("B2"),
      Some(1.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("B1"),
      Some("B2"),
      Some(2.0),
      Some("Y"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_delete_edge(Some("B1"), Some("B2"), Some("X"), Some(-1))
      .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    //  We should still have "Y" edge.
    let res = crate::mr_edgelist(None).unwrap();

    let n = res
      .map(|x| {
        let (ego, target, score) = x;
        assert_eq!(ego, "B1");
        assert_eq!(target, "B2");
        assert_eq!(score, 2.0);
      })
      .count();

    assert_eq!(n, 1);
  }

  #[pg_test]
  fn delete_nodes() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("B1"), Some("B2"), Some(1.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_delete_node(Some("B1"), None, Some(-1)).unwrap();
    let _ = crate::mr_delete_node(Some("B2"), None, Some(-1)).unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_edgelist(None).unwrap();

    assert_eq!(res.count(), 0);
  }

  #[pg_test]
  fn null_context_invariant() {
    let _ = crate::mr_reset().unwrap();

    let _ = crate::mr_put_edge(
      Some("B1"),
      Some("B2"),
      Some(1.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("B1"),
      Some("B2"),
      Some(2.0),
      Some("Y"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    //  Delete and put back again.
    let _ = crate::mr_delete_edge(Some("B1"), Some("B2"), Some("X"), Some(-1));
    let _ = crate::mr_put_edge(
      Some("B1"),
      Some("B2"),
      Some(1.0),
      Some("X"),
      Some(-1),
    );
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_edgelist(None).unwrap();

    let n = res
      .map(|x| {
        let (ego, target, score) = x;
        assert_eq!(ego, "B1");
        assert_eq!(target, "B2");
        assert_eq!(score, 3.0);
      })
      .count();

    assert_eq!(n, 1);
  }

  #[pg_test]
  fn node_score_context() {
    let _ = crate::mr_reset().unwrap();

    let _ = crate::mr_put_edge(
      Some("U1"),
      Some("U2"),
      Some(2.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("U1"),
      Some("U3"),
      Some(1.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("U3"),
      Some("U2"),
      Some(3.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_node_score(Some("U1"), Some("U2"), Some("X")).unwrap();

    let n = res
      .map(|x| {
        let (ego, dst, score_dst, score_ego, _, _) = x;
        assert_eq!(ego, "U1");
        assert_eq!(dst, "U2");
        assert!(score_dst > 0.3);
        assert!(score_dst < 0.45);
        assert!(score_ego > -0.1);
        assert!(score_ego < 0.1);
      })
      .count();

    assert_eq!(n, 1);
  }

  #[pg_test]
  fn scores_null_context() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), Some(""), Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), Some(""), Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), Some(""), Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res: Vec<_> = crate::mr_scores(
      Some("U1"),
      Some(false),
      Some(""),
      Some("U"),
      Some(10.0),
      None,
      Some(0.0),
      None,
      None,
      None,
    )
    .unwrap()
    .collect();

    assert_eq!(res.len(), 3);

    for x in res {
      assert_eq!(x.0, "U1");

      match x.1.as_str() {
        "U1" => {
          assert!(x.2 > 0.2);
          assert!(x.2 < 0.5);
        },

        "U2" => {
          assert!(x.2 > 0.1);
          assert!(x.2 < 0.4);
        },

        "U3" => {
          assert!(x.2 > 0.2);
          assert!(x.2 < 0.5);
        },

        _ => assert!(false),
      }
    }
  }

  #[pg_test]
  fn scores_context() {
    let _ = crate::mr_reset().unwrap();

    let _ = crate::mr_put_edge(
      Some("U1"),
      Some("U2"),
      Some(2.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("U1"),
      Some("U3"),
      Some(1.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("U2"),
      Some("U3"),
      Some(3.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res: Vec<_> = crate::mr_scores(
      Some("U1"),
      Some(false),
      Some("X"),
      Some("U"),
      Some(10.0),
      None,
      Some(0.0),
      None,
      None,
      None,
    )
    .unwrap()
    .collect();

    assert_eq!(res.len(), 3);

    for x in res {
      assert_eq!(x.0, "U1");

      match x.1.as_str() {
        "U1" => {
          assert!(x.2 > 0.2);
          assert!(x.2 < 0.5);
        },

        "U2" => {
          assert!(x.2 > 0.1);
          assert!(x.2 < 0.4);
        },

        "U3" => {
          assert!(x.2 > 0.2);
          assert!(x.2 < 0.5);
        },

        _ => assert!(false),
      }
    }
  }

  #[pg_test]
  fn scores_defaults() {
    let _ = crate::mr_reset().unwrap();

    let _ = crate::mr_put_edge(
      Some("U1"),
      Some("U2"),
      Some(2.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("U1"),
      Some("U3"),
      Some(1.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_put_edge(
      Some("U2"),
      Some("U3"),
      Some(3.0),
      Some("X"),
      Some(-1),
    )
    .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res: Vec<_> = crate::mr_scores(
      Some("U1"),
      Some(false),
      Some("X"),
      Some("U"),
      None,
      None,
      None,
      None,
      None,
      None,
    )
    .unwrap()
    .collect();

    assert_eq!(res.len(), 3);

    for x in res {
      assert_eq!(x.0, "U1");

      match x.1.as_str() {
        "U1" => {
          assert!(x.2 > 0.2);
          assert!(x.2 < 0.5);
        },

        "U2" => {
          assert!(x.2 > 0.1);
          assert!(x.2 < 0.4);
        },

        "U3" => {
          assert!(x.2 > 0.2);
          assert!(x.2 < 0.5);
        },

        _ => assert!(false),
      }
    }
  }

  #[pg_test]
  fn nodelist() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res: Vec<_> = crate::mr_nodelist(None).unwrap().collect();

    assert_eq!(res.len(), 3);

    for x in res {
      assert!(x.0 == "U1" || x.0 == "U2" || x.0 == "U3");
    }
  }

  #[pg_test]
  fn connected() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res: Vec<_> = crate::mr_connected(Some("U1"), None).unwrap().collect();

    assert_eq!(res.len(), 2);

    for x in res {
      assert_eq!(x.0, "U1");
      assert!(x.1 == "U2" || x.1 == "U3");
    }
  }

  #[pg_test]
  fn mutual_scores() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(3.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("U1"), Some(2.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("U3"), Some(4.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U3"), Some("U1"), Some(3.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U3"), Some("U2"), Some(2.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res: Vec<_> =
      crate::mr_mutual_scores(Some("U1"), None).unwrap().collect();

    assert_eq!(res.len(), 3);

    let mut u1 = true;
    let mut u2 = true;
    let mut u3 = true;

    for x in res.iter() {
      assert_eq!(x.0, "U1");

      match x.1.as_str() {
        "U1" => {
          assert!(res[0].2 > 0.15);
          assert!(res[0].2 < 0.45);
          assert!(res[0].3 > 0.15);
          assert!(res[0].3 < 0.45);
          assert!(u1);
          u1 = false;
        },

        "U2" => {
          assert!(res[1].2 > 0.15);
          assert!(res[1].2 < 0.45);
          assert!(res[1].3 > 0.05);
          assert!(res[1].3 < 0.45);
          assert!(u2);
          u2 = false;
        },

        "U3" => {
          assert!(res[2].2 > 0.05);
          assert!(res[2].2 < 0.45);
          assert!(res[2].3 > 0.15);
          assert!(res[2].3 < 0.45);
          assert!(u3);
          u3 = false;
        },

        _ => {
          assert!(false);
        },
      };
    }
  }

  #[pg_test]
  fn new_edges_fetch() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
        .unwrap();

    assert_eq!(
      crate::mr_fetch_new_edges(Some("U1"), Some("B"))
        .unwrap()
        .count(),
      0
    );

    let _ =
      crate::mr_put_edge(Some("U1"), Some("B3"), Some(2.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("B4"), Some(3.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_fetch_new_edges(Some("U1"), Some("B")).unwrap();

    let beacons: Vec<_> = res.collect();

    assert_eq!(beacons.len(), 2);
    assert_eq!(beacons[0].1, "B3");
    assert_eq!(beacons[1].1, "B4");

    assert_eq!(
      crate::mr_fetch_new_edges(Some("U1"), Some("B"))
        .unwrap()
        .count(),
      0
    );
  }

  #[pg_test]
  fn new_edges_filter() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
        .unwrap();

    assert_eq!(
      crate::mr_fetch_new_edges(Some("U1"), Some("B"))
        .unwrap()
        .count(),
      0
    );

    let _ =
      crate::mr_put_edge(Some("U1"), Some("B3"), Some(2.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("B4"), Some(3.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let filter: Vec<u8> = crate::mr_get_new_edges_filter(Some("U1")).unwrap();

    let res = crate::mr_fetch_new_edges(Some("U1"), Some("B")).unwrap();

    let beacons: Vec<_> = res.collect();

    assert_eq!(beacons.len(), 2);
    assert_eq!(beacons[0].1, "B3");
    assert_eq!(beacons[1].1, "B4");

    let _ = crate::mr_set_new_edges_filter(Some("U1"), Some(filter)).unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res = crate::mr_fetch_new_edges(Some("U1"), Some("B")).unwrap();

    let beacons: Vec<_> = res.collect();

    assert_eq!(beacons.len(), 2);
    assert_eq!(beacons[0].1, "B3");
    assert_eq!(beacons[1].1, "B4");
  }

  #[pg_test]
  fn five_scores_clustering() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(5.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U4"), Some(2.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U1"), Some("U5"), Some(3.0), None, Some(-1))
        .unwrap();
    let _ =
      crate::mr_put_edge(Some("U2"), Some("U1"), Some(4.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let res: Vec<_> = crate::mr_scores(
      Some("U1"),
      Some(true),
      Some(""),
      Some(""),
      None,
      None,
      Some(0.0),
      None,
      Some(0),
      Some(i32::MAX as i64),
    )
    .unwrap()
    .collect();

    assert_eq!(res.len(), 5);

    assert!(res[0].4 <= 100);
    assert!(res[0].4 >= 40);

    assert!(res[1].4 <= 100);
    assert!(res[1].4 >= 20);

    assert!(res[2].4 <= 100);
    assert!(res[2].4 >= 1);

    assert!(res[3].4 <= 80);
    assert!(res[3].4 >= 1);

    assert!(res[4].4 <= 60);
    assert!(res[4].4 >= 1);
  }

  #[pg_test]
  fn set_zero_opinion() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(5.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let s0: Vec<_> = crate::mr_node_score(Some("U1"), Some("U2"), None)
      .unwrap()
      .collect();

    let _ = crate::mr_set_zero_opinion(Some("U2"), Some(-10.0), None);
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let s1: Vec<_> = crate::mr_node_score(Some("U1"), Some("U2"), None)
      .unwrap()
      .collect();

    assert_ne!(s0[0].2, s1[0].2);
  }

  #[pg_test]
  fn neighbors_inbound() {
    let _ = crate::mr_reset().unwrap();

    let _ =
      crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
        .unwrap();
    let _ = crate::mr_sync(Some(1000)).unwrap();

    let neighbors: Vec<_> = crate::mr_neighbors(
      Some("U1"),
      Some("U2"),
      Some(NEIGHBORS_INBOUND),
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
    )
    .unwrap()
    .collect();

    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].0, "U1");
    assert_eq!(neighbors[0].1, "U1");
  }
}

#[cfg(test)]
pub mod pg_test {
  pub fn setup(_options: Vec<&str>) {}

  pub fn postgresql_conf_options() -> Vec<&'static str> {
    vec![]
  }
}
