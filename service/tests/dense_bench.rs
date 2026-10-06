//! Dense-graph benchmark of reverse scores (D14, service/LOAD_TEST_ANALYSIS.md).
//!
//! Three clusters of 1,000 users, each with ~30 reciprocal partners inside its cluster, and one
//! heavy ego linked both ways with 1,000 users across the clusters (≈ 1,000 mutual peers). One
//! cold `ReadMutualScores` per ego, then warm ones. One configuration per run, from the
//! environment (run each in its own process so RSS is its own):
//!
//! | Variable | Default | |
//! |---|---|---|
//! | BENCH_CACHE | 200 | MERITRANK_WALKS_CACHE_SIZE (0 = unbounded) |
//! | BENCH_WALKS | 1000 | NUM_WALKS |
//! | BENCH_ON_DEMAND | = BENCH_WALKS | ON_DEMAND_NUM_WALKS |
//! | BENCH_STALENESS | 0 | SNAPSHOT_STALENESS (c) |
//! | BENCH_SNAPSHOTS_MB | 256 | 0 = snapshots off (frames pinned in portions) |
//! | BENCH_WARM | 5 | warm reads per ego |
//! | BENCH_WRITES | 0 | random in-cluster writes before every warm read |
//! | BENCH_ORDINARY | 2 | ordinary users measured besides the heavy ego |
//! | BENCH_STALL | 0 | 1: also measure write+sync latency while the heavy ego's cold read runs |
//! | BENCH_LABEL | config | label of the JSON line |
//! | BENCH_OUT | — | file the JSON line is appended to |
//!
//! `cargo test --release -p meritrank_service --test dense_bench -- --ignored --nocapture`

use std::io::Write;
use std::time::Instant;

use bincode::{config::standard, encode_to_vec};
use meritrank_service::data::{
  BulkEdge, OpReadMutualScores, OpWriteBulkEdges, OpWriteEdge, ReqData, Request, ResScores,
  Response,
};
use meritrank_service::settings::Settings;
use meritrank_service::state_manager::{over_capacity_reads, MultiGraphProcessor};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const CLUSTERS: usize = 3;
const PER_CLUSTER: usize = 1_000;
const PARTNERS: usize = 15; // chosen per user; reciprocal, so ≈ 30 per user
const HEAVY_LINKS: usize = 1_000;
const ALPHA: f64 = 0.85;
const ZERO_OPINION_FACTOR: f64 = 0.2;

fn env<T: std::str::FromStr>(
  name: &str,
  default: T,
) -> T {
  std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn user(
  c: usize,
  i: usize,
) -> String {
  format!("U{}", c * PER_CLUSTER + i)
}

fn edges() -> Vec<(String, String)> {
  let mut rng = StdRng::seed_from_u64(42);
  let mut e = vec![];
  for c in 0..CLUSTERS {
    for i in 0..PER_CLUSTER {
      for _ in 0..PARTNERS {
        let mut j = rng.random_range(0..PER_CLUSTER);
        while j == i {
          j = rng.random_range(0..PER_CLUSTER);
        }
        e.push((user(c, i), user(c, j)));
        e.push((user(c, j), user(c, i)));
      }
    }
  }
  let mut picked = std::collections::BTreeSet::new();
  while picked.len() < HEAVY_LINKS {
    picked.insert(rng.random_range(0..CLUSTERS * PER_CLUSTER));
  }
  for u in picked {
    e.push(("H".into(), format!("U{u}")));
    e.push((format!("U{u}"), "H".into()));
  }
  e
}

fn status_kb(field: &str) -> u64 {
  std::fs::read_to_string("/proc/self/status")
    .ok()
    .and_then(|s| {
      s.lines()
        .find(|l| l.starts_with(field))
        .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
    })
    .unwrap_or(0)
}

fn fnv(bytes: &[u8]) -> u64 {
  bytes.iter().fold(0xCBF2_9CE4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01B3))
}

async fn request(
  proc: &MultiGraphProcessor,
  data: ReqData,
) -> Response {
  proc.process_request(&Request { subgraph: String::new(), data }).await
}

async fn sync(proc: &MultiGraphProcessor) {
  request(proc, ReqData::Sync(0)).await;
}

