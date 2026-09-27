# Load Test Results Analysis

## Test design (queue-based)

- **Data**: All edges from the configured CSV are loaded via a single **bulk** command, then sync. **Default**: `psql-connector/testdata/edges.csv` (855 edges, 101 users — larger and more realistic). Override with `MERITRANK_LOAD_TEST_EDGES`.
- **Warmup**: For every **user (U)** node, the test runs **N walks** (WriteCalculate). Default **N=1000** keeps warmup ≤30s for ~100 egos; set `MERITRANK_LOAD_TEST_NUM_WALKS` (e.g. `10000`) for stress. Then a single **sync** (the barrier `sync_future`) ensures all calculations are applied and visible; a 500 ms delay follows before load phases start. Only users are warmed up; other node types (e.g. B) are not calculated.
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
- **Behavior**: With more users than cache slots (e.g. 101 users, cache 20), a large fraction of reads will be **cache misses**: the ego and the peers whose reverse scores the read returns are not resident, so they are pinned and calculated (`EnsureCalculated`), evicting the least recently used unpinned egos (`ClearEgo`). Throughput and latency under this regime reflect the cost of frequent recalculation and eviction.
- **CSV**: Phase names are prefixed with `eviction_` (e.g. `eviction_low`, `eviction_medium`, `eviction_high`) so you can compare with default runs in the same `load_test_stats.csv`.

---

## Run: service consistency track (2026-09-27)

Same data and settings as the previous run: `psql-connector/testdata/edges.csv` (855 edges,
303 nodes, 101 users, 61 beacons), release build, `MERITRANK_LOAD_TEST_NUM_WALKS=10000`, 20 s per
phase. **Baseline** = `main` at `8285bde` (before the track); **new** = `feature/service-consistency`
(dispatcher, single queue with replay, watermark barrier, own LRU walk cache with pins, two-phase
reads, no score cache). Both runs on the same machine, one after another.

### Default mode (unlimited walk cache)

| | Baseline | New |
|---|---|---|
| Warmup, 101 egos × 10k walks | 1.2 s | 2.4 s |
| low: reads / writes | 1,795 / 12 | 1,784 / 23 |
| medium: reads / writes | 9,620 / 99 | 9,610 / 103 |
| **high: reads / writes** | 71,506 / 699 | **160,560 / 1,571 (×2.2)** |
| Final median / p95 / p99 | 15.0 / 40.9 / 110.0 ms | 12.9 / 42.2 / 81.5 ms |

