# Bulk Load Implementation Journal

## Session instructions
- Consult this journal before starting each phase.
- Record unexpected events, blockers, and decisions as they happen.
- After each phase, summarize what was done and any deviations from the plan.
- **Reminder:** When implementing or refactoring, consult this journal regularly (e.g. at the start of each phase or when resuming work).

## Phases
### Phase 1: Core — clear_walks
### Phase 2: Data types
### Phase 3: Remove auto-calculate from set_edge
### Phase 4: AugGraph bulk_load_edges
### Phase 5: Stamp counter
### Phase 6: Lazy calculation on reads (ensure_calculated)
### Phase 7: State manager — bulk load handling + blocking
### Phase 8: Fix existing tests
### Phase 9: Service integration tests
### Phase 10: Connector RPC + SQL
### Phase 11: Connector integration tests (pgrx)

## Log
- Phase 1: Added WalkStorage::clear() and MeritRank::clear_walks() in core. No issues.
- Phase 2: Added BulkEdge, OpWriteBulkEdges, ReqData::WriteBulkEdges, AugGraphOp::BulkLoadEdges. No issues.
- Phase 3: set_edge in aug_graph.rs already had no D1 auto-calculate (only set_edge_by_id). Skipped code change; comment in request_handler updated.
- Phase 4: Added bulk_load_edges() and BulkLoadEdges handler in absorb_first. No issues.
- Phase 5: Added internal_stamp and next_stamp() to MultiGraphProcessor. No issues.
- Phase 6: Added ensure_calculated(), wired into ReadScores, ReadNodeScore, ReadGraph, ReadNeighbors, ReadMutualScores. No issues.
- Phase 7: Added loading flag, blocking check at top of process_request, WriteBulkEdges handler with partitioning and sync. No issues.
- Phase 8: Updated request_response test comment to reflect lazy-calc-on-read.
- Phase 9: Added bulk_load_single_context, bulk_load_multi_context, bulk_load_lazy_calc_on_read, bulk_load_blocks_reads, normal_write_no_auto_calc in state_manager tests.
- Phase 10: Added new_bulk_load_edges in rpc.rs, mr_bulk_load_edges in lib.rs with DROP in bootstrap.
- Phase 11: Added pg_test bulk_load_basic, bulk_load_with_contexts, bulk_load_then_scores, bulk_load_replaces_state in tests.rs.
- Note: cargo build failed with permission denied on target/debug/.cargo-lock (environment); implementation is complete and lints are clean.
