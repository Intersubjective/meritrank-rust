# Plan: Service Consistency Track

**Status: IN PROGRESS** — phases 0–5 done on `feature/service-consistency`. Agreed 2026-09-26. Lands in `main` before the negative-edges
feature (`NEGATIVE_EDGES_FEATURE.md`, dependencies C1–C5; journal `NEGATIVE_EDGES_JOURNAL.md`,
D22 and D26). Every defect listed in §1 exists in `main` today, independently of walls.

---

## 1. Defects in the current service

| # | Defect | Where |
|---|---|---|
| S1 | **Buffer copies are not replicas.** `FanoutSender::send` awaits the two queues separately; concurrent senders interleave (`a1, a2, b2, b1`), so the copies can apply non-commuting writes in different orders and diverge in graph state. Their walks are different Monte Carlo samples, so reads jitter across swaps | `service/src/state_manager.rs:25-45` |
| S2 | **No order across contexts.** A User→User write is fanned out by independent tasks per subgraph; two concurrent writes can reach two contexts in different orders | `state_manager.rs:833-885` |
| S3 | **`mr_sync` is not a barrier.** The connector's stamp counter starts at 0 in every PostgreSQL backend process; the service waits for a published stamp `≥` the requested one; `Stamp` assigns rather than maximises. A new backend's `sync(1)` returns at once | `psql-connector/src/rpc.rs:38`, `state_manager.rs:273-303`, `aug_graph/absorb.rs:85` |
| S4 | **`mr_sync` and the reads are `IMMUTABLE`** in PostgreSQL, which may pre-evaluate or reuse their results | `psql-connector/src/lib.rs:45…286` |
| S5 | **Reader stall.** `process_read` loads the published Arc and then takes its read lock; a swap in between lets the worker take that copy's write lock and hold it while blocking for the next op | `state_manager.rs:240-270`, `:62-127` |
| S6 | **Liveness.** The worker waits only on the back queue: `min_ops_before_swap > 1` stalls a barrier without further traffic; `queue_len = 1, min_ops = 2` deadlocks | `state_manager.rs:90-127` |
| S7 | **Score cache never invalidated, shared by both copies.** `cached_scores` (TTL 1 h) and `cached_score_clusters` (TTL 6 h); moka clones share storage. Reverse scores are read cache-first, so they can be an hour stale even when fresh walks exist | `aug_graph/mod.rs:27-50`, `aug_graph/scores.rs:219-241` |
| S8 | **Reverse scores of absent peers live only in the cache**; `mr_mutual_scores` calculates every user one by one, each followed by a global sync | `state_manager.rs:345-365`, `aug_graph/neighbors.rs:136-178` |
| S9 | **Walk tracker is TinyLFU, not LRU.** When full, moka may reject a newly touched ego; the rejection is reported as `RemovalCause::Size`, so `ClearEgo` is sent for the ego that was just calculated and is being read, and rare users are recalculated on every request | `service/src/walk_tracker.rs`; moka 0.12 `policy.rs:132`, `sync/base_cache.rs:1585-1600` |
| S10 | **Only the request ego is tracked.** Peers calculated for mutual scores and egos calculated by explicit `WriteCalculate` are never tracked and never evicted | `state_manager.rs:370, 768` |
| S11 | **Eviction frees no walk memory.** `clear_ego` keeps the ego's block in `ego_blocks` and `RandomWalk::clear` keeps each `Vec`'s capacity | `core/src/rank.rs:32`, `core/src/walk_storage.rs`, `core/src/random_walk.rs` |
| S12 | **Nondeterminism.** Thread RNG; bulk aggregate edges assembled by iterating a `HashMap`; score ties ordered by a `HashSet` | `core/src/graph.rs`, `state_manager.rs:453`, `core/src/rank.rs:130` |
| S14 | **The published state can move backwards.** `FanoutSender` fills queue A before queue B, so B lags by the operations whose second send has not happened yet. After publishing A, the worker catches B up with only what is already in B's queue and publishes it at once (`drained >= min_ops`): readers see an older state than a moment before, and it stays older until the next write. Found by the phase 0 tests | `state_manager.rs:25-45, 107-126` |
| S13 | **Cached sums cancel.** `pos_sum` is maintained by `+=`/`−=`; after removing a weight 2^53 times larger than another, it is 0 while an edge remains | `core/src/graph.rs:232, 294` |

