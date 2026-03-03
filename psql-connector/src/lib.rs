mod rpc;
mod types;

#[cfg(any(test, feature = "pg_test"))]
pub mod testing;

use rpc::*;
use pgrx::iter::TableIterator;
use pgrx::*;
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

//  D3 (JOURNAL): Return connector version; no network call needed.
#[pg_extern(immutable)]
fn mr_service() -> &'static str {
  VERSION
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
  Ok(TableIterator::new(new_node_score(ego, target, ctx(context))?))
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
  Ok(TableIterator::new(new_scores(
    ego,
    hide_personal.unwrap_or(false),
    ctx(context),
    kind.unwrap_or(""),
    lt,
    lte,
    gt,
    gte,
    index.unwrap_or(0) as u32,
    count.unwrap_or(i32::MAX as i64) as u32,
  )?))
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
  Ok(TableIterator::new(new_graph(
    ego,
    focus,
    ctx(context),
    positive_only.unwrap_or(false),
    index.unwrap_or(0) as u64,
    count.unwrap_or(i32::MAX as i64) as u64,
  )?))
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
  Ok(TableIterator::new(new_neighbors(
    ego,
    focus,
    direction,
    hide_personal.unwrap_or(false),
    ctx(context),
    kind.unwrap_or(""),
    lt,
    lte,
    gt,
    gte,
    index.unwrap_or(0) as u32,
    count.unwrap_or(i32::MAX as i64) as u32,
  )?))
}

#[pg_extern(immutable)]
fn mr_nodelist(
  context: default!(Option<&str>, "''"),
) -> Result<
  TableIterator<'static, (name!(node, String),)>,
  Box<dyn Error + 'static>,
> {
  Ok(TableIterator::new(new_node_list(ctx(context))?))
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
  Ok(TableIterator::new(new_edgelist(ctx(context))?))
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
  Ok(TableIterator::new(new_connected(ego, ctx(context))?))
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
  Ok(TableIterator::new(new_mutual_scores(ego, ctx(context))?))
}

#[pg_extern]
fn mr_get_new_edges_filter(
  src: Option<&str>,
) -> Result<Vec<u8>, Box<dyn Error + 'static>> {
  let src = require(src, "src")?;
  new_get_new_edges_filter(src)
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
  new_sync(timeout_u64(timeout_msec))
}

//  D3 (JOURNAL): mr_log_level is a no-op in the new protocol; log level is
//  controlled server-side via environment variables.
#[pg_extern]
fn mr_log_level(
  _log_level: default!(Option<i64>, "1"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  Ok("Ok")
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
  new_create_context(ctx(context))
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
  let c = ctx(context);
  let src = require(src, "src")?;
  let dest = require(dst, "dst")?;
  let weight = require(weight, "weight")?;
  let index = require(index, "index")?;
  new_put_edge(src, dest, weight, c, index)?;
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
  let c = ctx(context);
  let ego = require(src, "src")?;
  let dst = require(dst, "dst")?;
  let index = require(index, "index")?;
  new_delete_edge(ego, dst, c, index)
}

#[pg_extern]
fn mr_delete_node(
  src: Option<&str>,
  context: default!(Option<&str>, "''"),
  index: default!(Option<i64>, "-1"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let c = ctx(context);
  let ego = require(src, "src")?;
  let index = require(index, "index")?;
  new_delete_node(ego, c, index)
}

#[pg_extern]
fn mr_set_zero_opinion(
  node: Option<&str>,
  score: Option<f64>,
  context: default!(Option<&str>, "''"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let c = ctx(context);
  let node = require(node, "node")?;
  let score = require(score, "score")?;
  new_set_zero_opinion(node, score, c)
}

#[pg_extern]
fn mr_set_new_edges_filter(
  src: Option<&str>,
  filter: Option<Vec<u8>>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let src = require(src, "src")?;
  let filter = require(filter, "filter")?;
  new_set_new_edges_filter(src, filter)
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
  Ok(TableIterator::new(new_fetch_new_edges(src, prefix)?))
}

#[pg_extern]
fn mr_reset() -> Result<&'static str, Box<dyn Error + 'static>> {
  new_reset()
}

#[pg_extern]
fn mr_zerorec(
  //  D6 (JOURNAL): blocking flag is no longer relevant in the new protocol;
  //  accepted for backward compatibility but ignored.
  _blocking: default!(Option<bool>, "true"),
  timeout_msec: default!(Option<i64>, "6000000"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  new_zerorec(timeout_u64(timeout_msec))
}

#[pg_extern]
fn mr_recalculate_clustering(
  _blocking: default!(Option<bool>, "true"),
  timeout_msec: default!(Option<i64>, "6000000"),
) -> Result<&'static str, Box<dyn Error + 'static>> {
  new_recalculate_clustering(timeout_u64(timeout_msec))
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
