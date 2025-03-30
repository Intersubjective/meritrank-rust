# MeritRank service (NNG server)
NNG server for [PSQL Connector](/psql-connector/README.md) with embedded Rust [MeritRank engine](/core/README.md).

## Env variables
- `MERITRANK_SERVICE_URL` - default `"tcp://127.0.0.1:10234"`
- `MERITRANK_SERVICE_THREADS` - default `1`
- `MERITRANK_NUM_WALKS` - default `10000`
- `MERITRANK_ZERO_OPINION_NUM_WALKS` - default `1000`
- `MERITRANK_TOP_NODES_LIMIT` - default `100`
- `MERITRANK_ZERO_OPINION_FACTOR` - from `0` to `100`, default `20`
- `MERITRANK_SCORE_CLUSTERS_TIMEOUT` - in seconds, default `21600` (6 hours)
- `MERITRANK_SCORES_CACHE_SIZE` - default `10240`
- `MERITRANK_WALKS_CACHE_SIZE` - default `1024`
- `MERITRANK_FILTER_NUM_HASHES` - default `10`
- `MERITRANK_FILTER_MIN_SIZE` - default `32`
- `MERITRANK_FILTER_MAX_SIZE` - default `8192`
