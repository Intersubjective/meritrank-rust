# MeritRank service (NNG server)
NNG server for https://github.com/Intersubjective/meritrank-psql-connector with embedded Rust MeritRank engine.

## Env variables
- `MERITRANK_SERVICE_URL` - default `"tcp://127.0.0.1:10234"`
- `MERITRANK_SERVICE_THREADS` - default `1`
- `MERITRANK_NUM_WALK` - default `10000`
- `MERITRANK_ZERO_NODE` - default `U000000000000`
- `MERITRANK_TOP_NODES_LIMIT` - default `100`
- `MERITRANK_FILTER_NUM_HASHES` - default `10`
- `MERITRANK_FILTER_MIN_SIZE` - default `32`
- `MERITRANK_FILTER_MAX_SIZE` - default `8192`