---

## 2. Target design

### 2.1 Operation sequence and dispatcher (S1, S2, S5, S6, S14)

- **Dispatcher.** `MultiGraphProcessor` owns one async mutex through which every mutating
  operation passes: edge writes, bulk load, reset, context creation, `DeleteNode`, zero opinion,
  `EnsureCalculated`, `ClearEgo`, `Barrier`. It assigns a global sequence number `seq` and enqueues
  the operation into every target subgraph in that order, then releases. Reads never take it.
  Fanned-out User→User writes therefore reach every context in the same order.
- **One queue per subgraph.** The two per-copy channels are replaced by one. The worker applies
  each operation to the back copy and appends it (`Arc<AugGraphOp>`) to a replay log. On
  publication the copies swap and the new back copy replays the log, which is then cleared. The
  copies are replicas by construction, and a copy is published only after it has applied every
  operation of the copy published before it, so publication is monotonic (S14).
- **Publication policy.** Publish when the queue is empty, when `MERITRANK_MIN_OPS_BEFORE_SWAP`
  operations have been applied (reinterpreted as the maximum batch size; the name is kept for
  compatibility), or right after an **urgent** operation (`Barrier`, `EnsureCalculated`). The
  worker always drains its single queue, so no threshold can stall it.
- **Publication watermark.** Each subgraph exposes `published_seq` through `tokio::sync::watch`:
  every operation up to it is visible in the published copy. "Wait for operation n" means
  `published_seq ≥ n`.
- **Safe reader acquisition.** A reader loads the published Arc, takes its read lock, then
  re-checks that the Arc is still the published one; otherwise it releases and retries. The worker
  holds the back copy's write lock only while applying operations, never while waiting for input.

### 2.2 Barrier and connector (S3, S4)

- `mr_sync`: under the dispatcher, enqueue an urgent `Barrier` into every subgraph (the set is
  captured under the same lock, so a context created concurrently is either covered or created
  after), then wait for each subgraph's watermark to pass its barrier's `seq`. Every write
  acknowledged before the call was enqueued before the barrier. Enqueue and application errors
  are returned, not ignored.
- Wire protocol unchanged: `ReqData::Sync(stamp)` keeps its field; the value is ignored.
  `ReqData::Stamp` becomes a no-op kept for compatibility.
- Connector: writes and `mr_sync` become `VOLATILE`, reads `STABLE`. An extension upgrade script
  (`ALTER FUNCTION … VOLATILE/STABLE`) ships with the change. The connector's `SYNC_STAMP` counter
  stays only to fill the ignored field.
- Bulk load and reset go through the dispatcher; bulk validates the whole batch before any
  mutation (also R20 of the negative-edges feature).

### 2.3 Randomness (S12)

- The random generator is a field of `MeritRank` (`reseed(seed)`); inside the core it is handed
  to every random draw: neighbour sampling, continuation, optimized invalidation, the incremental
  push, future absorption trials. The public API takes no generator.
- Before applying an operation the worker reseeds the copy's generator from
  `(seed, subgraph, seq)` (SplitMix64 over the settings seed, an FNV-1a key of the subgraph name
  and the sequence number). Both copies apply the same operation with the same stream, so they
  stay identical without sharing generator state; reruns of the same operation sequence
  reproduce every walk.
- `MERITRANK_SEED` sets the seed; unset, a random seed is drawn once at start-up and shared by all
  subgraphs and both copies.
- Deterministic ordering: bulk edges keep input order and contexts are processed in sorted order;
  `get_all_scores` breaks score ties by node id.

### 2.4 Core changes (S11, S13)

- **Lazy exact distributions.** A node's positive-edge distribution and sum are built together in
  one pass and cached in `OnceLock<Option<PosDistr>>`, `PosDistr { index: WeightedIndex, sum }`.
  An edge change (writer, `&mut`) resets the `OnceLock` in O(1); the first consumer builds it in
  O(degree). The sum is exact by construction; the `pos_sum` field becomes a method. Bulk load
  still builds nothing (edges are inserted one by one; eager building would cost O(Σ degree²)).
