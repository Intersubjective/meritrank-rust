# What is pgmer2?

**pgmer2** is a PostgreSQL extension that connects to the [MeritRank service](https://github.com/Intersubjective/meritrank-rust/tree/main/service), exposing SQL functions to manage the graph (edges, contexts), run ranking, and read scores.

## Batch loading

For cold start or backfilling after a restart, use **`mr_bulk_load_edges`** instead of many `mr_put_edge` calls. It sends all edges in one request and is much faster.

**Signature:**

```sql
mr_bulk_load_edges(
  src_arr text[],
  dst_arr text[],
  weight_arr float8[],
  context_arr text[],
  timeout_msec bigint DEFAULT 120000
) RETURNS text
```

- All four arrays must have the same length; each index is one edge: `(src_arr[i], dst_arr[i], weight_arr[i], context_arr[i])`.
- The service clears existing state, loads these edges, and blocks other requests until the load finishes. Walks are created lazily on first score/graph/neighbor reads.
- `timeout_msec` (default 120000) is the RPC timeout in milliseconds.

**Example:**

```sql
SELECT mr_bulk_load_edges(
  ARRAY['U1', 'U1', 'U2']::text[],
  ARRAY['U2', 'U3', 'U3']::text[],
  ARRAY[1.0, 2.0, 1.5]::float8[],
  ARRAY['', '', 'my_ctx']::text[]
);
```

For incremental updates after the graph is loaded, use `mr_put_edge` as usual.
