# Feature: Negative Edges as Absorbing Walls

**Status: SPECIFIED, NOT IMPLEMENTED.** Phase 1 below is the agreed scope, including the
implementation design review of 2026-09-26 (journal D16–D25). The code in `core/` still implements
the forward-penalty semantics. The feature depends on the service-consistency track (§6,
"Dependencies"), which lands in `main` first.

This document states *what* MeritRank must do: requirements, API contract and acceptance tests.
The reasoning behind each decision lives in `NEGATIVE_EDGES_JOURNAL.md`, cited here as
**J-P1, J-P2** (problems), **J-D1…J-D25** (decisions) and **A1…A9** (axioms).
Requirement IDs are stable: a new requirement takes the next free number, and a withdrawn one keeps
its number, so numbers within a section need not be consecutive. The interactive model is
`scripts/negative_edges_demo.ipynb`. The application-side design (Tentura) is
`tentura/docs/plans/post-request-closure-social-design.md`, §17.4. Where this document and the
journal differ, this document is current.

---

## 1. Summary

Today a negative edge is a *traversable* edge: a walk that takes it continues along the
distrusted node's own positive edges and penalises everything it meets from there on. Bob
therefore chooses whom his distrusters punish (the curse attack), and the more people distrust
him, the more he can aim (J-P1, J-P2).

The feature replaces this with an **absorbing wall**. A negative User→User edge `A→B` makes B a
wall in A's frame: A's walks travel only along positive edges, and every time a walk steps into B
it is absorbed with probability `d`. An absorbed walk credits nobody (withholding). Optionally
(`λ > 0`), it also subtracts blame from the nodes that routed it to B (backward discredit). Bob's
own edges matter in A's frame only through the `(1 − d)` share that passes through him.

No new API function is needed: a wall is encoded as a negative edge weight (J-D8).

---

## 2. Glossary

This glossary is binding for this document, the journal, the notebook and the Tentura design
note. The Russian column gives the term used in the notebook and in Tentura's documents.

