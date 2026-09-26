//! Which egos keep their walks in memory, per subgraph (SERVICE_CONSISTENCY_PLAN.md §2.5).
//!
//! A least-recently-used set with pins. A read pins every frame it needs for its duration; only
//! unpinned egos are evicted. Decisions that calculate or evict are made by the caller while it
//! holds the dispatcher lock and are dispatched as operations in the same order, so an eviction
//! can never overtake a later calculation of the same ego.

use lru::LruCache;
use meritrank_core::NodeId;
use parking_lot::Mutex;
use std::sync::Arc;

/// `ready_seq` of an ego whose calculation has been decided but not dispatched yet.
const PENDING: u64 = u64::MAX;

struct Entry {
  pins:      u32,
  /// Sequence number of the `EnsureCalculated` that made the ego resident.
  ready_seq: u64,
}

pub struct Residency {
  lru:      Mutex<LruCache<NodeId, Entry>>,
  /// Maximum number of resident egos; 0 means unlimited.
  capacity: usize,
}

/// What a caller must dispatch after `plan`: evictions first, then one `EnsureCalculated`.
pub struct Plan {
  pub evict:     Vec<NodeId>,
  pub calculate: Vec<NodeId>,
  /// Highest `ready_seq` among the requested egos that were already resident.
  pub ready_seq: u64,
}

impl Residency {
  pub fn new(capacity: usize) -> Arc<Self> {
    Arc::new(Residency {
      lru: Mutex::new(LruCache::unbounded()),
      capacity,
    })
  }

  pub fn capacity(&self) -> usize {
    self.capacity
  }

  pub fn len(&self) -> usize {
    self.lru.lock().len()
  }

  pub fn is_resident(
    &self,
    ego: NodeId,
  ) -> bool {
    self.lru.lock().contains(&ego)
  }

  /// Fast path, no dispatch needed: if every ego is resident and ready, pins them all and returns
  /// the sequence number to wait for. Otherwise changes nothing.
  pub fn try_pin_resident(
    &self,
    egos: &[NodeId],
  ) -> Option<u64> {
    let mut lru = self.lru.lock();
    if !egos
      .iter()
      .all(|e| lru.peek(e).map_or(false, |x| x.ready_seq != PENDING))
    {
      return None;
    }
    let mut ready = 0;
    for ego in egos {
      let entry = lru.get_mut(ego).expect("checked above");
      entry.pins += 1;
      ready = ready.max(entry.ready_seq);
    }
    Some(ready)
  }

  /// Pins every ego (adding absent ones as pending) and chooses evictions to get back within
  /// capacity. The caller holds the dispatcher lock, dispatches the plan, then calls `set_ready`.
  /// An ego still pending from another request is calculated again by this one: the operation is
  /// idempotent, and waiting for one's own dispatch is enough.
  pub fn plan(
    &self,
    egos: &[NodeId],
    pin: bool,
  ) -> Plan {
    let mut lru = self.lru.lock();
    let mut calculate = vec![];
    let mut ready_seq = 0;
    for &ego in egos {
      match lru.get_mut(&ego) {
        Some(entry) => {
          if pin {
            entry.pins += 1;
          }
          if entry.ready_seq == PENDING {
            calculate.push(ego);
          } else {
            ready_seq = ready_seq.max(entry.ready_seq);
          }
        },
        None => {
          lru.push(
            ego,
            Entry {
              pins:      pin as u32,
              ready_seq: PENDING,
            },
          );
          calculate.push(ego);
        },
      }
    }

    let mut evict = vec![];
    if self.capacity > 0 {
      while lru.len() > self.capacity {
        // Least recently used first; skip pinned egos and the ones requested right now (an
        // unpinned request, e.g. an explicit calculation, must not evict itself).
        let victim = lru
          .iter()
          .rev()
          .find(|(id, e)| e.pins == 0 && !egos.contains(id))
          .map(|(id, _)| *id);
        match victim {
          Some(id) => {
            lru.pop(&id);
            evict.push(id);
          },
          None => break, // all pinned: over capacity until they are released
        }
      }
    }
    Plan {
      evict,
      calculate,
      ready_seq,
    }
  }

  /// Records the sequence number of the dispatched `EnsureCalculated` for egos still pending.
  pub fn set_ready(
    &self,
    egos: &[NodeId],
    seq: u64,
  ) {
    let mut lru = self.lru.lock();
    for ego in egos {
      if let Some(entry) = lru.peek_mut(ego) {
        if entry.ready_seq == PENDING {
          entry.ready_seq = seq;
        }
      }
    }
  }

  pub fn unpin(
    &self,
    egos: &[NodeId],
  ) {
    let mut lru = self.lru.lock();
    for ego in egos {
      if let Some(entry) = lru.peek_mut(ego) {
        entry.pins = entry.pins.saturating_sub(1);
      }
    }
  }
}

/// Pins held by one read; released on drop.
pub struct Lease {
  residency: Arc<Residency>,
  egos:      Vec<NodeId>,
}

impl Lease {
  pub fn new(
    residency: Arc<Residency>,
    egos: Vec<NodeId>,
  ) -> Self {
    Lease { residency, egos }
  }
}

impl Drop for Lease {
  fn drop(&mut self) {
    self.residency.unpin(&self.egos);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn resident(r: &Residency) -> Vec<NodeId> {
    let mut v: Vec<_> = r.lru.lock().iter().map(|(id, _)| *id).collect();
    v.sort();
    v
  }

  /// S9: the least recently used unpinned ego is evicted, never the one just requested
  /// (moka's TinyLFU could reject a new, rare key).
  #[test]
  fn evicts_least_recently_used_not_the_new_ego() {
    let r = Residency::new(2);
    for _ in 0..20 {
      r.plan(&[1], false);
      r.plan(&[2], false);
    }
    r.set_ready(&[1, 2], 1);
    let p = r.plan(&[3], false);
    assert_eq!(p.evict, vec![1]);
    assert_eq!(p.calculate, vec![3]);
    assert_eq!(resident(&r), vec![2, 3]);
  }

  #[test]
  fn pinned_egos_are_not_evicted() {
    let r = Residency::new(2);
    r.plan(&[1, 2], true);
    r.set_ready(&[1, 2], 5);
    let p = r.plan(&[3], false);
    assert!(p.evict.is_empty(), "both residents pinned, and 3 was just requested");
    assert_eq!(p.calculate, vec![3]);
    assert_eq!(r.len(), 3, "over capacity while pinned");
    r.set_ready(&[3], 6);
    r.unpin(&[1, 2]);
    let p = r.plan(&[3], false);
    assert_eq!(p.evict, vec![1]);
    assert!(p.calculate.is_empty());
    assert_eq!(r.len(), 2);
  }

  #[test]
  fn fast_path_only_when_all_ready() {
    let r = Residency::new(0);
    assert_eq!(r.try_pin_resident(&[1]), None);
    let p = r.plan(&[1], true);
    assert_eq!(p.calculate, vec![1]);
    assert_eq!(r.try_pin_resident(&[1]), None, "pending");
    r.set_ready(&[1], 7);
    assert_eq!(r.try_pin_resident(&[1]), Some(7));
  }

  #[test]
  fn pending_ego_is_calculated_by_every_requester() {
    let r = Residency::new(0);
    assert_eq!(r.plan(&[4], true).calculate, vec![4]);
    assert_eq!(r.plan(&[4], true).calculate, vec![4]);
    r.set_ready(&[4], 3);
    let p = r.plan(&[4], true);
    assert!(p.calculate.is_empty());
    assert_eq!(p.ready_seq, 3);
  }
}
