//! Load test: bulk load edges from CSV, warmup 10k walks per user, then drive mixed
//! read/write load from a shared op queue with configurable worker count and pacing per phase.

use meritrank_service::data::{
  BulkEdge, FilterOptions, NodeKind, OpReadMutualScores, OpReadScores, OpWriteBulkEdges,
  OpWriteCalculate, OpWriteDeleteNode, OpWriteEdge, ReqData, Request, Response, ResNodeList,
};
use meritrank_service::node_registry::node_kind_from_prefix;
use meritrank_service::processor_stats::ProcessorStats;
use meritrank_service::settings::Settings;
use meritrank_service::state_manager::MultiGraphProcessor;
use meritrank_service::utils::log::init_log_cmd_from_env;

use rand::prelude::*;
use std::collections::VecDeque;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEFAULT_EDGES_PATH: &str = "testdata/edges.csv";
const PHASE_DURATION_SECS: u64 = 20;
const STATS_INTERVAL_MS: u64 = 1000;
const CLIENT_STATS_INTERVAL: Duration = Duration::from_secs(1);
const MAX_STATS_SAMPLES: usize = 50_000;
const READ_WRITE_RATIO: (u32, u32) = (100, 1);
/// Max client op queue length; oldest ops are dropped when over this.
const CLIENT_QUEUE_CAP: usize = 10_000;

#[derive(Debug, Clone, serde::Deserialize)]
struct CsvEdge {
  src:    String,
  dst:    String,
  weight: f64,
}

#[derive(Clone)]
enum LoadTestOp {
  ReadScores(String),
  ReadMutualScores(String),
  WriteEdge(String, String),
  WriteDeleteNode(String),
}

fn load_edges_from_csv(path: &Path) -> Result<Vec<BulkEdge>, Box<dyn std::error::Error>> {
  let mut rdr = csv::Reader::from_path(path)?;
  let mut edges = Vec::new();
  for result in rdr.deserialize() {
    let row: CsvEdge = result?;
    edges.push(BulkEdge {
      src:       row.src,
      dst:       row.dst,
      amount:    row.weight,
      magnitude: 0,
      context:   String::new(),
    });
  }
  Ok(edges)
}