| English | Russian | Symbol | Meaning |
|---|---|---|---|
| ego | эго | A | The node whose frame is computed |
| frame (ego frame) | рамка (рамка эго, рамка A) | `score_A(·)` | All scores computed from A's walks. Unrelated to "рамка" in the sense of a theoretical framework in Tentura's analytical texts |
| walk | блуждание | | Random walk from the ego along positive edges |
| walks per ego | число блужданий на эго | W | `walks_per_ego`, service setting `MERITRANK_NUM_WALKS`; the score denominator |
| continuation probability | вероятность продолжения блуждания | α | Probability that a walk takes another step |
| visits, hits | визиты, хиты | `n_C` | Probability that a walk visits C. A node counts at most once per walk (`Counter::increment_unique_counts`), so `n_C` is not an expected visit count. Not Tentura's *engagement* (вовлечение, доля вовлечения) |
| negative edge ("minus") | отрицательное ребро, минус | `w < 0` | Encodes a wall (J-D8); User→User only (J-D18) |
| wall | стена | `B ∈ D_A` | A node that absorbs walks in A's frame |
| wall set | множество стен | `D_A` | A's walls. Phase 1: exactly A's own negative out-edges |
| wall strength, absorption probability | сила стены, вероятность поглощения | `d_A(B)` | `min(|w|, 1)`, applied on every entry into B (J-D16). The notebook's slider `s` is `|w|` |
| effective strength | эффективная сила стены | `d_eff` | Absorption probability of a walk entering B, re-entries included: `d / (1 − (1 − d)·r)`, r = return probability |
| hard / soft wall | жёсткая / мягкая стена | `d = 1` / `d < 1` | |
| absorbed walk, poisoned walk | поглощённое блуждание, отравленное блуждание | `absorbed_at` | One walk under two names: *absorbed* names the event, *poisoned* names its accounting status |
| absorption-ahead probability | — | `q_C` | Probability that a walk at C is later **absorbed** at a wall. Not merely that it reaches a walled node: for a soft wall these differ |
| prefix | префикс | | The distinct nodes an absorbed walk visited up to and including the absorbing arrival, ego excluded |
| withholding | неначисление | (`λ = 0`) | An absorbed walk credits nothing |
| backward discredit | обратная дискредитация | (`λ > 0`) | An absorbed walk also subtracts blame along its prefix |
| discredit weight | вес обратной дискредитации | λ | Global |
| blame | вина | `−λ·b` | What a prefix node receives, once per absorbed walk |
| blame weight | вес вины | `b` | `γ^k`, k = steps from the node's visit nearest to the absorbing arrival; `b = 1` at the wall itself |
| blame decay | затухание вины | γ | Global |
| blame radius | радиус вины | | *Whole prefix* (γ-decayed) or *direct voucher only*. Not the limit γ → 0: that concentrates blame on the wall node itself |
| voucher, direct voucher | поручитель, прямой поручитель | | A node that routes flow to B; the direct voucher is the last node before B. In Tentura an inviter is a voucher by construction (an invite creates a bidirectional edge) |
| accuser, issuer | обвинитель | | The node whose negative edge created the wall |
| accusation | обвинение | | Issuing a negative edge. Distinct from blame (вина) |
| distrust inheritance | наследование недоверия | | Walls taken over from trusted nodes. Phase 2 (§10) |
| forward penalty | штраф вперёд | | The current semantics |
| curse attack | curse-attack | | J-P1 |
| normalisation trap | ловушка нормировки | | Dividing by realised hits instead of a fixed denominator |
| walk mass reaching B | масса блужданий, доходящих до B | `M_B` | Expected number of distinct nodes (baseline, no wall, ego excluded) on the walks that reach B at least once |
| reverse score | обратный скор | | The viewer's score in a peer's frame (`reverse_score` in `mr_mutual_scores`; Tentura's `reverse_mr`) |
| ego generation | поколение эго | | Counter bumped whenever an ego's walks change; keys derived caches (§6, Dependencies) |
| sync barrier | барьер синхронизации | `mr_sync` | The point after which every earlier write is visible in score reads (R21) |
| poisoned honeypot | отравленная приманка | | See J-D11 |
| hub problem | проблема хаба | | `q` accumulating at a hub across many walls |

---

## 3. Scope

**Phase 1 (this document):**

- Negative User→User weight = wall; absorption on every entry; withholding; backward discredit with
  γ and blame radius.
- Denominator W.
- Negative edges excluded from walk transitions and from out-weight normalisation (walks and
  `mr_graph`).
- Walls integrated into the optimized incremental invalidation.
- Walls stored exactly: exempt from VSIDS scaling and pruning.
- Walls in every context subgraph; bulk and incremental writes equivalent.
- Configuration of λ, γ, blame radius and seed.

**Dependency, separate track (lands first):** service consistency — replica copies, sync barrier,
cache redesign, reverse scores, owned RNG (§6, Dependencies; J-D22).

**Out of scope:**

- **Distrust inheritance** — phase 2, `Intersubjective/meritrank-rust#86`, J-D7.
- **Wall kinds, time decay of walls, publication timing** — the application's job (§4).
- **Reconciling trust and a wall on the same pair** — impossible by encoding; the application
  folds (J-D8).
- **Hiding walls from clients** — the application's job (J-D23).
- **Removing ownership logic from MR** — separate track; walls do not depend on it (J-D18).

---

## 4. Division of responsibility

The pipeline is client → application → MR → application → client. Application-specific logic
stays in the application, where it is cheap to change; MR stays a generic walk engine and knows
nothing about episodes, messages or friends.

| Application (e.g. Tentura) | MeritRank |
|---|---|
| Decides which walls exist and their `d` levels (quantized) | Stores walls as negative User→User edges |
| Folds its two-dimensional state (trust + wall) into one signed weight per pair | Never sees both on one pair |
| Publishes only walls that justify discredit: the walled node's own behaviour, and bans (J-D9) | One global λ and γ; kind-agnostic |
| Handles effects of the ego's own actions (re-ranking its own list) | Nothing |
| Time decay of walls: re-publishes `d` or deletes the edge | No decay field for walls |
| Quantizes publication in time (probing mitigation) | Applies each change as it arrives |
| Friends always visible; ban cascade by invite genealogy; voucher alerts | Nothing |
| Hides negative edges and raw reverse scores from clients (`positive_only = true` in its own graph endpoints) | Returns what it stores; no read filtering (J-D23) |
| Calibrates λ on a dump and enables discredit | Ships with λ = 0 (J-D24) |

Informational — Tentura's mapping:

| Tentura state of pair A→B | Weight published |
|---|---|
| ban | `−1` (today Tentura publishes 0; −1 is on its §17.4 checklist) |
| trust > 0 and a contact-noise wall ("ты шумишь") | the trust weight; the wall is not published |
| no trust, contact-noise wall | `−d`, a quantized level strictly below 1 |
| "I just contacted B" ("я его дёргал") | nothing — handled inside Tentura |
| nothing | 0 (edge deleted) |

A reverse score reflects the peer's walls against the viewer (a wall drives the viewer's score in
the peer's frame to ≤ 0). An application that wants walls unobservable must not show raw reverse
scores or reverse clusters to clients.

---

## 5. API contract

No function is added, removed or re-signatured.

| Function | Change |
|---|---|
| `mr_put_edge(src, dst, weight, context, index)` | A negative `weight` on a User→User edge in the null context means a wall (R1). Any other negative write is rejected (R2) |
| `mr_delete_edge(src, dst, context)` | Deleting a negative edge removes the wall |
| `mr_node_score`, `mr_scores`, `mr_mutual_scores` | Semantics per §6; signatures unchanged. Scale changes by ≈`L̄` (R11) |
| `mr_graph` | Positive weights normalised by the positive out-weight (R24). Otherwise unchanged; negative edges are not filtered (J-D23) |
| `mr_neighbors`, `mr_connected`, `mr_edgelist`, `mr_nodelist` | Unchanged |
| `mr_fetch_new_edges` | Not implemented in the service (`NotImplemented`); unchanged |
| `mr_bulk_load_edges` | Applies R1–R3 and R19; rejects an invalid batch as a whole (R20) |
| `mr_sync` | Sync barrier (R21), provided by the consistency track; declared `VOLATILE` |

---

## 6. Requirements

"MUST" is binding; "SHOULD" may be deviated from with a written reason in the journal.

### Encoding

- **R1.** A weight `w < 0` on the null-context User→User edge `A→B` MUST mean `B ∈ D_A` with
  `d_A(B) = min(|w|, 1)`. Any finite negative weight is a wall, however small. Positive weights keep
  their meaning; weight exactly 0 deletes the edge, and with it the wall.
- **R2.** Every write path MUST reject a negative weight when the context is not the null context
  or when either endpoint is not a User node (J-D18). Rejection is an error, not a silent clamp.
  Reason: walls belong to no context by construction (MR's null context is last-write-wins, and
  User→User writes fan out to every context whatever their `context` field says; J-D25), and
  phase 1 has no semantics for walls on other node kinds.
- **R3.** `mr_put_edge` MUST reject a negative self-edge `A→A`.
- **R19.** Walls MUST be stored exactly. A negative edge MUST be exempt from VSIDS: no magnitude
  scaling by `index`, no participation in the node's rescale, no pruning by the deletion
  threshold, and no contribution to the node's min/max tracking (`service/src/vsids.rs`,
  `service/src/aug_graph/edges.rs::set_edge_by_id`, `apply_edge_rescales_and_deletions`).
  `d_A(B)` MUST equal the published `min(|w|, 1)` whatever the `index`, across any rescale of A's
  positive edges and across reloads. A wall write MUST NOT change any positive edge weight other
  than the replaced positive edge of the same pair in a sign transition. The core MUST NOT treat a
  small negative weight as a deletion (today `|w| ≤ 1e-6` is one, `core/src/rank.rs:205`); a write
  with `|w| ≤ ε` to an absent edge MUST be a no-op, not a panic (J-D20). Reason: positive weights are
  normalised, so VSIDS scaling cancels for them; `d` is not normalised, so scaling would silently
  weaken a wall and pruning would delete a soft one (J-D15).
