# Migration Journal: NNG → TCP/bincode

This file records every design decision made during the migration from the legacy
NNG/MessagePack transport to the new TCP/bincode transport. Consult this file before
starting each phase.

---

## Architecture overview

### Before
```
psql-connector  --NNG+msgpack--> legacy_request_handler (port 10234)
                                        |
                                 MultiGraphProcessor
                                        ^
                                 (new TCP server port 8080 also running, unused by connector)
```

### After
```
psql-connector  --TCP+bincode--> request_handler (port 8080)
                                        |
                                 MultiGraphProcessor
```

---

## Decisions

### D1 — Auto-calculate on new nodes

**Context**: `aug_graph.rs::set_edge` (called from `absorb_first(WriteEdge)`) registers
new nodes into the node registry but does NOT run `calculate(ego)` for them. Without
`calculate`, no random walks are initialised, so `read_scores` / `read_node_score` always
return empty for that ego.

The legacy NNG handler kept an `InternalState { node_names: HashSet<String> }` across
all connections and emitted an explicit `WriteCalculate(ego)` the first time each src/dst
node was seen.  The new TCP `request_handler.rs` has no such logic.

**Decision (REVISED after testing)**: Do NOT add WriteCalculate at the state-manager
level. Instead, inside `aug_graph::set_edge`, AFTER calling `set_edge_by_id`, check
`self.mr.get_personal_hits().contains_key(&node_id)`. If the node has never been
calculated (no entry in pos_hits), call `self.calculate(node_name)` immediately.
This ensures:
- New nodes get initial walks on the graph that already has the new edge (meaningful).
- Existing nodes keep their incrementally-updated walks; `meritrank_core::set_edge_`
  handles incremental updates for them.
- The legacy NNG path is unaffected: its explicit WriteCalculate runs first, so
  pos_hits already has an entry and the aug_graph check is a no-op.
- Multiple recalculations per node are prevented (check is idempotent).

The earlier approach (WriteCalculate before WriteEdge in state_manager) was wrong
because it reset walks at intermediate graph states, corrupting scores across edges.

---

### D2 — WriteDeleteNode implementation

**Context**: Both `state_manager.rs` and `legacy_sync_state_manager.rs` ignore
`WriteDeleteNode` with a `log_warning!`.  The connector test `delete_nodes` exists but
its assertions are commented out.

**Decision**: Add `AugGraphOp::DeleteNode(NodeName)` to the `AugGraphOp` enum in
`data.rs`.  In `aug_graph.rs::absorb_first`, handle it by collecting all outgoing edges
from the node and calling `set_edge_by_id(src, dst, 0.0, magnitude)` for each, effectively
zeroing them.  Wire `ReqData::WriteDeleteNode` in both state managers to enqueue this op.

The `index` field in `OpWriteDeleteNode` is passed as `magnitude` for the zero-weight
edge (matching how `WriteDeleteEdge` is implemented in the same state managers).

---

### D3 — Version / LogLevel

**Context**: `mr_service()` calls `CMD_VERSION` over NNG to get the server version string
and parses it as "X.Y.Z".  `mr_log_level()` sets a server-side log level over NNG.
Neither has a `ReqData` variant in the new protocol.

**Decision**:
- `mr_service()` → return the connector's own `CARGO_PKG_VERSION` string directly (no
  network call). The PG test `service()` already parses the result as a semver; the
  connector package has version "0.3.26" which satisfies that format.
- `mr_log_level()` → return `"Ok"` immediately (no-op). Log level is controlled by
  server-side env vars in the new architecture.

---

### D4 — Sync stamp management

**Context**: `ReqData::Sync(stamp: u64)` in the new protocol requires a monotonically
increasing stamp. The server enqueues `AugGraphOp::Stamp(stamp)` to all subgraph queues
and waits until every subgraph's `aug_graph.stamp` reaches the given value.  The legacy
NNG handler managed an `InternalState.stamp` counter.  The connector has no persistent
state between PG function calls.

**Decision**: Add `static SYNC_STAMP: AtomicU64 = AtomicU64::new(0)` in
`psql-connector/src/new_rpc.rs`.  `mr_sync` increments it (fetch_add + 1) and uses the
new value as the stamp.  Since the AtomicU64 lives for the lifetime of the Postgres backend
process, stamps are monotonically increasing within a session.  A backend restart (or
`pg_reload_conf`) resets the counter to 0; this is safe because a reset implies a fresh
`MultiGraphProcessor` state on the server too.

---

