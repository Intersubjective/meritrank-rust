/// When true, run consistency checks (visits + counters) after calculate and set_edge.
/// Follows Rust convention: on in dev/test (`cargo build`, `cargo test`), off in release.
/// Set `MERITRANK_FORCE_ASSERT=1` to enable in release (e.g. when debugging).
pub const ASSERT: bool =
  cfg!(debug_assertions) || option_env!("MERITRANK_FORCE_ASSERT").is_some();
pub const VERBOSE: bool = true;
pub const OPTIMIZE_INVALIDATION: bool = true;
pub const EPSILON: f64 = 1e-6;
