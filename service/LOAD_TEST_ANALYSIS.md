# Load Test Results Analysis

## Test design (queue-based)

- **Data**: All edges from the configured CSV are loaded via a single **bulk** command, then sync. **Default**: `psql-connector/testdata/edges.csv` (855 edges, 101 users — larger and more realistic). Override with `MERITRANK_LOAD_TEST_EDGES`.
- **Warmup**: For every **user (U)** node, the test runs **N walks** (WriteCalculate). Default **N=1000** keeps warmup ≤30s for ~100 egos; set `MERITRANK_LOAD_TEST_NUM_WALKS` (e.g. `10000`) for stress. Then a single **sync** (Stamp + `sync_future`) ensures all calculations are applied and visible; a 500 ms delay follows before load phases start. Only users are warmed up; other node types (e.g. B) are not calculated.
- **Write constraints**: Random writes are restricted to:
  - **WriteEdge**: either **U→U** (two distinct users from the warmed-up set) or **U→B** (user → beacon, when beacons exist).
  - **WriteDeleteNode**: deletes a node from the set of **write targets** (users + beacons that exist in the graph).
- **Read mix**: Reads are a 50/50 mix of **ReadScores(ego)** and **ReadMutualScores(ego)** (ego = random user).
- **Op queue**: The client maintains a **shared queue of ops** (mixed reads and writes) with a fixed **read:write ratio** (default 100:1). A **producer** task enqueues ops (with optional pacing); **N worker** tasks concurrently pop from the queue and call the service (`process_request`) for each op. Order of execution does not matter.
- **Phases** (workers + pacing):
  - **low**: 3 workers, 10 ms delay per op (producer and workers).
  - **medium**: 10 workers, 1 ms delay per op.
  - **high**: 30 workers, no delay.
- **Logging**: CMD (read/write ops) logging is **off by default**. Set `MERITRANK_LOG_CMD=1` to enable.
- **Phase duration**: Default 20 s per phase; override with `MERITRANK_LOAD_TEST_PHASE_SECS` (e.g. `5` for quicker runs).

### Eviction mode (walk cache under pressure)

- **Mode**: Set `MERITRANK_LOAD_TEST_MODE=eviction` to run with a **bounded walk cache** so the system is constantly near the eviction limit.
- **Cache size**: In eviction mode, `walks_cache_size` is set from `MERITRANK_LOAD_TEST_EVICTION_CACHE_SIZE` (default **20**). Only that many egos keep walk data; the rest are evicted when new egos are calculated or touched.
- **Behavior**: With more users than cache slots (e.g. 101 users, cache 20), a large fraction of reads will be **cache misses**: the ego is not in the cache, so `ensure_calculated` triggers `WriteCalculate`, and inserting the ego into the tracker may **evict** another ego (which then receives `ClearEgo`). Throughput and latency under this regime reflect the cost of frequent recalculation and eviction.
- **CSV**: Phase names are prefixed with `eviction_` (e.g. `eviction_low`, `eviction_medium`, `eviction_high`) so you can compare with default runs in the same `load_test_stats.csv`.

---

## Run: `psql-connector/testdata/edges.csv` (latest)

- **Edges**: 855  
- **Nodes**: 303 total, **101 users**, **61 beacons**, 162 write targets  
- **Warmup**: 101 users × N walks (default 1000), then sync + 500 ms, then **ResetStats**; Warmup duration is printed; if >30s, a hint suggests lowering `MERITRANK_LOAD_TEST_NUM_WALKS`. no “Node is not calculated” spam.
- **Client queue**: Capped at 10k; oldest ops dropped when over cap.
- **Latest run**: Release build, **10k walks/ego** (`MERITRANK_LOAD_TEST_NUM_WALKS=10000`). Warmup **1.3 s**; 20 s per phase.

### Client throughput (this run)

| Phase   | Reads  | Writes | Ratio (r/w) | Reads/s | Notes                    |
|--------|--------|--------|-------------|---------|---------------------------|
| low    | 1,783  | 26     | 68.6        | ~89     | 3 workers, 10 ms delay    |
| medium | 9,608  | 89     | 108.0       | ~480    | 10 workers, 1 ms delay   |
| high   | 75,337 | 714    | 105.5       | ~3,767  | 30 workers, no delay     |

- **Max read throughput**: **12,048 reads/s** (peak at 6s in high phase; client queue at cap thereafter).
- **Client queue**: 0–1 during low/medium; high phase hits 10k cap (workers stay busy). Throughput limited by service-side processing; read:write ratio near target (100:1).