#[ignore = "benchmark: run explicitly with --ignored (see the module docs)"]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn dense_reverse_scores_bench() {
  let cache: usize = env("BENCH_CACHE", 200);
  let walks: usize = env("BENCH_WALKS", 1000);
  let on_demand: usize = env("BENCH_ON_DEMAND", walks);
  let staleness: f64 = env("BENCH_STALENESS", 0.0);
  let snapshots_mb: usize = env("BENCH_SNAPSHOTS_MB", 256);
  let warm: usize = env("BENCH_WARM", 5);
  let writes: usize = env("BENCH_WRITES", 0);
  let ordinary: usize = env("BENCH_ORDINARY", 2);
  let label: String = env("BENCH_LABEL", format!("cache{cache}"));

  let settings = Settings {
    num_walks: walks,
    on_demand_num_walks: on_demand,
    walks_cache_size: cache,
    snapshot_staleness: staleness,
    snapshots_mb,
    alpha: ALPHA,
    zero_opinion_factor: ZERO_OPINION_FACTOR,
    seed: 0xD14,
    ..Settings::default()
  };
  let proc = std::sync::Arc::new(MultiGraphProcessor::new(settings.clone()));

  let rss_before = status_kb("VmRSS:");
  let t = Instant::now();
  let r = request(
    &proc,
    ReqData::WriteBulkEdges(OpWriteBulkEdges {
      edges: edges()
        .into_iter()
        .map(|(src, dst)| BulkEdge { src, dst, amount: 1.0, magnitude: 0, context: String::new() })
        .collect(),
    }),
  )
  .await;
  assert!(matches!(r, Response::Ok));
  let load_ms = t.elapsed().as_secs_f64() * 1e3;

  // Write latency (write + sync) alone, then during the heavy ego's cold read.
  let stall = if env::<u8>("BENCH_STALL", 0) == 1 {
    Some(measure_stall(&proc).await)
  } else {
    None
  };

  let mut egos = vec!["H".to_string()];
  for k in 0..ordinary {
    egos.push(user(k % CLUSTERS, 17 + k));
  }

  let mut rng = StdRng::seed_from_u64(7);
  let mut per_ego = vec![];
  for ego in &egos {
    let read = || ReqData::ReadMutualScores(OpReadMutualScores { ego: ego.clone() });
    let before = proc.snapshot_stats("").unwrap();
    let over_before = over_capacity_reads();
    let t = Instant::now();
    let cold = request(&proc, read()).await;
    let cold_ms = t.elapsed().as_secs_f64() * 1e3;
    let rows = match &cold {
      Response::Scores(ResScores { scores }) => scores.len(),
      _ => 0,
    };
    let cold_hash = fnv(&encode_to_vec(&cold, standard()).unwrap());
    let reverse: Vec<(String, f64)> = match &cold {
      Response::Scores(ResScores { scores }) => {
        scores.iter().map(|s| (s.target.clone(), s.reverse_score)).collect()
      },
      _ => vec![],
    };
    let after_cold = proc.snapshot_stats("").unwrap();
    let over_cold = over_capacity_reads() - over_before;
    sync(&proc).await; // admissions published

    let mut warm_ms = vec![];
    let mut warm_hashes = vec![];
    for _ in 0..warm {
      for _ in 0..writes {
        let c = rng.random_range(0..CLUSTERS);
        let (a, b) = (rng.random_range(0..PER_CLUSTER), rng.random_range(0..PER_CLUSTER));
        if a != b {
          request(
            &proc,
            ReqData::WriteEdge(OpWriteEdge {
              src:       user(c, a),
              dst:       user(c, b),
              amount:    rng.random_range(0.5..2.0),
              magnitude: 0,
            }),
          )
          .await;
        }
      }
      if writes > 0 {
        sync(&proc).await;
      }
      let t = Instant::now();
      let w = request(&proc, read()).await;
      warm_ms.push(t.elapsed().as_secs_f64() * 1e3);
      warm_hashes.push(fnv(&encode_to_vec(&w, standard()).unwrap()));
      sync(&proc).await;
    }
    let after = proc.snapshot_stats("").unwrap();
    per_ego.push(serde_json::json!({
      "ego": ego,
      "rows": rows,
      "cold_ms": cold_ms,
      "warm_ms": warm_ms,
      "cold_hash": format!("{cold_hash:016x}"),
      "warm_hashes": warm_hashes.iter().map(|h| format!("{h:016x}")).collect::<Vec<_>>(),
      "cold_sampled": after_cold.reverse_sampled - before.reverse_sampled,
      "cold_frames_calculated": after_cold.graph.frames_calculated - before.graph.frames_calculated,
      "warm_sampled": after.reverse_sampled - after_cold.reverse_sampled,
      "warm_frames_calculated": after.graph.frames_calculated - after_cold.graph.frames_calculated,
      "warm_from_snapshot": after.reverse_from_snapshot - after_cold.reverse_from_snapshot,
      "over_capacity_cold": over_cold,
      "over_capacity_warm": over_capacity_reads() - over_before - over_cold,
      "reverse": if ego == "H" { serde_json::json!(reverse) } else { serde_json::Value::Null },
    }));
  }

  let stats = proc.snapshot_stats("").unwrap();
  let allocated_walks = proc.read_subgraph("", |g| g.mr.allocated_walks()).unwrap();
  let line = serde_json::json!({
    "label": label,
    "cache": cache,
    "walks": walks,
    "on_demand": on_demand,
    "staleness": staleness,
    "snapshots_mb": snapshots_mb,
    "writes_between": writes,
    "zero_opinion_factor": ZERO_OPINION_FACTOR,
    "load_ms": load_ms,
    "rss_kb": status_kb("VmRSS:"),
    "rss_before_kb": rss_before,
    "peak_rss_kb": status_kb("VmHWM:"),
    "snapshot_count": stats.snapshot_count,
    "snapshot_bytes": stats.snapshot_bytes,
    "snapshot_quota": stats.snapshot_quota,
    "allocated_walks": allocated_walks,
    "snapshots_invalidated": stats.graph.snapshots_invalidated,
    "snapshots_admitted": stats.graph.snapshots_admitted,
    "snapshots_rejected": stats.graph.snapshots_rejected,
    "egos": per_ego,
    "stall": stall,
  });
  println!("BENCH {}", line);
  if let Ok(path) = std::env::var("BENCH_OUT") {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    writeln!(f, "{}", line).unwrap();
  }
}