- **R20.** `mr_bulk_load_edges` and every other write path MUST apply R1–R3 and R19 exactly as
  `mr_put_edge` does. A batch containing an invalid wall MUST be rejected as a whole, before any
  mutation (today the bulk path clears subgraphs before looking at the edges,
  `service/src/state_manager.rs:410`).

### Walk mechanics

- **R4.** Walk transitions MUST use positive edges only. Transition probabilities MUST be
  normalised over the node's positive out-weights only: a negative edge dilutes nothing. This
  fixes today's side effect where issuing a minus drains `|w⁻|/Σ|w|` of the accuser's own walk
  mass.
- **R5.** Every time a walk of ego A steps into `B ∈ D_A`, it MUST be absorbed with probability
  `d_A(B)`: it terminates and records `absorbed_at = B`. Otherwise it continues along B's positive
  edges. **Every entry is an independent trial** (J-D16): the walk's state is its current node
  only. Consequence, accepted: through a cycle back to itself B raises the effective strength of
  its own wall to `d_eff = d / (1 − (1 − d)·r) ≤ 1` (see A1).
- **R6.** Phase 1: `D_A` MUST consist of A's own negative out-edges only. Negative edges of other
  nodes MUST have no effect on A's frame (A9).
- **R7.** Walls MUST apply in every context subgraph in which A's null-context user edges take
  part. The service fans User→User edges into every context (`service/src/state_manager.rs:833`,
  bulk path around line 440; test `context_aggregate_user_edges_dup`), and walls MUST follow the
  same fan-out, including into contexts created after the wall was written: seeding a new context
  (`seed_context_from_aggregate`) MUST copy walls with their exact `d`, bypassing VSIDS. An
  incremental write, a restart reload and a bulk load MUST produce the same walls in every context.

