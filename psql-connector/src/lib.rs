mod rpc;
mod types;

#[cfg(any(test, feature = "pg_test"))]
pub mod testing;

use meritrank_service::legacy_protocol::*;
use pgrx::iter::TableIterator;
use pgrx::*;
use rpc::*;
use std::error::Error;
use types::*;

pg_module_magic!();

const VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
  Some(x) => x,
  None => "dev",
};

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
    Err(e) => format!("{e}"),
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
  let ego = require(src, "src")?;
  let target = require(dst, "dst")?;
  let response: Vec<(String, String, f64, f64, i32, i32)> =
    call(CMD_NODE_SCORE, ctx(context), true, (ego, target))?;
  Ok(TableIterator::new(response))
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
  let ego = require(src, "ego")?;
  let (lt_val, lte_flag, gt_val, gte_flag) = validate_bounds(lt, lte, gt, gte)?;
  let response: Vec<(String, String, f64, f64, i32, i32)> = call(
    CMD_SCORES,
    ctx(context),
    true,
    (
      ego,
      kind.unwrap_or(""),
      hide_personal.unwrap_or(false),
      lt_val,
      lte_flag,
      gt_val,
      gte_flag,
      index.unwrap_or(0) as u32,
      count.unwrap_or(i32::MAX as i64) as u32,
    ),
  )?;
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
  let ego = require(ego, "ego")?;
  let focus = require(focus, "focus")?;
  let response: Vec<(String, String, f64, f64, f64, i32, i32)> = call(
    CMD_GRAPH,
    ctx(context),
    true,
    (
      ego,
      focus,
      positive_only.unwrap_or(false),
      index.unwrap_or(0) as u32,
      count.unwrap_or(i32::MAX as i64) as u32,
    ),
  )?;
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
  let ego = require(ego, "ego")?;
  let focus = require(focus, "focus")?;
  let direction = match direction {
    Some(d @ (0 | 1 | 2)) => d,
    _ => {
      return Err(
        "direction should be 0 (all), 1 (outbound) or 2 (inbound)".into(),
      )
    },
  };
  let (lt_val, lte_flag, gt_val, gte_flag) = validate_bounds(lt, lte, gt, gte)?;
  let response: Vec<(String, String, f64, f64, i32, i32)> = call(
    CMD_NEIGHBORS,
    ctx(context),
    true,
    (
      ego,
      focus,
      direction,
      kind.unwrap_or(""),
      hide_personal.unwrap_or(false),
      lt_val,
      lte_flag,
      gt_val,
      gte_flag,
      index.unwrap_or(0) as u32,
      count.unwrap_or(i32::MAX as i64) as u32,
    ),
  )?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_nodelist(
  context: default!(Option<&str>, "''"),
) -> Result<
  TableIterator<'static, (name!(node, String),)>,
  Box<dyn Error + 'static>,
> {
  let response: Vec<(String,)> = call(CMD_NODE_LIST, ctx(context), true, ())?;
  Ok(TableIterator::new(response))
}

#[pg_extern(immutable)]
fn mr_edgelist(
  context: default!(Option<&str>, "''"),
) -> Result<
  TableIterator<
    'static,
    (name!(src, String), name!(dst, String), name!(weight, f64)),
  >,
  Box<dyn Error + 'static>,
> {
  let response: Vec<(String, String, f64)> =
    call(CMD_EDGES, ctx(context), true, ())?;
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
  let ego = require(src, "src")?;
  let response: Vec<(String, String)> =
    call(CMD_CONNECTED, ctx(context), true, ego)?;
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
  let ego = require(src, "src")?;
  let response: Vec<(String, String, f64, f64, i32, i32)> =
    call(CMD_MUTUAL_SCORES, ctx(context), true, ego)?;
  Ok(TableIterator::new(response))
}

#[pg_extern]
fn mr_get_new_edges_filter(
  src: Option<&str>,
) -> Result<Vec<u8>, Box<dyn Error + 'static>> {
  let src = require(src, "src")?;
  call(CMD_READ_NEW_EDGES_FILTER, "", true, src)
}

//  ================================================================
//
//    Sync & admin
//
//  ================================================================

