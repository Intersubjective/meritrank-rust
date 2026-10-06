use crate::utils::log::*;
pub use meritrank_core::BlameRadius;

use std::env::*;
use std::fmt::*;
use std::str::FromStr;

#[derive(Clone)]
pub struct Settings {
  pub legacy_server_port: u16,
  pub legacy_server_num_threads: usize,
  pub server_address: String,
  pub server_port: u16,
  pub num_walks: usize,
  /// Random-walk continuation probability (PageRank-style damping).
  pub alpha: f64,
  pub zero_opinion_factor: f64,
  pub score_clusters_cache_size: usize,
  pub score_clusters_timeout: u64,
  pub scores_cache_size: usize,
  pub scores_cache_timeout: u64,
  /// Max number of egos to keep walk data for per subgraph (0 = unlimited).
  pub walks_cache_size: usize,
  // pub filter_num_hashes: usize,
  // pub filter_max_size: usize,
  // pub filter_min_size: usize,
  pub omit_neg_edges_scores: bool,
  pub force_read_graph_conn: bool,
  pub num_score_quantiles: usize,
  // pub cache_capacity: u64,
  // pub cache_ttl: u64,
  pub min_ops_before_swap: usize,
  pub subgraph_queue_capacity: usize,
  /// When true, collect ops queue and processing-time stats (for GetStats / ResetStats). Off by default.
  pub collect_stats: bool,
  /// Seed of every random draw (MERITRANK_SEED). Unset: drawn once when the settings are created,
  /// so all subgraphs and both buffer copies share it.
  pub seed: u64,
  /// λ, weight of blame for walks absorbed at walls (MERITRANK_DISCREDIT_LAMBDA, ≥ 0). 0 = walls
  /// only withhold credit.
  pub discredit_lambda: f64,
  /// γ, blame decay along an absorbed walk's prefix (MERITRANK_BLAME_DECAY, in [0, 1]).
  pub blame_decay: f64,
  /// Who takes blame (MERITRANK_BLAME_RADIUS: `prefix` or `voucher`).
  pub blame_radius: BlameRadius,
  /// Maximum number of named contexts (MERITRANK_MAX_CONTEXTS): each one is a worker thread and
  /// two graph copies, and any write can name a new one.
  pub max_contexts: usize,
  /// Retained reverse-score snapshots, MiB, for the whole process (both buffer copies of every
  /// subgraph; MERITRANK_SNAPSHOTS_MB). 0 disables snapshots (D14).
  pub snapshots_mb: usize,
  /// Walks of an on-demand frame sampled for a reverse score (MERITRANK_ON_DEMAND_NUM_WALKS);
  /// capped at `num_walks`.
  pub on_demand_num_walks: usize,
  /// c of the staleness heuristic (MERITRANK_SNAPSHOT_STALENESS): a snapshot serves while its
  /// drift ≤ c·(1+λ)·sqrt(ln(2/δ)/(2n)). 0 = strict: any change in its footprint invalidates it.
  pub snapshot_staleness: f64,
  /// Reads that may sample frames at once (MERITRANK_SAMPLING_CONCURRENCY).
  pub sampling_concurrency: usize,
  /// Bytes of sampled frames queued for admission, MiB (MERITRANK_ADMIT_QUEUE_MB); beyond it the
  /// samples are not kept.
  pub admit_queue_mb: usize,
  /// Test support: keep every operation each subgraph applies (`recorded_ops`).
  pub record_ops: bool,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      legacy_server_port: 10234,
      legacy_server_num_threads: 4,
      server_address: "127.0.0.1".into(),
      server_port: 8080,
      num_walks: 10000,
      alpha: 0.85,
      zero_opinion_factor: 0.2,
      score_clusters_cache_size: 1024 * 10,
      score_clusters_timeout: 60 * 60 * 6,
      scores_cache_size: 1024 * 10,
      scores_cache_timeout: 60 * 60,
      walks_cache_size: 0,
      omit_neg_edges_scores: false,
      force_read_graph_conn: false,
      num_score_quantiles: 100,
      min_ops_before_swap: 1,
      subgraph_queue_capacity: 1024,
      collect_stats: false,
      seed: rand::random(),
      discredit_lambda: 0.0,
      blame_decay: 0.8,
      blame_radius: BlameRadius::Prefix,
      max_contexts: 256,
      snapshots_mb: 256,
      on_demand_num_walks: 1000,
      snapshot_staleness: 1.0,
      sampling_concurrency: std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(1),
      admit_queue_mb: 64,
      record_ops: false,
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

/// Load alpha. Range checking is `Settings::validate`'s (alpha must be in (0, 1)).
fn load_alpha(val: &mut f64) {
  const NAME: &str = "MERITRANK_ALPHA";
  if let Ok(s) = var(NAME) {
    match s.parse::<f64>() {
      Ok(x) => *val = x,
      Err(_) => {
        log_error!("Failed to parse {} as float: {:?}", NAME, s);
      },
    }
  }
}

/// Loads a setting that must parse; a malformed value is an error, not a silent default.
fn load_strict<T: FromStr>(
  name: &str,
  val: &mut T,
  errors: &mut Vec<String>,
) {
  if let Ok(raw) = var(name) {
    match raw.parse() {
      Ok(x) => *val = x,
      Err(_) => errors.push(format!("{} has an invalid value {:?}", name, raw)),
    }
  }
}

impl Settings {
  /// Walks of an on-demand frame: `on_demand_num_walks`, capped at `num_walks`.
  pub fn on_demand_walks(&self) -> usize {
    let _ = self;
    todo!("D14: on_demand_walks")
  }

