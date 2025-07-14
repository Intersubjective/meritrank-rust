use serde::Deserialize;

fn _num_walks() -> usize {
  10000
}
fn _zero_opinion_num_walks() -> usize {
  1000
}
fn _top_nodes_limit() -> usize {
  100
}
fn _zero_opinion_factor() -> f64 {
  0.20
}
fn _score_clusters_cache_size() -> usize {
  1024 * 10
}
fn _score_clusters_timeout() -> u64 {
  21600
} // 60 * 60 * 6 (6 hours)
fn _scores_cache_size() -> usize {
  1024 * 10
} // 1024 * 10
fn _scores_cache_timeout() -> u64 {
  3600
} // 60 * 60 (1 hour)
fn _walks_cache_size() -> usize {
  1024
}
fn _filter_num_hashes() -> usize {
  10
}
fn _filter_max_size() -> usize {
  8192
}
fn _filter_min_size() -> usize {
  32
}
fn _omit_neg_edges_scores() -> bool {
  false
}
fn _force_read_graph_conn() -> bool {
  false
}
fn _num_score_quantiles() -> usize {
  100
}
fn _cache_capacity() -> u64 {
  1_000_000
}
fn _cache_ttl() -> u64 {
  3600
}

#[derive(Clone, Deserialize)]
pub struct AugGraphSettings {
  #[serde(default = "_num_walks")]
  pub num_walks: usize,

  // #[serde(default = "_zero_opinion_num_walks")]
  // pub zero_opinion_num_walks: usize,

  // #[serde(default = "_top_nodes_limit")]
  // pub top_nodes_limit: usize,

  #[serde(default = "_zero_opinion_factor")]
  pub zero_opinion_factor: f64,

  #[serde(default = "_score_clusters_cache_size")]
  pub score_clusters_cache_size: usize,

  #[serde(default = "_score_clusters_timeout")]
  pub score_clusters_timeout: u64,

  #[serde(default = "_scores_cache_size")]
  pub scores_cache_size: usize,

  #[serde(default = "_scores_cache_timeout")]
  pub scores_cache_timeout: u64,

  // #[serde(default = "_walks_cache_size")]
  // pub walks_cache_size: usize,

  // #[serde(default = "_filter_num_hashes")]
  // pub filter_num_hashes: usize,

  // #[serde(default = "_filter_max_size")]
  // pub filter_max_size: usize,

  // #[serde(default = "_filter_min_size")]
  // pub filter_min_size: usize,

  #[serde(default = "_omit_neg_edges_scores")]
  pub omit_neg_edges_scores: bool,

  // #[serde(default = "_force_read_graph_conn")]
  // pub force_read_graph_conn: bool,

  #[serde(default = "_num_score_quantiles")]
  pub num_score_quantiles: usize,

  // #[serde(default = "_cache_capacity")]
  // pub cache_capacity: u64,

  // #[serde(default = "_cache_ttl")]
  // pub cache_ttl: u64,
}

impl AugGraphSettings {
  pub fn from_env() -> Result<Self, envy::Error> {
    envy::from_env::<AugGraphSettings>()
  }
}

impl Default for AugGraphSettings {
  fn default() -> Self {
    // Use envy to deserialize default values from an empty environment
    envy::from_iter::<_, AugGraphSettings>(
      std::iter::empty::<(String, String)>(),
    )
    .expect("Failed to create default settings")
  }
}