### Scoring

- **R8.** A walk that is not absorbed MUST credit `+1` to each distinct node it visits, the ego
  included — once per walk, however many times the node is visited, as today
  (`Counter::increment_unique_counts`).
- **R9.** An absorbed walk MUST credit nothing to any node it visited, the ego included
  (withholding; J-D21). If `λ > 0`, every distinct node on its prefix — including B, excluding the
  ego — MUST receive `−λ·b`, once.
- **R10.** Blame weight MUST be `b = γ^k`, where k is the number of steps from the node's visit
  nearest to the absorbing arrival (so a node visited several times takes its largest `b`;
  `k = 0`, `b = 1` for B itself). In blame-radius mode *direct voucher only*, `b = 1` for B and
  for the node visited immediately before the absorbing arrival, and `b = 0` elsewhere. The limit
  `γ → 0` of the whole-prefix mode is **not** this mode: it concentrates blame on B alone.
- **R11.** `score_A(X)` MUST be `(credits_X − λ·blame_X) / W`, where `blame_X = Σ b` over the
  absorbed walks and W is the number of walks per ego. The denominator MUST NOT depend on realised
  hits (J-D5, J-D17). This replaces `rank.rs::get_node_score` lines 108–112. Without walls a score
  is the probability that a walk of A visits X; all scores grow by ≈`L̄` against today's values,
  which Tentura does not depend on (J-D17). `score_A(A) = 1 − P(a walk of A is absorbed)`.
- **R12.** λ (`≥ 0`), γ (`∈ [0, 1]`) and the blame-radius mode MUST be global configuration
  (R23), never per edge or per ego. `λ = 0` means walls withhold only.