  /// Whether reverse scores use snapshots and on-demand samples (D14): snapshots are on and the
  /// walk cache is bounded (with an unbounded cache every frame stays resident).
  pub fn snapshots_enabled(&self) -> bool {
    let _ = self;
    todo!("D14: snapshots_enabled")
  }

  /// Checks the settings the walk semantics depend on (R23).
  pub fn validate(&self) -> std::result::Result<(), String> {
    if !(self.alpha > 0.0 && self.alpha < 1.0) {
      return Err(format!(
        "MERITRANK_ALPHA must be in (0, 1), got {}: at 1 a walk on a cyclic graph never ends",
        self.alpha
      ));
    }
    if !(self.discredit_lambda.is_finite() && self.discredit_lambda >= 0.0) {
      return Err(format!(
        "MERITRANK_DISCREDIT_LAMBDA must be finite and >= 0, got {}",
        self.discredit_lambda
      ));
    }
    if !(0.0..=1.0).contains(&self.blame_decay) {
      return Err(format!(
        "MERITRANK_BLAME_DECAY must be in [0, 1], got {}",
        self.blame_decay
      ));
    }
    if self.num_walks == 0 {
      return Err("MERITRANK_NUM_WALKS must be positive".into());
    }
    Ok(())
  }
}

/// Settings from the environment, validated: the service refuses to start on invalid values of
/// the settings the walk semantics depend on.
pub fn load_from_env_checked() -> std::result::Result<Settings, String> {
  let mut errors = vec![];
  let mut s = load_from_env();
  load_strict("MERITRANK_DISCREDIT_LAMBDA", &mut s.discredit_lambda, &mut errors);
  load_strict("MERITRANK_BLAME_DECAY", &mut s.blame_decay, &mut errors);
  if let Ok(raw) = var("MERITRANK_BLAME_RADIUS") {
    match raw.as_str() {
      "prefix" => s.blame_radius = BlameRadius::Prefix,
      "voucher" => s.blame_radius = BlameRadius::Voucher,
      _ => errors.push(format!(
        "MERITRANK_BLAME_RADIUS must be `prefix` or `voucher`, got {:?}",
        raw
      )),
    }
  }
  if let Ok(raw) = var("MERITRANK_ALPHA") {
    if raw.parse::<f64>().is_err() {
      errors.push(format!("MERITRANK_ALPHA has an invalid value {:?}", raw));
    }
  }
  if !errors.is_empty() {
    return Err(errors.join("; "));
  }
  s.validate()?;
  Ok(s)
}

/// Load zero opinion factor; must be in [0.0, 1.0]. Invalid values are rejected and default is kept.
fn load_zero_opinion_factor(val: &mut f64) {
  const NAME: &str = "MERITRANK_ZERO_OPINION_FACTOR";
  const MIN: f64 = 0.0;
  const MAX: f64 = 1.0;
  if let Ok(s) = var(NAME) {
    match s.parse::<f64>() {
      Ok(x) if x >= MIN && x <= MAX => *val = x,
      Ok(x) => {
        log_error!(
          "{} must be in [{}, {}], got {}; using default {}",
          NAME,
          MIN,
          MAX,
          x,
          *val
        );
      },
      Err(_) => {
        log_error!("Failed to parse {} as float: {:?}", NAME, s);
      },
    }
  }
}

pub fn load_from_env() -> Settings {
  let mut s = Settings::default();

  load_var("MERITRANK_LEGACY_SERVER_PORT", &mut s.legacy_server_port);
  load_var(
    "MERITRANK_LEGACY_SERVER_NUM_THREADS",
    &mut s.legacy_server_num_threads,
  );
  load_var("MERITRANK_SERVER_ADDRESS", &mut s.server_address);
  load_var("MERITRANK_SERVER_PORT", &mut s.server_port);
  load_var("MERITRANK_NUM_WALKS", &mut s.num_walks);
  load_alpha(&mut s.alpha);
  load_zero_opinion_factor(&mut s.zero_opinion_factor);
  load_var(
    "MERITRANK_SCORE_CLUSTERS_CACHE_SIZE",
    &mut s.score_clusters_cache_size,
  );
  load_var(
    "MERITRANK_SCORE_CLUSTERS_TIMEOUT",
    &mut s.score_clusters_timeout,
  );
  for name in ["MERITRANK_SCORES_CACHE_SIZE", "MERITRANK_SCORES_CACHE_TIMEOUT"] {
    if var(name).is_ok() {
      log_warning!("{} is ignored: scores are no longer cached", name);
    }
  }
  load_var("MERITRANK_WALKS_CACHE_SIZE", &mut s.walks_cache_size);
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
    "MERITRANK_MIN_OPS_BEFORE_SWAP",
    &mut s.min_ops_before_swap,
  );
  load_var(
    "MERITRANK_SUBGRAPH_QUEUE_CAPACITY",
    &mut s.subgraph_queue_capacity,
  );
  load_var("MERITRANK_COLLECT_STATS", &mut s.collect_stats);
  load_var("MERITRANK_SEED", &mut s.seed);
  load_var("MERITRANK_MAX_CONTEXTS", &mut s.max_contexts);

  s
}
