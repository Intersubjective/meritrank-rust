use crate::utils::log::*;

use std::env::*;
use std::fmt::*;
use std::str::FromStr;

#[derive(Clone)]
pub struct Settings {
  pub legacy_server_port: u16,
  pub legacy_server_num_threads: usize,
  pub legacy_connections_mode: bool,
  pub server_address: String,
  pub server_port: u16,
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
  pub sleep_duration_after_publish_ms: u64,
  pub subgraph_queue_capacity: usize,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      legacy_server_port: 8040,
      legacy_server_num_threads: 4,
      legacy_connections_mode: true,
      server_address: "127.0.0.1".into(),
      server_port: 8080,
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
      sleep_duration_after_publish_ms: 10,
      subgraph_queue_capacity: 1024,
    }
  }
}

enum AllErrors {
  Var,
  Parse(String),
}

impl Display for AllErrors {
  fn fmt(
    &self,
    f: &mut Formatter,
  ) -> Result {
    match self {
      AllErrors::Var => Ok(()),
      AllErrors::Parse(name) => write!(f, "Failed to parse: {}", name),
    }
  }
}

fn load_var<T>(
  name: &str,
  val: &mut T,
) where
  T: FromStr,
{
  var(name)
    .map_err(|_| AllErrors::Var)
    .and_then(|a| a.parse().map_err(|_| AllErrors::Parse(name.into())))
    .map(|x| *val = x)
    .unwrap_or_else(|e| {
      if let AllErrors::Parse(message) = e {
        log_error!("{}", message);
      }
    });
}

pub fn load_from_env() -> Settings {
  let mut s = Settings::default();

  load_var("MERITRANK_LEGACY_SERVER_PORT", &mut s.legacy_server_port);
  load_var(
    "MERITRANK_LEGACY_SERVER_NUM_THREADS",
    &mut s.legacy_server_num_threads,
  );
  load_var(
    "MERITRANK_LEGACY_CONNECTIONS_MODE",
    &mut s.legacy_connections_mode,
  );
  load_var("MERITRANK_SERVER_ADDRESS", &mut s.server_address);
  load_var("MERITRANK_SERVER_PORT", &mut s.server_port);
  load_var("MERITRANK_NUM_WALKS", &mut s.num_walks);
  load_var(
    "MERITARNK_ZERO_OPINION_NUM_WALKS",
    &mut s.zero_opinion_num_walks,
  );
  load_var("MERITRANK_TOP_NODES_LIMIT", &mut s.top_nodes_limit);
  load_var("MERITRANK_ZERO_OPINION_FACTOR", &mut s.zero_opinion_factor);
  load_var(
    "MERITRANK_SCORE_CLUSTERS_CACHE_SIZE",
    &mut s.score_clusters_cache_size,
  );
  load_var(
    "MERITRANK_SCORE_CLUSTERS_TIMEOUT",
    &mut s.score_clusters_timeout,
  );
  load_var(
    "MERITRANK_OMIT_NEG_EDGES_SCORES",
    &mut s.omit_neg_edges_scores,
  );
  load_var(
    "MERITRANK_FORCE_READ_GRAPH_CONN",
    &mut s.force_read_graph_conn,
  );
  load_var("MERITRANK_NUM_SCORE_QUANTILES", &mut s.num_score_quantiles);
  load_var(
    "MERITRANK_SLEEP_DURATION_AFTER_PUBLISH_MS",
    &mut s.sleep_duration_after_publish_ms,
  );
  load_var(
    "MERITRANK_SUBGRAPH_QUEUE_CAPACITY",
    &mut s.subgraph_queue_capacity,
  );

  s
}