- **R13.** The ego MUST NOT receive blame.

Closed form for checking (one wall, `n_C` = probability that a walk visits C): for a node C
with `b = 1` on every absorbed walk, `score_A(C) = n_C·(1 − (1+λ)·q_C)`. In general,
`score_A(C) = n_C·(1 − q_C) − λ·E[b_C · 1{walk is absorbed and visits C}]`. Both hold for the ego
(`n = 1`, no blame term).

### Privacy

- **R14.** *Withdrawn* (J-D23). MR does not filter reads; hiding walls from clients is the
  application's job (§4).

### State and updates

- **R15.** The result MUST be a function of the current graph and the current walls only — state,
  not log (A5) — **in distribution**: different histories that reach the same graph give the same
  distribution of walks, not necessarily the same finite sample (J-D25).
- **R16.** After any change to `D_A` (add, change `d`, remove), A's stored walks MUST be
  distributed exactly as if regenerated under the current `D_A`. Other egos' walks MUST NOT be
  touched (R6). The conforming scheme re-couples every arrival at B (R5), scanning each candidate
  walk from its first arrival at B (the `visits` index holds that position):
  - `d` rises from `d₀` to `d₁` (including a new wall, `d₀ = 0`): at each arrival the walk passed,
    it is absorbed with probability `(d₁ − d₀)/(1 − d₀)`; the first success truncates the walk
    after that arrival and drops the remainder (including a later absorption at another wall);
  - `d` falls from `d₀` to `d₁` (including removal, `d₁ = 0`): a walk absorbed at B stays absorbed
    with probability `d₁/d₀`; otherwise it continues from B as an unabsorbed walk would (the
    continuation trial α at B first);
  - a change of `|w|` above 1 does not change `d` and touches no walk.

  Candidate walks MUST be taken from `visits[B]` filtered by A's walk-id block or from A's W
  walks, whichever is smaller (J-D19). The whole contribution of a touched walk (credits and
  blame) is removed and re-added. A wall change MUST mark A's generation, even if A is evicted
  (§6, Dependencies). A **sign transition** on `A→B` (trust replaced by a wall, or back) is
  executed as two exact operations: the positive-edge deletion (or addition), which is an
  ordinary positive-edge change in every frame and therefore outside A9, and the wall addition
  (or removal) in A's frame.