- **Throughput under saturation more than doubles.** Reads no longer wait behind the worker (the
  worker holds a copy's write lock only while applying, never while idle — S5), mutual scores no
  longer calculate every user with one global sync per user, and each ego's calculation is one
  idempotent batch waiting only for its own subgraph's watermark.
- **Warmup is slower** (1.2 → 2.4 s): `WriteCalculate` is urgent, so each one is published (and
  replayed into the other copy) on its own, where the baseline let them accumulate in a batch.
  A first version was 7.6 s: explicit `WriteCalculate` went through the read path, which queued an
  `EnsureCalculated`, waited for it and then recalculated again; fixed (the `WriteCalculate` itself
  now registers the ego as resident and is the calculation).
- low/medium phases are paced by the client and identical.

### Eviction mode (`walks_cache_size=20`, 101 users)

| | Baseline | New |
|---|---|---|
| Warmup | 1.2 s | 3.0 s |
| eviction_low: reads | 562 | 101 |
| eviction_medium: reads | **hangs** (reads stop at 786, r/s = 0 for the rest of the run; process had to be killed) | 179 |
| eviction_high: reads | — | 595 |
| Final median / p95 / p99 | — | 8.2 / 31.7 / 130.0 ms, `pending=0` |

- **Baseline hangs** under eviction pressure: the counters freeze in the medium phase and never
  move again. Consistent with the defects fixed by the track (reader stall behind an idle worker,
  S5; a lagging copy published over a newer one, S14; the TinyLFU tracker evicting the ego being
  read, S9). The new version runs every phase to the end with an empty queue.
- **New throughput in this mode is low (~30 reads/s) by design, not a defect.** Every
  `ReadScores` without a limit and every `ReadMutualScores` needs the frames of 67–100 peers for its
  reverse scores, while the cache holds 20: each such read pins peers in portions of 20 and
  recalculates most of them (≈ 10k walks each). The baseline was fast here only because it served
  reverse scores from a stale cache or as 0 — wrong answers — before it hung.
- The service now logs, sparsely, when one read's working set exceeds the cache:
  `A read needs N peer frames but MERITRANK_WALKS_CACHE_SIZE is C …`.
- **Recommendation:** size `MERITRANK_WALKS_CACHE_SIZE` to cover the users that appear together in
  reads. At 10k walks a resident frame costs about 1 MB (walks + visit index), so the whole user
  base of this dataset is ~100 MB. Evicted frames now free their memory and their slots are
  reused, so the cache size really bounds memory.

## Run: negative edges as walls (2026-09-27)

Same data and settings; `feature/negative-edges` (walks follow positive edges only, absorption on
every entry into a wall, blame accounting). The loader now drops the 121 legacy negative edges to
non-user nodes (old dislikes; walls are User→User only, R2); the 7 negative User→User edges load
as walls. New knobs: `MERITRANK_LOAD_TEST_WALLS` (fraction of User→User writes that set a wall:
20 % hard, the rest soft with d in [0.1, 1)) and `MERITRANK_LOAD_TEST_LAMBDA` (discredit).

| | Consistency track, no walls | Walls feature, no walls | Walls feature, 30 % walls, λ = 0.5 |
|---|---|---|---|
| Warmup | 2.4 s | 2.1 s | 2.0 s |
| high: reads / writes | 160,560 / 1,571 | 179,017 / 1,716 | **186,367 / 1,835** |
| Final median / p95 / p99 | 12.9 / 42.2 / 81.5 ms | 11.6 / 35.5 / 69.6 ms | 9.4 / 30.6 / 53.6 ms |

- **Walls cost nothing measurable.** A wall write touches only its owner's walks (R16), and
  absorbed walks are shorter, so heavy wall traffic is if anything cheaper.
- **Slightly faster without walls too:** walks sample positive edges only, with no second
  (absolute-weight) distribution per node.

Eviction mode (cache 20) with 30 % walls: eviction_low / medium / high reads 84 / 188 / 629,
`pending=0`, p99 129 ms — the same working-set thrash as before (the warning fired 10 times,
sparsely logged), no hang.

### Previous run (for reference, before the track)

| Phase | Reads | Writes | Reads/s |
|--------|--------|--------|---------|
| low    | 1,783  | 26     | ~89     |
| medium | 9,608  | 89     | ~480    |
| high   | 75,337 | 714    | ~3,767  |

Final stats then: `median_us=15045`, `p95_us=50045`, `p99_us=91838`.

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

- **Default** (unlimited cache): After warmup, all user egos have walks; reads are served from resident frames (no recalculation). Latency is dominated by score computation.
- **Eviction** (e.g. cache size 20, 101 users): After warmup, only the last ~20 egos remain in the walk cache; the rest were evicted. During load, most reads hit a **cold** ego, so the service must run `WriteCalculate` (10k walks) and then may evict an existing ego. You should see:
  - **Higher median/p95/p99** (many more full recalculations).
  - **Lower reads/s** for the same worker count (each cold read is expensive).
  - **Similar or higher pending** (queue backs up when recalc is frequent).
- **Comparison**: Run default and eviction with the same `MERITRANK_LOAD_TEST_EDGES` and `MERITRANK_LOAD_TEST_PHASE_SECS`, then compare `load_test_stats.csv` rows for `low`/`medium`/`high` vs `eviction_low`/`eviction_medium`/`eviction_high` (median_us, p95_us, p99_us, sample_count).
