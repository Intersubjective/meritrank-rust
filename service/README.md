# MeritRank service (NNG server)

NNG server for [PSQL Connector](/psql-connector/README.md) with embedded
Rust [MeritRank engine](/core/README.md).

## Env variables

- `MERITRANK_LEGACY_SERVER_NUM_THREADS` - default `4`
- `MERITRANK_LEGACY_SERVER_PORT` - default `10234`
- `MERITRANK_SERVER_PORT` - default `8080`
- `MERITRANK_SERVER_ADDRESS` - default `127.0.0.1`
- `MERITRANK_NUM_WALKS` - default `10000`
- `MERITRANK_ALPHA` - walk continuation probability, from `0.0` (exclusive) to `1.0`, default `0.85`
- `MERITRANK_ZERO_OPINION_NUM_WALKS` - default `1000`
- `MERITRANK_TOP_NODES_LIMIT` - default `100`
- `MERITRANK_ZERO_OPINION_FACTOR` - from `0.0` to `1.0`, default `0.2`
- `MERITRANK_SCORE_CLUSTERS_CACHE_SIZE` - default `10240`
- `MERITRANK_SCORE_CLUSTERS_TIMEOUT` - in seconds, default `21600` (6 hours)
- `MERITRANK_WALKS_CACHE_SIZE` - resident frames per subgraph, default `0` (unbounded). A resident
  frame costs about 1 MB at 10,000 walks (walks + visit index).
- `MERITRANK_SNAPSHOTS_MB` - default `256`; `0` disables snapshots. Reverse-score snapshots (see
  below), retained for the whole process: the budget is split evenly over every subgraph and its
  two buffer copies. Ignored (off) with an unbounded walk cache.
- `MERITRANK_ON_DEMAND_NUM_WALKS` - default `1000`, capped at `MERITRANK_NUM_WALKS`: walks of a frame
  sampled for a reverse score.
- `MERITRANK_SNAPSHOT_STALENESS` - `c`, default `1.0`, finite and `>= 0`; `0` is strict mode (see
  below).
- `MERITRANK_SAMPLING_CONCURRENCY` - reads that may sample frames at once, default half the CPUs.
- `MERITRANK_ADMIT_QUEUE_MB` - default `256`: samples of a read waiting to be stored as snapshots;
  beyond it the rest are sampled again by a later read.
- `MERITRANK_OMIT_NEG_EDGES_SCORES` - default `false` - forces showing a virtual edge on `read_graph` command if there is no real path from ego to focus.
  Useful for demo purposes.
- `MERITRANK_FORCE_READ_GRAPH_CONN` - default `false`
- `MERITRANK_NUM_SCORE_QUANTILES` - default `100`
- `MERITRANK_MIN_OPS_BEFORE_SWAP` - default `1`
- `MERITRANK_SUBGRAPH_QUEUE_CAPACITY` - default `1024`
- `MERITRANK_COLLECT_STATS` - default `false`. When set to `true`, the service collects ops queue length and per-op processing time (for load testing and tuning). When enabled, use the protocol commands **ResetStats** (e.g. after warmup) and **GetStats** (to read pending count, median/p95/p99/min/max/count in µs). Stats are off by default in production.

## One node class, isolated contexts

Every node is a plain node: any non-empty name, any node can be an ego, there are no owners
(`kind` and `hide_personal` of the protocol and the SQL functions are accepted and ignored).
Contexts are isolated: a write (edge, wall, deletion, zero opinion, calculation) reaches only the
context it names; nothing is aggregated into the null context and a new context starts empty.
Walls are valid between any two nodes, in any context. (JOURNAL.md D14.)

## Reverse scores and snapshots

A reverse score (the ego's score in a peer's frame: `ReadMutualScores`, `ReadScores`,
`ReadNodeScore`, `ReadNeighbors`, `ReadGraph`) comes from the peer's resident frame, else from a
snapshot of it, else from a frame of `MERITRANK_ON_DEMAND_NUM_WALKS` walks sampled by the read
itself (outside the walk storage) and then kept as a snapshot. A frame evicted from the walk
cache is kept as a snapshot too. So a read never pins peers' frames, and once warm it computes
nothing. Fresh frames are seeded per ego: a sample equals what a calculation would give.

- Strict mode (`MERITRANK_SNAPSHOT_STALENESS=0`): a snapshot is dropped by any change of a
  positive out-edge of a node its walks visited, and by a wall change of its owner — an admitted
  sample then always equals a fresh calculation, and a kept evicted frame equals that frame.
- Heuristic (`c > 0`): each such change adds `(1+λ)·α·tv·visits(S)/n` to the snapshot's drift
  (`tv` = total variation of S's next-step distribution); it is dropped once the drift exceeds
  `c·(1+λ)·sqrt(ln(2/δ)/(2n))`, δ = 0.05 — the Monte Carlo noise of the snapshot. Changes where
  its walks never went are not seen (no guarantee). Wall changes of its owner, node deletion,
  reset and bulk load always drop it.

Size: about **24 bytes per node a frame's walks visited** (+ 128 bytes, + 800 bytes of cluster
bounds): 10–60 KB at 1,000 walks (dense graph: ~28 KB), versus ~1 MB for a resident frame at
10,000 walks. Both buffer copies hold the snapshots; `MERITRANK_SNAPSHOTS_MB` counts both.

## Batch loading

For cold start or backfill, the service supports **batch loading** of edges in a single request (`WriteBulkEdges`):

- All existing subgraphs are reset; then every edge is applied to its own context only (isolated contexts).
- The service **blocks** other read/write requests until the bulk load completes.
- Walks are **not** computed during the load; they are created **lazily on first read** (scores, graph, neighbors, mutual scores) for each ego. This keeps bulk load fast and spreads computation to query time.
- Use the PSQL function `mr_bulk_load_edges` from the [connector](psql-connector/README.md#batch-loading) to send parallel arrays of (src, dst, weight, context).