- **R21.** A write acknowledgement means only "queued". After `mr_sync` returns, every write
  acknowledged before the call MUST be reflected in every score read, in every context.
  Applications that version their MR state (Tentura's publication epoch) advance the version only
  after `mr_sync`. Provided by the consistency track (server-owned barrier; J-D22). `mr_sync` MUST
  be declared `VOLATILE`.

### Scores served by the API

- **R18.** Zero-opinion blending stays as it is and applies to every node, walled ones included:
  served score = `(1 − k)·score_A(X) + k·zero(X)`. R8–R13 and A1–A9 are stated on the core score
  `score_A(X)` (before blending). MR integration tests MUST either run with
  `MERITRANK_ZERO_OPINION_FACTOR=0` or assert on the core score (J-D14). A6 holds on served scores
  too, because the zero-opinion term does not depend on walls. Tentura sets no zero opinion, so
  there blending reduces to `0.98·score` (J-D25). The existing setting
  `MERITRANK_OMIT_NEG_EDGES_SCORES` (default `false`) now drops the ego's walled nodes from score
  lists; it stays off for Tentura, which decides visibility itself.
- **R24.** `mr_graph` MUST normalise a node's positive edge weights by its positive out-weight
  (`pos_sum`), matching the transition probabilities of R4 (today `abs_sum`,
  `service/src/aug_graph/graph_read.rs:184`; J-D23).

### Configuration

- **R23.** The service MUST read, validate at start-up, and refuse to start on invalid values of
  (J-D24):

  | Setting | Meaning | Valid | Default |
  |---|---|---|---|
  | `MERITRANK_DISCREDIT_LAMBDA` | λ | finite, ≥ 0 | 0 |
  | `MERITRANK_BLAME_DECAY` | γ | [0, 1] | 0.8 |
  | `MERITRANK_BLAME_RADIUS` | `prefix` or `voucher` | enum | `prefix` |
  | `MERITRANK_SEED` | RNG seed (R22) | u64 | unset: random at start-up, shared by both buffer copies |

  All are start-up only. Start-up MUST also reject `α ≥ 1` (a walk on a cyclic graph never ends).

### Testing

- **R22.** Under `MERITRANK_SEED`, walk generation MUST be deterministic: the same seed and the
  same ordered operation sequence — lazy calculations and evictions included — give identical
  scores (J-D25). Tests that run without it MUST use statistical tolerances derived from W. Tests
  inspect walls through the administrative reads (`mr_edgelist`).

### Dependencies (service-consistency track, J-D22)

The feature relies on these, delivered separately and first:

- **C1. Replica copies.** One ordered operation sequence with a replay log, applied by both
  buffer copies; copies are exact replicas (same order, same seed). Order holds across contexts
  for fanned-out User→User writes. Readers acquire a copy without stalling behind the worker.
- **C2. Barrier.** Server-owned barrier in the sequence with forced publication; errors
  propagated; `mr_sync` `VOLATILE` (R21).
- **C3. Caches.** No score cache. Cluster bounds cached per copy, keyed by ego generation and
  zero-opinion revision. The core reports egos whose walks changed during an operation (all nested
  `set_edge` calls included, e.g. VSIDS rescales), plus `calculate` and `clear_ego`.
- **C4. Reverse scores** are computed by calculating the needed peers before answering, never
  served from a remembered value.
- **C5. RNG** owned by `MeritRank`, every draw through it; deterministic bulk ordering and score
  ties.

### Rollout

- **R17.** The switch MUST happen in one release, with no mixed mode. Before it, the application
  MUST have removed every legacy negative edge (J-D12), because afterwards each is a wall.

---

## 7. Acceptance tests

Each axiom is a test on the core score (R18). The notebook already checks A1 and A6 live; the
remaining axioms need graph fixtures in `core` tests.

| # | Test |
|---|---|
| A1 | Hard wall: changing any out-edge of B leaves `score_A(X)` unchanged for every `X ≠ B`. Soft wall: changing B's out-edges changes the score of a node not reachable from B only through extra absorption — re-entry into B (R5) or a wall reachable from B — hence only downward and never beyond its loss under a hard wall at B. Fixtures must include a cycle through B (checking `d_eff = d/(1 − (1 − d)·r)`) and a second wall behind B |
| A2 | Hard wall: a node reachable from A only through B scores 0. Soft wall: at most `(1 − d)` of its baseline score (exactly `(1 − d)` on the notebook's chain) |
| A3 | A node X with independent support and `q_X = 0` keeps a positive score (no collateral damage) |
| A4 | C's multiplier is monotone in `q_C`, and C's loss never exceeds `(1+λ)·n_C·q_C` — its investment in B, i.e. the baseline credit it would get from the walks later absorbed at B, plus at most λ times that as blame |
| A5 | Removing `C→B` restores C immediately |
| A6 | Walls never raise any score above its value in the same graph without walls. A per-step version ("adding one more wall never raises any score") is false: a wall upstream of an existing wall B makes B unreachable and lifts B's negative score toward 0 |
| A7 | Total loss in A's frame ≤ `(1+λ)·M_B` (walk mass reaching B, see glossary) |
| A8 | A's negative edge affects only frames that transitively trust A |
| A9 | Phase 1: adding, changing or removing a wall of A, with A's positive out-edges unchanged, changes no stored walk and no score in any frame other than A's. A sign transition is a positive-edge change and is exempt (R16). This also catches a regression of R4: a negative edge that entered normalisation would dilute A's out-flow in other frames |

Regression values from the notebook's default topology (α = 0.85, K = 6, m = 3, λ = 0.6,
γ = 0.8) that the implementation must reproduce qualitatively (the chain has no cycle through the
wall, so R5's per-entry trial does not change them):

| Configuration | Expected |
|---|---|
| hard wall, fixed denominator | A1 shift 0.00%, A6 gain 0.00% |
| hard wall, normalisation trap | control chain gains +6.7% (A6 fails) |
| soft wall `d = 0.5`, fixed denominator | A1 shift 0.00% |
| soft wall `d = 0.5`, normalisation trap | A1 shift ≈ 1.5% (fails) — why R11 is mandatory |

Incremental maintenance (statistical, incremental vs generated from scratch on the final graph,
per ego and node, `|z| < 5`, with a negative control proving sensitivity; the harness of
`core/tests/test_incremental_bias.rs`):

- Every mutation class — positive add, delete, reweight; wall add, strengthen, weaken, remove;
  sign transitions both ways — on a chain, a multi-ego star with a hub, and a cycle through B.
- Positive edge changes at B and behind B while a wall stands (the absorbed terminal is never
  re-coupled).
- Recalculation and `clear_ego` + `calculate` of an ego with absorbed walks (clearing resets
  `absorbed_at`, J-D20).
- The adversarial scenarios of `core/tests/test_incremental_adversarial*.rs`, rewritten for walls.
- A weight ratio above 2^53 on one node leaves invalidation probabilities correct (exact sums).

Accounting and storage:

- A positive cycle through C counts C once per walk (R8), with and without walls.
- A node visited several times on an absorbed walk takes the `b` of its visit nearest to the wall
  (R10).
- Blame accumulated incrementally equals a from-scratch recount (debug check).
- A soft wall keeps its exact `d` after bumped positive edges of the same node trigger a VSIDS
  rescale, a wall below the deletion threshold is not pruned, and a wall with `|w| ≤ 1e-6` is
  stored (R19).
- Incremental writes, a restart reload and a bulk load give identical walls in every context,
  including a context created after the wall (R7, R20).
- A bulk batch with one invalid wall changes nothing (R20).
- After `mr_sync`, a score read reflects every earlier write (R21); cluster bounds do not survive
  a change of the ego's walks (C3).
- With the seed set, two runs of the same operation sequence give identical scores (R22).

API:

- `mr_put_edge` with a negative weight in a named context, on an edge with a non-User endpoint,
  and a negative self-edge all fail (R2, R3).
- `mr_graph` weights equal `w / pos_sum` for a node with walls (R24).
- Invalid λ, γ, radius or `α ≥ 1` refuse start-up (R23).

---

## 8. Implementation map

| Concept | Code location |
|---|---|
| Transitions over positive edges only (R4) | `core/src/graph.rs::random_neighbor` — always positive; the `abs_distr_cache` goes away |
| One stepping function with the absorption trial (R5, R6) | `core/src/graph.rs::generate_walk_segment`, the `random < α` push in `core/src/rank.rs::set_edge_`, `Graph::extend_walk_in_case_of_edge_deletion` — all call it; `negative_continuation_mode` goes away; the ego is `walk.first_node()`, its walls `neg_edges[ego]` |
| Absorbed instead of a negative suffix | `core/src/random_walk.rs` — `negative_segment_start` → `absorbed_at: Option<NodeId>` (the last node); `positive_subsegment` / `negative_subsegment` collapse; `clear()` and `split_from` maintain it |
| Blame `Σ b`, λ at read (R9, R10) | new accounting step in `core/src/rank.rs`; f64 accumulator per (ego, node) |
| Denominator W (R11) | `core/src/rank.rs::get_node_score`, lines 108–112 |
| Positive-edge invalidation (J-D19) | `core/src/walk_storage.rs::decide_skip_invalidation_on_edge_addition` — one probability `w/(pos_sum + w)`, skip the terminal position of an absorbed walk; `rank.rs::set_edge_` |
| Wall change (R16) | new path in `core/src/rank.rs`; candidates from `walk_storage.rs::get_visits_through_node(B)` or A's walk block |
| Exact cached sums (J-D19) | `core/src/graph.rs::set_edge`, `remove_edge` — recompute `pos_sum` |
| Dirty egos (C3) | collected in `core/src/rank.rs` repair loops, drained by `service/src/aug_graph/absorb.rs::apply_op` |
| Encoding checks (R1–R3) | `psql-connector/src/lib.rs::mr_put_edge`, bulk, and the service write path |
| Exact wall storage (R19) | `service/src/vsids.rs`; `service/src/aug_graph/edges.rs::set_edge_by_id`, `apply_edge_rescales_and_deletions` — keep negative edges out of scaling, rescale, pruning and min/max; `core/src/rank.rs::set_edge_` deletion only on exact 0 |
| Bulk validation (R20), context fan-out and seeding (R7) | `service/src/state_manager.rs` (bulk path, `seed_context_from_aggregate`), `service/src/aug_graph/edges.rs` |
| `mr_graph` normalisation (R24) | `service/src/aug_graph/graph_read.rs:184` |
| Settings (R23) | `service/src/settings.rs` |
| Consistency track (C1–C5) | `service/src/state_manager.rs`, `aug_graph/{mod,scores,neighbors,absorb}.rs`, `psql-connector/src/{rpc,lib}.rs`, `core` RNG |

---

## 9. Rollout order

1. MR: the consistency track (C1–C5) lands in `main`.
2. Application: stop publishing negative trust weights and resync, so MR holds no negative edge
   (Tentura: clamp published weights to `≥ 0`).
3. Application: force `positive_only = true` in its own graph endpoints.
4. MR: release R1–R24 together, with λ = 0.
5. Application: start publishing walls (bans as `−1`, then soft walls).
6. Application: calibrate λ (and γ, radius) on a dump; enable discredit.

---

## 10. Open questions and risks

- **Hub problem.** `q` accumulates at a hub across many walls. Widespread soft walls make it more
  acute. Measure on `tentura_dump.sql.gz`.
- **λ and γ values.** Only a dump simulation can settle them.
- **Latency budget.** No number yet for publishing a daily batch of N wall changes (R16 plus the
  R21 barrier). Measure on the dump, then add an acceptance threshold.
- **Context subgraphs.** R7's verification item.
- **Distrust inheritance** — phase 2, `#86`. Requirements already fixed: computed in MR as a
  separate pass before the walks; weighted by inbound trust, never by the issuer's outgoing trust
  or a global aggregate; one step; capped; hard walls only or all walls proportionally to `d` is
  open. Inheriting soft walls is the only way to extend a spammer's loss of standing beyond his
  recipients' own frames to third parties who walk through them. With inheritance, A8 becomes a
  real boundary and needs three tests plus a separate one-step test (see the issue). Own positive
  edge versus inherited wall stays the application's concern. The per-entry trial (R5) applies to
  inherited walls as well.
- **Probing through one's own frame.** Irrelevant in phase 1 (A9). With inheritance, anyone who
  trusts A can read A's walls off diffs of their own frame. The application mitigates this by
  quantizing publication in time; inheriting hard walls only narrows it further.

---

## 11. References

- `NEGATIVE_EDGES_JOURNAL.md` — decisions J-D1…J-D25, axioms A1…A9, known attacks.
- `scripts/negative_edges_demo.ipynb` — linear-chain model, analytic vs Monte-Carlo.
- `core/tests/test_incremental_bias.rs`, `core/tests/test_incremental_adversarial.rs`,
  `core/tests/test_incremental_adversarial_fable.rs` — incremental-vs-fresh statistical harness
  and adversarial suites.
- `tentura/docs/plans/post-request-closure-social-design.md` — application design, §17.4 layer
  split.
- Guha, Kumar, Raghavan, Tomkins, "Propagation of Trust and Distrust", WWW 2004.
