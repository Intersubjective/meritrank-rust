//! Per-subgraph walk cache eviction tracker. Tracks which egos have calculated walks
//! and evicts least-recently-used ones when capacity is exceeded.

use meritrank_core::NodeId;
use moka::notification::RemovalCause;
use moka::sync::Cache;
use parking_lot::Mutex;
use std::sync::Arc;

/// Tracks which egos have walks in the cache and collects evicted ego IDs when capacity is exceeded.
pub struct WalkTracker {
  cache:   Cache<NodeId, ()>,
  evicted: Arc<Mutex<Vec<NodeId>>>,
}

impl WalkTracker {
  /// Creates a new tracker that allows at most `max_egos` egos. When the cache is full,
  /// inserting a new ego evicts the least-recently-used one; the evicted NodeId is
  /// collected and can be drained via `drain_evicted`.
  pub fn new(max_egos: u64) -> Self {
    let evicted = Arc::new(Mutex::new(Vec::new()));
    let evicted_clone = Arc::clone(&evicted);

    let cache = Cache::builder()
      .max_capacity(max_egos)
      .eviction_listener(move |key: Arc<NodeId>, _value: (), cause: RemovalCause| {
        if matches!(cause, RemovalCause::Size) {
          evicted_clone.lock().push(*key);
        }
      })
      .build();

    WalkTracker { cache, evicted }
  }

  /// Records that the given ego was used (read or calculated). If the cache is at capacity,
  /// this may trigger an eviction; the evicted ego ID will be available from `drain_evicted`.
  pub fn touch(&self, ego_id: NodeId) {
    self.cache.insert(ego_id, ());
  }

  /// Returns and clears the list of ego IDs that were evicted since the last drain.
  /// The caller should send `ClearEgo(id)` for each returned ID so that walk storage is freed.
  pub fn drain_evicted(&self) -> Vec<NodeId> {
    std::mem::take(&mut *self.evicted.lock())
  }
}