### D5 — URL env var

**Context**: The connector reads `MERITRANK_SERVICE_URL` (default
`tcp://127.0.0.1:10234`) and passes it verbatim to `nng::Socket::dial`.  The new TCP
server listens on a separate address (`MERITRANK_SERVER_ADDRESS:MERITRANK_SERVER_PORT`,
default `127.0.0.1:8080`).

**Decision**: Reuse `MERITRANK_SERVICE_URL` in `new_rpc.rs` but change its default to
`tcp://127.0.0.1:8080`.  Strip the `tcp://` prefix before `TcpStream::connect`.  This
keeps deployments that already set `MERITRANK_SERVICE_URL` working with a simple port
change; the `tcp://` scheme prefix is accepted but ignored.

---

### D6 — Blocking flag

**Context**: The legacy `Command` struct has a `blocking: bool` field.  Write ops used
`blocking: false`; reads used `blocking: true`. The new TCP server always responds after
the op is accepted (writes) or computed (reads) — there is no non-blocking mode at the
protocol level.

**Decision**: Drop the `blocking` concept entirely.  For write ops the server returns
`Response::Ok` as soon as the op is enqueued (async, same as `blocking: false`). A
subsequent `mr_sync` call is still required to wait for processing.

---

### D7 — Timeout handling

**Context**: `mr_sync`, `mr_zerorec`, and `mr_recalculate_clustering` accept a
`timeout_msec` parameter; the legacy connector passed it as a recv timeout on the NNG
socket. The new TCP server processes `Sync` server-side (blocking until the stamp is
reached), so the timeout must be enforced client-side.

**Decision**: Set `TcpStream::set_read_timeout(Some(Duration::from_millis(t)))` before
calling `read_response_sync`.  `None` timeout → no read timeout (wait forever).

---

### D8 — FilterOptions bounds mapping

**Context**: The connector's `validate_bounds` helper uses `i32::MAX` / `i32::MIN` as
sentinel values when bounds are unset.  The new `FilterOptions` uses `f64::MAX` /
`f64::MIN` as defaults (and `score_lte: true` / `score_gte: true`).

**Decision**: Map `None` upper bound to `f64::MAX, score_lte: true`; map `None` lower
bound to `f64::MIN, score_gte: true`.  Do NOT use `validate_bounds` from `types.rs` for
new-protocol calls — build `FilterOptions` directly.

---

### D9 — magnitude from index

**Context**: `mr_put_edge` takes `index: i64` (default `-1`).  The new `OpWriteEdge` has
`magnitude: u32`.

**Decision**: `magnitude = if index < 0 { 0 } else { index as u32 }`.  Same rule applies
when building `OpWriteDeleteEdge` and `OpWriteDeleteNode`.

---

### D10 — New edges filter

**Context**: `WriteFetchNewEdges`, `WriteNewEdgesFilter`, `ReadNewEdgesFilter` are stubs
in the new server.

**Decision**: Server returns `Response::NotImplemented` for these three operations (no
silent success or empty payload). The connector maps `NotImplemented` to `Err(...)` so
`mr_fetch_new_edges`, `mr_set_new_edges_filter`, and `mr_get_new_edges_filter` raise a
PostgreSQL ERROR with a clear "not implemented" message instead of silently returning
empty data or Ok.

---

### D11 — Sync TCP wire helpers location

**Decision**: Add `pub mod rpc_sync` to `service/src/lib.rs` and create
`service/src/rpc_sync.rs`.  Export it so the connector (which depends on
`meritrank_service`) can reuse the same encode/decode helpers without duplicating the
framing logic.

---

### D12 — `data` module visibility

**Decision**: Export `pub mod data` from `service/src/lib.rs` so the connector can
reference `Request`, `Response`, `ReqData`, `OpWriteEdge`, `FilterOptions`, etc. by type
without re-declaring them.

---

## Phase log

| Phase | Status | Notes |
|-------|--------|-------|
| 0 — Journal + service lib exports | ✅ complete | |
| 1 — Service semantic fixes | ✅ complete | auto-calc in aug_graph::set_edge; DeleteNode op |
| 2 — New connector RPC module | ✅ complete | new_rpc.rs with tcp_call + unit tests |
| 3 — Migrate connector functions | ✅ complete | all pg_extern functions migrated |
| 4 — Un-comment connector tests | ✅ complete | all 28 tests pass with service running |
| 5 — Remove NNG legacy | ✅ complete | legacy files deleted; nng/rmp-serde removed |