#[pg_extern(immutable)]
fn mr_sync(
  timeout_msec: default!(Option<i64>, "6000000"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  call_void(CMD_SYNC, "", true, (), timeout_u64(timeout_msec))
}

#[pg_extern]
fn mr_log_level(
  log_level: default!(Option<i64>, "1"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let log_level = log_level.unwrap_or(0) as u32;
  call_void(CMD_LOG_LEVEL, "", true, log_level, Some(*RECV_TIMEOUT_MSEC))
}

//  ================================================================
//
//    Mutable functions
//
//  ================================================================

#[pg_extern]
fn mr_create_context(
  context: Option<&str>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  call_void(
    CMD_CREATE_CONTEXT,
    ctx(context),
    false,
    (),
    Some(*RECV_TIMEOUT_MSEC),
  )
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
  let ctx = ctx(context);
  let src = require(src, "src")?;
  let dest = require(dst, "dst")?;
  let weight = require(weight, "weight")?;
  let index = require(index, "index")?;
  call_void(
    CMD_PUT_EDGE,
    ctx,
    false,
    (src, dest, weight, index),
    Some(*RECV_TIMEOUT_MSEC),
  )?;
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
  let ctx = ctx(context);
  let ego = require(src, "src")?;
  let dst = require(dst, "dst")?;
  let index = require(index, "index")?;
  call_void(
    CMD_DELETE_EDGE,
    ctx,
    false,
    (ego, dst, index),
    Some(*RECV_TIMEOUT_MSEC),
  )
}

#[pg_extern]
fn mr_delete_node(
  src: Option<&str>,
  context: default!(Option<&str>, "''"),
  index: default!(Option<i64>, "-1"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let ctx = ctx(context);
  let ego = require(src, "src")?;
  let index = require(index, "index")?;
  call_void(
    CMD_DELETE_NODE,
    ctx,
    false,
    (ego, index),
    Some(*RECV_TIMEOUT_MSEC),
  )
}

#[pg_extern]
fn mr_set_zero_opinion(
  node: Option<&str>,
  score: Option<f64>,
  context: default!(Option<&str>, "''"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let ctx = ctx(context);
  let node = require(node, "node")?;
  let score = require(score, "score")?;
  call_void(
    CMD_SET_ZERO_OPINION,
    ctx,
    false,
    (node, score),
    Some(*RECV_TIMEOUT_MSEC),
  )
}

#[pg_extern]
fn mr_set_new_edges_filter(
  src: Option<&str>,
  filter: Option<Vec<u8>>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let src = require(src, "src")?;
  let filter = require(filter, "filter")?;
  call_void(
    CMD_WRITE_NEW_EDGES_FILTER,
    "",
    false,
    (src, filter),
    Some(*RECV_TIMEOUT_MSEC),
  )
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
  let src = require(src, "src")?;
  let prefix = prefix.unwrap_or("");
  let response: Vec<(String, f64, f64, i32, i32)> =
    call(CMD_FETCH_NEW_EDGES, "", true, (src, prefix))?;
  let edges: Vec<_> = response
    .into_iter()
    .map(|(dst, sv_dst, sv_src, sc_dst, sc_src)| {
      (src.to_string(), dst, sv_dst, sv_src, sc_dst, sc_src)
    })
    .collect();
  Ok(TableIterator::new(edges))
}

#[pg_extern]
fn mr_reset() -> Result<&'static str, Box<dyn Error + 'static>> {
  call_void(CMD_RESET, "", false, (), Some(*RECV_TIMEOUT_MSEC))
}

#[pg_extern]
fn mr_zerorec(
  blocking: default!(Option<bool>, "true"),
  timeout_msec: default!(Option<i64>, "6000000"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  call_void(
    CMD_RECALCULATE_ZERO,
    "",
    blocking.unwrap_or(true),
    (),
    timeout_u64(timeout_msec),
  )
}

#[pg_extern]
fn mr_recalculate_clustering(
  blocking: default!(Option<bool>, "true"),
  timeout_msec: default!(Option<i64>, "6000000"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  call_void(
    CMD_RECALCULATE_CLUSTERING,
    "",
    blocking.unwrap_or(true),
    (),
    timeout_u64(timeout_msec),
  )
}

//  ================================================================
//
//    Tests
//
//  ================================================================

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
  include!("tests.rs");
}

#[cfg(test)]
pub mod pg_test {
  pub fn setup(_options: Vec<&str>) {}

  pub fn postgresql_conf_options() -> Vec<&'static str> {
    vec![]
  }
}