/// Latencies (ms) of `write + sync` in the same subgraph: idle, and while a cold
/// `ReadMutualScores` of the heavy ego (sampling ~all its peers) runs.
async fn measure_stall(proc: &std::sync::Arc<MultiGraphProcessor>) -> serde_json::Value {
  async fn writes(proc: &MultiGraphProcessor, n: usize, stop: Option<&std::sync::atomic::AtomicBool>) -> Vec<f64> {
    let mut v = vec![];
    for i in 0..n {
      if stop.map_or(false, |s| s.load(std::sync::atomic::Ordering::Relaxed)) {
        break;
      }
      let t = Instant::now();
      request(
        proc,
        ReqData::WriteEdge(OpWriteEdge {
          src:       user(0, i % PER_CLUSTER),
          dst:       user(0, (i * 7 + 3) % PER_CLUSTER),
          amount:    1.0 + (i % 3) as f64,
          magnitude: 0,
        }),
      )
      .await;
      sync(proc).await;
      v.push(t.elapsed().as_secs_f64() * 1e3);
    }
    v
  }
  let idle = writes(proc, 50, None).await;
  let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
  let (p, st) = (std::sync::Arc::clone(proc), std::sync::Arc::clone(&stop));
  let writer = tokio::spawn(async move { writes(&p, 100_000, Some(&st)).await });
  let t = Instant::now();
  request(proc, ReqData::ReadMutualScores(OpReadMutualScores { ego: "H".into() })).await;
  let read_ms = t.elapsed().as_secs_f64() * 1e3;
  stop.store(true, std::sync::atomic::Ordering::Relaxed);
  let during = writer.await.unwrap();
  let stats = |v: &[f64]| {
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let q = |p: f64| v.get(((v.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0.0);
    serde_json::json!({ "n": v.len(), "p50": q(0.5), "p95": q(0.95), "max": q(1.0) })
  };
  serde_json::json!({ "idle": stats(&idle), "during_cold_read": stats(&during), "cold_read_ms": read_ms })
}