- The optimizer computes its probability `p = w / (pos_sum + w)` only when some walk visits the
  source (`visits[src]` non-empty), so bulk load never forces a build.
- **Dirty egos.** `MeritRank` collects the egos whose walks changed during an operation (every
  repaired walk's `first_node()`, over all nested `set_edge` calls, including VSIDS rescales), plus
  `calculate` and `clear_ego`; `take_dirty_egos()` drains the set.
- **Calculated set.** An explicit set of calculated egos replaces "has a `pos_hits` entry".
- **`clear_ego` frees memory.** It returns the ego's block to a free list, removes it from
  `ego_blocks`, replaces each walk's `nodes` with an empty `Vec`, and removes the ego from the
  calculated set and the counters. `ensure_block_for_ego` reuses free blocks first.
- `meritrank_core` 0.11 (API change: `pos_sum`/`neg_sum` fields become methods; `reseed`).

### 2.5 Residency: own LRU with pins (S8, S9, S10)

Per subgraph, on the tokio side under a `parking_lot::Mutex`:

```rust
struct Residency {
  lru:      LinkedHashMap<NodeId, Entry>,  // order = recency
  capacity: usize,                          // MERITRANK_WALKS_CACHE_SIZE; 0 = unlimited
}
struct Entry { pins: u32, ready_seq: u64 }  // seq of the EnsureCalculated that made it resident
```

`acquire(egos) -> Lease`:

1. Under the mutex: move each ego to the MRU end and increment `pins`; an ego not in the LRU is
   added and listed for calculation.
2. While residents exceed `capacity`, remove unpinned egos from the LRU head and list them for
   eviction. If every resident is pinned, capacity is exceeded temporarily and restored on a later
   `acquire`.
3. Outside the mutex, through the dispatcher: `ClearEgo(evicted)`, then urgent
   `EnsureCalculated(to_calculate)`; its `seq` becomes the new entries' `ready_seq`.
4. Wait for `published_seq ≥ max(ready_seq)` over all requested egos, including entries another
   request is still calculating (their `ready_seq` is already recorded; no duplicate calculation).
5. Dropping the `Lease` decrements `pins`.

Properties: an ego is evicted only unpinned, decided under the same mutex as pinning, so no
`ClearEgo` is enqueued for an ego a request holds; an eviction already enqueued removed the ego
from the LRU, so the next `acquire` enqueues a later `EnsureCalculated` and waits for it.
`EnsureCalculated` is idempotent in the worker and is not split, even for many egos. Explicit
`WriteCalculate` goes through `acquire` without pinning. Bulk load and reset clear the residency.

### 2.6 Two-phase reads (S7, S8)

Phase 1 acquires the ego and computes forward scores, filters and pagination; phase 2 acquires
the peers whose reverse scores the response needs and reads forward and reverse scores from one
published copy taken after the wait.

| Read | Phase 1 | Phase 2 peers |
|---|---|---|
| `mr_node_score(ego, target)` | ego | target (if a user) |
| `mr_scores` | ego | users on the returned page, after filters and pagination |
| `mr_graph` | ego | destinations of the returned edges |
| `mr_neighbors` | ego | none |
| `mr_mutual_scores` | ego | users with a positive forward score, processed in portions of at most `capacity`, releasing pins between portions |

The peer set is taken from phase 1's copy; both copies are at least as new as any preceding
`mr_sync`, so R21 holds.

Implementation: the first pass records, in a thread-local, every ego whose frame the read touched
(`record_frames` around `get_node_score`/`get_all_scores`); those are the peers. When they exceed
the capacity, every read (not only mutual scores) pins them in portions and takes each row (one
per peer: a score's target, a graph edge's destination) from its portion's read, keeping the first
pass's order. An unpinned request (explicit `WriteCalculate`) never evicts the ego it requests.

### 2.7 Caches (S7)

> **Superseded in part by D14 (JOURNAL.md).** Reverse scores no longer need resident peer frames:
> they come from snapshots or read-local samples, so §2.6's portions are used only with snapshots
> off. `gen[ego]` became `revisions[ego]`, which does **not** change on `ClearEgo` (the evicted
> frame is kept as a snapshot); the cluster key is `(ego, revision, zero_rev)` (no kind).

- `cached_scores` is removed: a score is two counter lookups, and every frame a read needs is
  resident. `MERITRANK_SCORES_CACHE_SIZE` and `MERITRANK_SCORES_CACHE_TIMEOUT` become ignored
  (warning at start-up).
- `cached_score_clusters`: one instance per copy (`AugGraph` implements `Clone` by hand and builds
  fresh caches), key `(ego, kind, gen[ego], zero_rev)`.
- `gen[ego]` lives in `AugGraph` and is bumped for every dirty ego after each operation, on
  `calculate` and on `ClearEgo`. `zero_rev` is bumped by `WriteZeroOpinion`. Bulk load and reset
  start a new epoch. Copies are replicas, so generations agree.

---

## 3. Phases

Each phase lands in `main` separately with its tests.

| # | Phase | Contents | Tests |
|---|---|---|---|
| 0 | **Failing tests first** | Tests that reproduce S1–S3, S5–S7, S9, S14 against current `main`, marked `#[ignore = "S#: fixed in phase N"]` so `main` stays green; each phase removes its markers (`service/tests/consistency.rs`, `service/src/walk_tracker.rs`) | concurrent writes to one edge → copies differ; `sync(1)` after published stamp 100; reader stall; `queue_len=1, min_ops=2` deadlock; lone barrier below the batch threshold; stale reverse score after a change; newly touched ego evicted by the tracker; order across contexts; published state moving backwards |
| 1 | **Core** | RNG as a reseedable `MeritRank` field; lazy exact distributions; `p` only when visited; dirty egos; calculated set; `clear_ego` frees memory; stable tie order. core 0.11 | existing core tests; `test_incremental_bias.rs`; seeded determinism; weight-ratio > 2^53; memory reuse after eviction |
| 2 | **Sequencing** | Dispatcher, single queue, replay log, publication policy, watermark, safe reader acquisition, per-operation RNG | copies bit-identical after concurrent writes; no stall; no deadlock at any threshold; order across contexts; publication monotonic |
| 3 | **Barrier** | `Barrier` op, `mr_sync`, idempotent `EnsureCalculated`, bulk/reset/context creation through the dispatcher, connector volatility and upgrade script | two connector processes with overlapping stamps; sync covers every context incl. one created concurrently; prepared statements see fresh results |
| 4 | **Residency and reads** | Own LRU with pins, `Lease`, two-phase reads, mutual in portions | no eviction of a pinned ego; concurrent acquires of one ego calculate once; mutual with more candidates than capacity; memory bounded by capacity |
| 5 | **Caches** | Remove `cached_scores`; per-copy cluster cache keyed by generation and zero revision | reverse score fresh after a change + `mr_sync`; cluster bounds recomputed after a change; no cross-copy contamination |

---

## 4. Compatibility

- Wire protocol and SQL signatures unchanged; only volatility changes (upgrade script).
- Settings: `MERITRANK_MIN_OPS_BEFORE_SWAP` reinterpreted as the maximum batch size;
  `MERITRANK_WALKS_CACHE_SIZE` keeps its meaning (now a real bound on memory and repair work);
  `MERITRANK_SCORES_CACHE_*` ignored; `MERITRANK_SEED` added.
- `service/src/legacy/` is untouched. Stale "duplicated logic with `*_blocking`" comments in
  `state_manager.rs` are removed.

## 5. Risks and open points

- **Dispatcher throughput.** All writes are serialized through one mutex; enqueueing is cheap, but
  one full subgraph queue backpressures every write. Measure with `service` load tests.
- **Memory.** At W = 10 000 a resident frame is about 1 MB (walks plus visit index); capacity must
  cover the working set, and a large `mr_mutual_scores` holds up to `capacity` pinned frames.
- **Latency of `EnsureCalculated`.** A batch of new peers blocks the subgraph's writes while it
  runs (≈ 60k steps per peer at W = 10 000); accepted, not split.
- **Executor.** Not decided yet.
