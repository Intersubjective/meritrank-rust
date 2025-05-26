pub use std::num::NonZeroUsize;

pub const VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
  Some(x) => x,
  None => "dev",
};

pub const NUM_SCORE_QUANTILES: usize = 100;

pub const DEFAULT_NUM_WALKS: usize = 10000;
pub const DEFAULT_ZERO_OPINION_NUM_WALKS: usize = 1000;
pub const DEFAULT_TOP_NODES_LIMIT: usize = 100;
pub const DEFAULT_ZERO_OPINION_FACTOR: f64 = 0.20;
pub const DEFAULT_SCORE_CLUSTERS_TIMEOUT: u64 = 60 * 60 * 6; // 6 hours
pub static DEFAULT_SCORES_CACHE_SIZE: NonZeroUsize =
  unsafe { NonZeroUsize::new_unchecked(1024 * 10) };
pub static DEFAULT_WALKS_CACHE_SIZE: NonZeroUsize =
  unsafe { NonZeroUsize::new_unchecked(1024) };
pub const DEFAULT_FILTER_NUM_HASHES: usize = 10;
pub const DEFAULT_FILTER_MAX_SIZE: usize = 8192;
pub const DEFAULT_FILTER_MIN_SIZE: usize = 32;
pub const DEFAULT_OMIT_NEG_EDGES_SCORES: bool = false;
pub const DEFAULT_FORCE_READ_GRAPH_CONN: bool = false;
