//! Ops queue and processing-time stats for load testing and tuning.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// Snapshot of processor stats: pending count and processing-time percentiles.
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
  pub pending:    usize,
  pub median_us:  u64,
  pub p95_us:     u64,
  pub p99_us:     u64,
  pub min_us:     u64,
  pub max_us:     u64,
  pub count:      usize,
}

/// Collects ops pending count and per-op processing time samples for percentile reporting.
pub struct ProcessorStats {
  pub ops_pending: AtomicUsize,
  samples:         Mutex<Vec<Duration>>,
  max_samples:     usize,
}

impl ProcessorStats {
  pub fn new(max_samples: usize) -> Self {
    Self {
      ops_pending: AtomicUsize::new(0),
      samples:     Mutex::new(Vec::with_capacity(max_samples.min(4096))),
      max_samples,
    }
  }

  /// Reset stats: clear samples and pending count (e.g. after warmup, before load phase).
  pub fn reset(&self) {
    self.ops_pending.store(0, Ordering::Relaxed);
    if let Ok(mut g) = self.samples.lock() {
      g.clear();
    }
  }

  /// Call when an op is enqueued (e.g. in send_op).
  pub fn record_enqueue(&self) {
    self.ops_pending.fetch_add(1, Ordering::Relaxed);
  }

  /// Call when an op has been applied (e.g. in processing_loop). Decrements pending and stores duration.
  pub fn record_applied(
    &self,
    elapsed: Duration,
  ) {
    self.ops_pending.fetch_sub(1, Ordering::Relaxed);
    if let Ok(mut g) = self.samples.lock() {
      g.push(elapsed);
      let n = g.len();
      if n > self.max_samples {
        g.drain(0..(n - self.max_samples));
      }
    }
  }

  /// Take a snapshot: current pending and percentiles over the sample buffer.
  pub fn snapshot(&self) -> StatsSnapshot {
    let pending = self.ops_pending.load(Ordering::Relaxed);
    let mut samples: Vec<Duration> = if let Ok(g) = self.samples.lock() {
      g.clone()
    } else {
      vec![]
    };
    let count = samples.len();
    if samples.is_empty() {
      return StatsSnapshot {
        pending,
        median_us: 0,
        p95_us:    0,
        p99_us:    0,
        min_us:    0,
        max_us:    0,
        count:     0,
      };
    }
    samples.sort();
    let to_us = |d: Duration| d.as_micros() as u64;
    let median_us = to_us(samples[(count - 1) / 2]);
    let p95_idx = (count as f64 * 0.95).floor() as usize;
    let p95_us = to_us(samples[p95_idx.min(count - 1)]);
    let p99_idx = (count as f64 * 0.99).floor() as usize;
    let p99_us = to_us(samples[p99_idx.min(count - 1)]);
    let min_us = to_us(samples[0]);
    let max_us = to_us(samples[count - 1]);
    StatsSnapshot {
      pending,
      median_us,
      p95_us,
      p99_us,
      min_us,
      max_us,
      count,
    }
  }
}