fn edges_path() -> std::path::PathBuf {
  env::var("MERITRANK_LOAD_TEST_EDGES")
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|_| {
      Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_EDGES_PATH)
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  init_log_cmd_from_env();
  let path = edges_path();
  if !path.exists() {
    eprintln!(
      "Edges CSV not found at {}. Set MERITRANK_LOAD_TEST_EDGES or add {}",
      path.display(),
      DEFAULT_EDGES_PATH
    );
    std::process::exit(1);
  }

  let edges = load_edges_from_csv(&path)?;
  println!("Loaded {} edges from {}", edges.len(), path.display());

  let settings = Settings {
    num_walks:               10_000,
    zero_opinion_num_walks:  1_000,
    ..Settings::default()
  };

  let stats = Arc::new(ProcessorStats::new(MAX_STATS_SAMPLES));
  let processor = Arc::new(MultiGraphProcessor::new_with_stats(
    settings.clone(),
    Arc::clone(&stats),
  ));

  let req = Request {
    subgraph: String::new(),
    data:     ReqData::WriteBulkEdges(OpWriteBulkEdges {
      edges: edges.clone(),
    }),
  };
  let resp = processor.process_request(&req).await;
  if !matches!(resp, Response::Ok) {
    eprintln!("WriteBulkEdges failed: {:?}", resp);
    std::process::exit(1);
  }
  let stamp = 1u64;
  let _ = processor
    .process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::Stamp(stamp),
    })
    .await;
  processor.sync_future(stamp).await;
  println!("Bulk load and sync done.");

  let node_list = processor
    .process_request(&Request {
      subgraph: String::new(),
      data:     ReqData::ReadNodeList,
    })
    .await;
  let all_nodes: Vec<String> = match node_list {
    Response::NodeList(ResNodeList { nodes }) => nodes.into_iter().map(|(name,)| name).collect(),
    _ => {
      eprintln!("ReadNodeList failed");
      std::process::exit(1);
    },
  };
  let users: Vec<String> = all_nodes
    .iter()
    .filter(|n| node_kind_from_prefix(n) == Some(NodeKind::User))
    .cloned()
    .collect();
  let beacons: Vec<String> = all_nodes
    .iter()
    .filter(|n| node_kind_from_prefix(n) == Some(NodeKind::Beacon))
    .cloned()
    .collect();
  let write_targets: Vec<String> = all_nodes
    .iter()
    .filter(|n| {
      let k = node_kind_from_prefix(n);
      k == Some(NodeKind::User) || k == Some(NodeKind::Beacon)
    })
    .cloned()
    .collect();
  println!(
    "Nodes: {} total, {} users, {} beacons (write targets: {})",
    all_nodes.len(),
    users.len(),
    beacons.len(),
    write_targets.len()
  );
  if users.is_empty() {
    eprintln!("No user nodes; cannot run load test.");
    std::process::exit(1);
  }

  for (i, u) in users.iter().enumerate() {
    let _ = processor
      .process_request(&Request {
        subgraph: String::new(),
        data:     ReqData::WriteCalculate(OpWriteCalculate { ego: u.clone() }),
      })
      .await;
    if (i + 1) % 50 == 0 {
      println!("  Warmup: {}/{}", i + 1, users.len());
    }
  }
  // Single sync point: wait until all user calculations are applied and visible before any reads.
  let warmup_stamp = 2u64;
  processor.sync_future(warmup_stamp).await;
  // Brief delay so the swapped front is fully visible to readers before we start load phases.
  tokio::time::sleep(Duration::from_millis(500)).await;
  println!("Warmup (10k walks per user) done; all user nodes synced.");

  let stats_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("load_test_stats.csv");
  let mut file = OpenOptions::new()
    .create(true)
    .write(true)
    .append(true)
    .open(&stats_path)?;
  writeln!(
    file,
    "phase,elapsed_sec,pending,median_us,p95_us,p99_us,min_us,max_us,sample_count"
  )?;

  #[derive(Clone)]
  struct PhaseConfig {
    name:        &'static str,
    num_workers: usize,
    delay:       Option<Duration>,
  }
  let phases: &[PhaseConfig] = &[
    PhaseConfig {
      name:        "low",
      num_workers: 3,
      delay:       Some(Duration::from_millis(10)),
    },
    PhaseConfig {
      name:        "medium",
      num_workers: 10,
      delay:       Some(Duration::from_millis(1)),
    },
    PhaseConfig {
      name:        "high",
      num_workers: 30,
      delay:       None,
    },
  ];

  for phase_cfg in phases {
    println!("Phase: {}", phase_cfg.name);
    let phase_duration = Duration::from_secs(PHASE_DURATION_SECS);
    let reads = Arc::new(AtomicU64::new(0));
    let writes = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let stats_interval = Duration::from_millis(STATS_INTERVAL_MS);

    let queue: Arc<Mutex<VecDeque<LoadTestOp>>> = Arc::new(Mutex::new(VecDeque::new()));
    let (read_ratio, write_ratio) = READ_WRITE_RATIO;
    let total_ratio = read_ratio + write_ratio;

    let producer = {
      let queue = Arc::clone(&queue);
      let users = users.clone();
      let write_targets = write_targets.clone();
      let beacons = beacons.clone();
      let start = start;
      let phase_duration = phase_duration;
      let delay = phase_cfg.delay;
      tokio::spawn(async move {
        let mut rng = rand::rngs::StdRng::seed_from_u64(
          start.elapsed().as_nanos() as u64,
        );
        while start.elapsed() < phase_duration {
          let op = if rng.random_ratio(read_ratio, total_ratio) {
            if rng.random_bool(0.5) {
              users.choose(&mut rng).map(|u| LoadTestOp::ReadScores(u.clone()))
            } else {
              users.choose(&mut rng).map(|u| LoadTestOp::ReadMutualScores(u.clone()))
            }
          } else {
            if rng.random_bool(0.5) && users.len() >= 2 {
              let a = users.choose(&mut rng).unwrap().clone();
              let b = users.choose(&mut rng).unwrap().clone();
              if a != b {
                Some(LoadTestOp::WriteEdge(a, b))
              } else {
                None
              }
            } else if !beacons.is_empty() && !users.is_empty() {
              let u = users.choose(&mut rng).unwrap().clone();
              let b = beacons.choose(&mut rng).unwrap().clone();
              Some(LoadTestOp::WriteEdge(u, b))
            } else if let Some(node) = write_targets.choose(&mut rng) {
              Some(LoadTestOp::WriteDeleteNode(node.clone()))
            } else {
              None
            }
          };
          if let Some(op) = op {
            if let Ok(mut q) = queue.lock() {
              while q.len() >= CLIENT_QUEUE_CAP {
                q.pop_front();
              }
              q.push_back(op);
            }
          }
          if let Some(d) = delay {
            tokio::time::sleep(d).await;
          }
        }
      })
    };

    let worker = |worker_id: usize| {
      let proc = Arc::clone(&processor);
      let queue = Arc::clone(&queue);
      let reads = Arc::clone(&reads);
      let writes = Arc::clone(&writes);
      let start = start;
      let phase_duration = phase_duration;
      let delay = phase_cfg.delay;
      async move {
        let _ = worker_id;
        while start.elapsed() < phase_duration {
          let op = {
            let mut q = match queue.lock() {
              Ok(g) => g,
              Err(_) => continue,
            };
            q.pop_front()
          };
          let op = match op {
            Some(o) => o,
            None => {
              tokio::time::sleep(Duration::from_millis(1)).await;
              continue;
            },
          };
          let req = match op {
            LoadTestOp::ReadScores(ego) => Request {
              subgraph: String::new(),
              data:     ReqData::ReadScores(OpReadScores {
                ego:           ego,
                score_options: FilterOptions::default(),
              }),
            },
            LoadTestOp::ReadMutualScores(ego) => Request {
              subgraph: String::new(),
              data:     ReqData::ReadMutualScores(OpReadMutualScores { ego }),
            },
            LoadTestOp::WriteEdge(src, dst) => Request {
              subgraph: String::new(),
              data:     ReqData::WriteEdge(OpWriteEdge {
                src,
                dst,
                amount:    1.0,
                magnitude: 0,
              }),
            },
            LoadTestOp::WriteDeleteNode(node) => Request {
              subgraph: String::new(),
              data:     ReqData::WriteDeleteNode(OpWriteDeleteNode { node, index: 0 }),
            },
          };
          let is_write = matches!(
            req.data,
            ReqData::WriteEdge(_) | ReqData::WriteDeleteNode(_)
          );
          let _ = proc.process_request(&req).await;
          if is_write {
            writes.fetch_add(1, Ordering::Relaxed);
          } else {
            reads.fetch_add(1, Ordering::Relaxed);
          }
          if let Some(d) = delay {
            tokio::time::sleep(d).await;
          }
        }
      }
    };

    let stats_handle = {
      let stats = Arc::clone(&stats);
      let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&stats_path)?;
      let phase = phase_cfg.name.to_string();
      let phase_start = start;
      let phase_dur = phase_duration;
      tokio::spawn(async move {
        let mut f = file;
        while phase_start.elapsed() < phase_dur {
          tokio::time::sleep(stats_interval).await;
          if phase_start.elapsed() >= phase_dur {
            break;
          }
          let s = stats.snapshot();
          let elapsed = phase_start.elapsed().as_secs();
          let _ = writeln!(
            f,
            "{},{},{},{},{},{},{},{},{}",
            phase,
            elapsed,
            s.pending,
            s.median_us,
            s.p95_us,
            s.p99_us,
            s.min_us,
            s.max_us,
            s.count
          );
          let _ = f.flush();
        }
      })
    };

    let client_stats_handle = {
      let reads = Arc::clone(&reads);
      let writes = Arc::clone(&writes);
      let queue = Arc::clone(&queue);
      let phase_name = phase_cfg.name;
      let phase_start = start;
      let phase_dur = phase_duration;
      tokio::spawn(async move {
        let mut last_elapsed_secs: u64 = 0;
        let mut last_reads: u64 = 0;
        let mut last_writes: u64 = 0;
        while phase_start.elapsed() < phase_dur {
          tokio::time::sleep(CLIENT_STATS_INTERVAL).await;
          let elapsed = phase_start.elapsed();
          if elapsed >= phase_dur {
            break;
          }
          let elapsed_secs = elapsed.as_secs();
          let r = reads.load(Ordering::Relaxed);
          let w = writes.load(Ordering::Relaxed);
          let queue_len = queue.lock().map(|q| q.len()).unwrap_or(0);
          let interval_secs = elapsed_secs.saturating_sub(last_elapsed_secs);
          let (r_s, w_s) = if interval_secs > 0 {
            (
              (r - last_reads) / interval_secs,
              (w - last_writes) / interval_secs,
            )
          } else {
            (0u64, 0u64)
          };
          last_elapsed_secs = elapsed_secs;
          last_reads = r;
          last_writes = w;
          println!(
            "  [{}] {}s reads={} writes={} queue={} r/s={} w/s={}",
            phase_name, elapsed_secs, r, w, queue_len, r_s, w_s
          );
        }
      })
    };

    let mut handles: Vec<_> = (0..phase_cfg.num_workers)
      .map(|i| tokio::spawn(worker(i)))
      .collect();
    handles.push(producer);

    for h in handles {
      let _ = h.await;
    }
    let _ = stats_handle.await;
    let _ = client_stats_handle.await;

    let total_reads = reads.load(Ordering::Relaxed);
    let total_writes = writes.load(Ordering::Relaxed);
    println!(
      "  {}: reads={} writes={} ratio={:.1}",
      phase_cfg.name,
      total_reads,
      total_writes,
      if total_writes > 0 {
        total_reads as f64 / total_writes as f64
      } else {
        0.0
      }
    );
  }

  let snap = stats.snapshot();
  println!(
    "Final stats: pending={} median_us={} p95_us={} p99_us={} count={}",
    snap.pending, snap.median_us, snap.p95_us, snap.p99_us, snap.count
  );
  println!("Stats written to {}", stats_path.display());

  Ok(())
}
