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

#[cfg(test)]
mod tests {
  use super::*;

  /// S9 (SERVICE_CONSISTENCY_PLAN.md): the tracker must evict the least recently used ego, never
  /// the one just touched. moka's default TinyLFU policy may reject a newly inserted rare key and
  /// report it as `RemovalCause::Size`, which the service turns into `ClearEgo` for the ego it is
  /// about to read.
  #[ignore = "S9: fixed in phase 4 (own LRU with pins)"]
  #[test]
  fn evicts_least_recently_used_not_the_new_ego() {
    let tracker = WalkTracker::new(2);
    for _ in 0..20 {
      tracker.touch(1);
      tracker.touch(2);
      tracker.cache.run_pending_tasks();
    }
    tracker.touch(3);
    tracker.cache.run_pending_tasks();
    let evicted = tracker.drain_evicted();
    assert!(
      !evicted.contains(&3),
      "the just-touched ego was evicted: {:?}",
      evicted
    );
    assert_eq!(evicted, vec![1], "expected the LRU ego 1 to be evicted");
  }
}