### Op processing time (from `load_test_stats.csv`)

| Metric | This run (10k walks, release) | Notes |
|--------|--------------------------------|--------|
| median | ~15 ms  | Score/mutual reads on 303-node graph. |
| p95    | ~50 ms  | Heavy reads + occasional write path. |
| p99    | ~92 ms  | Tail of mutual scores; some spikes. |
| max    | —       | From sample. |

**Final stats** (end of run): `median_us=15045`, `p95_us=50045`, `p99_us=91838`, `count=1261`. The CSV may show `pending` underflow in some rows (known double-buffer stats artifact); treat very large `pending` values as invalid.

### Pending queue

- **low / medium**: `pending` in the CSV can show underflow (artifact); use it only as a rough backlog indicator.
- **high**: `pending` is meaningful; client queue can hit 10k cap under sustained load.

---

## When do queues get too long?

- With 30 workers and no pacing, the **client** queue is capped at 10k (oldest ops dropped). The **service** processing queue stays in the hundreds. To stress the service further: more workers, larger graph, or higher write fraction.
- Watch for `pending` approaching `subgraph_queue_capacity` (e.g. 1024) in the CSV.

---

## Clone vs apply-ops

- **Apply-ops (current)**: One logical op applied once per copy; no full graph clone. Median apply time in the tens of µs for the fast path; p95/p99 reflect WriteCalculate and heavy reads (scores, mutual scores).
- **Clone (hypothetical)**: Full graph clone on swap would add large swap latency and likely higher p95/p99 and faster queue growth under the same load.
- **Conclusion**: Op processing cost is dominated by **WriteCalculate** and heavy reads (scores, mutual scores), not the arc-swap/apply machinery.

---

## How to re-run and inspect

```bash
cd /path/to/meritrank-rust
# Default: psql-connector/testdata/edges.csv (855 edges, 101 users, 61 beacons)
cargo run --bin load_test -p meritrank_service

# Optional: enable read/write ops logging
MERITRANK_LOG_CMD=1 cargo run --bin load_test -p meritrank_service

# Inspect server-side stats (appended each run)
cat service/load_test_stats.csv

# Eviction mode: small walk cache, constant eviction pressure (same default edges; phases prefixed eviction_* in CSV)
MERITRANK_LOAD_TEST_MODE=eviction MERITRANK_LOAD_TEST_EVICTION_CACHE_SIZE=20 \
  cargo run --bin load_test -p meritrank_service

# Shorter phases (e.g. 5 s) for quicker comparison runs
MERITRANK_LOAD_TEST_PHASE_SECS=5 cargo run --bin load_test -p meritrank_service
MERITRANK_LOAD_TEST_MODE=eviction MERITRANK_LOAD_TEST_PHASE_SECS=5 \
  MERITRANK_LOAD_TEST_EVICTION_CACHE_SIZE=20 \
  cargo run --bin load_test -p meritrank_service

# Override edges file (e.g. use service testdata for a smaller graph)
MERITRANK_LOAD_TEST_EDGES=service/testdata/edges.csv cargo run --bin load_test -p meritrank_service

# Warmup: default 1000 walks/ego (keeps warmup ≤30s). Stress test with more walks:
MERITRANK_LOAD_TEST_NUM_WALKS=10000 cargo run --release --bin load_test -p meritrank_service
```

---

## Eviction vs default: what to expect

- **Default** (unlimited cache): After warmup, all user egos have walks; reads are served from cache (no recalculation). Latency is dominated by score computation and moka score caches.
- **Eviction** (e.g. cache size 20, 101 users): After warmup, only the last ~20 egos remain in the walk cache; the rest were evicted. During load, most reads hit a **cold** ego, so the service must run `WriteCalculate` (10k walks) and then may evict an existing ego. You should see:
  - **Higher median/p95/p99** (many more full recalculations).
  - **Lower reads/s** for the same worker count (each cold read is expensive).
  - **Similar or higher pending** (queue backs up when recalc is frequent).
- **Comparison**: Run default and eviction with the same `MERITRANK_LOAD_TEST_EDGES` and `MERITRANK_LOAD_TEST_PHASE_SECS`, then compare `load_test_stats.csv` rows for `low`/`medium`/`high` vs `eviction_low`/`eviction_medium`/`eviction_high` (median_us, p95_us, p99_us, sample_count).
