# Load Test Results Analysis

## Test design (queue-based)

- **Data**: All edges from the configured CSV are loaded via a single **bulk** command, then sync. Default: `testdata/edges.csv`; override with `MERITRANK_LOAD_TEST_EDGES` (e.g. `psql-connector/testdata/edges.csv`).
- **Warmup**: For every **user (U)** node, the test runs **10k walks** (WriteCalculate). Then a single **sync** (Stamp + `sync_future`) ensures all calculations are applied and visible; a 500 ms delay follows before load phases start. Only users are warmed up; other node types (e.g. B) are not calculated.
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

---

## Run: `psql-connector/testdata/edges.csv`

- **Edges**: 855  
- **Nodes**: 303 total, **101 users**, **61 beacons**, 162 write targets  
- **Warmup**: 101 users × 10k walks, then sync + 500 ms; no “Node is not calculated” spam (warmup/sync fix applied).

### Client throughput (this run)

| Phase   | Reads  | Writes | Ratio (r/w) | Notes                    |
|--------|--------|--------|-------------|---------------------------|
| low    | 1,787  | 18     | 99.3        | 3 workers, 10 ms delay    |
| medium | 9,388  | 99     | 94.8        | 10 workers, 1 ms delay   |
| high   | 45,378 | 422    | 107.5       | 30 workers, no delay     |

- **High phase**: Client op queue grew very large (e.g. ~13.5M ops) because the producer enqueued with no delay while 30 workers could not keep up with the service. Throughput is limited by service-side processing, not by the client.

### Op processing time (from `load_test_stats.csv`)

| Metric | This run (psql-connector edges) | Notes |
|--------|----------------------------------|--------|
| median | ~148 ms                           | Dominated by score/mutual reads on 303-node graph. |
| p95    | ~361 ms                           | Heavy reads + occasional write path. |
| p99    | ~447 ms                           | Tail of mutual scores / 10k-walk cost. |
| max    | ~510 ms                           | Single heavy request. |

**Final stats** (end of run): `pending=487`, `median_us=148358`, `p95_us=361193`, `p99_us=447190`, `count=133`. The CSV still shows `pending` underflow in some rows (known double-buffer stats artifact); treat very large `pending` values as invalid.

### Pending queue

- **low / medium**: `pending` in the CSV can show underflow (artifact); use it only as a rough backlog indicator.
- **high**: `pending` is meaningful (e.g. 469 at end of phase); queue drains but remains in the hundreds under sustained load.

---

## When do queues get too long?

- With 30 workers and no pacing on this graph, the **client** queue grew to millions of ops (producer much faster than workers). The **service** processing queue stayed in the hundreds. To stress the service further: more workers, larger graph, or higher write fraction.
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
# Default: service/testdata/edges.csv
cargo run --bin load_test -p meritrank_service

# psql-connector edges (855 edges, 101 users, 61 beacons)
MERITRANK_LOAD_TEST_EDGES=psql-connector/testdata/edges.csv cargo run --bin load_test -p meritrank_service

# Optional: enable read/write ops logging
MERITRANK_LOG_CMD=1 cargo run --bin load_test -p meritrank_service

# Inspect server-side stats (appended each run)
cat service/load_test_stats.csv
```
