# MeritRank service (NNG server)

NNG server for [PSQL Connector](/psql-connector/README.md) with embedded
Rust [MeritRank engine](/core/README.md).

## Env variables

- `MERITRANK_LEGACY_SERVER_NUM_THREADS` - default `4`
- `MERITRANK_LEGACY_SERVER_PORT` - default `10234`
- `MERITRANK_SERVER_PORT` - default `8080`
- `MERITRANK_SERVER_ADDRESS` - default `127.0.0.1`
- `MERITRANK_NUM_WALKS` - default `10000`
- `MERITRANK_ZERO_OPINION_NUM_WALKS` - default `1000`
- `MERITRANK_TOP_NODES_LIMIT` - default `100`
- `MERITRANK_ZERO_OPINION_FACTOR` - from `0.0` to `1.0`, default `0.2`
- `MERITRANK_SCORE_CLUSTERS_CACHE_SIZE` - default `10240`
- `MERITRANK_SCORE_CLUSTERS_TIMEOUT` - in seconds, default `21600` (6 hours)
- `MERITRANK_SCORES_CACHE_SIZE` - default `10240`
- `MERITRANK_SCORES_CACHE_TIMEOUT` - default `3600`
- `MERITRANK_OMIT_NEG_EDGES_SCORES` - default `false` - forces showing a virtual edge on `read_graph` command if there is no real path from ego to focus.
  Useful for demo purposes.
- `MERITRANK_FORCE_READ_GRAPH_CONN` - default `false`
- `MERITRANK_NUM_SCORE_QUANTILES` - default `100`
- `MERITRANK_SLEEP_DURATION_AFTER_PUBLISH_MS` - default `10`
- `MERITRANK_SUBGRAPH_QUEUE_CAPACITY` - default `1024`

## Batch loading

For cold start or backfill, the service supports **batch loading** of edges in a single request (`WriteBulkEdges`):

- All existing subgraphs are reset; then edges are applied per context (user–user edges go to all contexts, others to their context and the aggregate).
- The service **blocks** other read/write requests until the bulk load completes.
- Walks are **not** computed during the load; they are created **lazily on first read** (scores, graph, neighbors, mutual scores) for each ego. This keeps bulk load fast and spreads computation to query time.
- Use the PSQL function `mr_bulk_load_edges` from the [connector](psql-connector/README.md#batch-loading) to send parallel arrays of (src, dst, weight, context).
