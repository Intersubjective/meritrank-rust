# Feature: Negative Edges as Absorbing Walls

**Status: SPECIFIED, NOT IMPLEMENTED.** Phase 1 below is the agreed scope. The code in `core/`
still implements the forward-penalty semantics.

This document states *what* MeritRank must do: requirements, API contract and acceptance tests.
The reasoning behind each decision lives in `NEGATIVE_EDGES_JOURNAL.md`, cited here as
**J-P1, J-P2** (problems), **J-D1…J-D15** (decisions) and **A1…A9** (axioms).
Requirement IDs are stable: a new requirement takes the next free number, so numbers within a
section need not be consecutive. The interactive model is
`scripts/negative_edges_demo.ipynb`. The application-side design (Tentura) is
`tentura/docs/plans/post-request-closure-social-design.md`, §17.4. Where this document and the
journal differ, this document is current.

---

## 1. Summary

Today a negative edge is a *traversable* edge: a walk that takes it continues along the
distrusted node's own positive edges and penalises everything it meets from there on. Bob
therefore chooses whom his distrusters punish (the curse attack), and the more people distrust
him, the more he can aim (J-P1, J-P2).

The feature replaces this with an **absorbing wall**. A negative edge `A→B` makes B a wall in
A's frame: A's walks travel only along positive edges, and a walk that steps into B is absorbed
with probability `d`. An absorbed walk credits nobody (withholding). Optionally (`λ > 0`), it
also subtracts blame from the nodes that routed it to B (backward discredit). Bob's own edges
never matter in A's frame beyond the `(1 − d)` share that passes through him.

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
| continuation probability | вероятность продолжения блуждания | α | Probability that a walk takes another step |
| visits, hits | визиты, хиты | `n_C` | Probability that a walk visits C. A node counts at most once per walk (`Counter::increment_unique_counts`), so `n_C` is not an expected visit count. In code: the `pos_hits` / `neg_hits` counters. Not Tentura's *engagement* (вовлечение, доля вовлечения) |
| negative edge ("minus") | отрицательное ребро, минус | `w < 0` | Encodes a wall (J-D8) |
| wall | стена | `B ∈ D_A` | A node that absorbs walks in A's frame |
| wall set | множество стен | `D_A` | A's walls. Phase 1: exactly A's own negative out-edges |
| wall strength, absorption probability | сила стены, вероятность поглощения | `d_A(B)` | `min(|w|, 1)`. The notebook's slider `s` is `|w|` |
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
| fixed denominator | фиксированный знаменатель | W | Number of walks per ego |
| walk mass reaching B | масса блужданий, доходящих до B | `M_B` | Expected number of distinct nodes (baseline, no wall, ego excluded) on the walks that reach B at least once |
| sync barrier | барьер синхронизации | `mr_sync` | The point after which every earlier write is visible in score reads (R21) |
| poisoned honeypot | отравленная приманка | | See J-D11 |
| hub problem | проблема хаба | | `q` accumulating at a hub across many walls |

---

## 3. Scope

**Phase 1 (this document):**

- Negative weight = wall; absorption; withholding; backward discredit with γ and blame radius.
- Fixed denominator.
- Negative edges excluded from walk transitions and from out-weight normalisation.
- Walls are private in the read API.
- Walls stored exactly: exempt from VSIDS scaling and pruning.
- Walls in every context subgraph; bulk and incremental writes equivalent.
- Walk-storage and cache invalidation for wall changes; a sync barrier.
- Deterministic test mode.

**Out of scope:**

- **Distrust inheritance** — phase 2, `Intersubjective/meritrank-rust#86`, J-D7.
- **Wall kinds, time decay of walls, publication timing** — the application's job (§4).
- **Reconciling trust and a wall on the same pair** — impossible by encoding; the application
  folds (J-D8).

---

## 4. Division of responsibility

The pipeline is client → application → MR → application → client. Application-specific logic
stays in the application, where it is cheap to change; MR stays a generic walk engine and knows
nothing about episodes, messages or friends.

| Application (e.g. Tentura) | MeritRank |
|---|---|
| Decides which walls exist and their `d` levels (quantized) | Stores walls as negative edges |
| Folds its two-dimensional state (trust + wall) into one signed weight per pair | Never sees both on one pair |
| Publishes only walls that justify discredit: the walled node's own behaviour, and bans (J-D9) | One global λ and γ; kind-agnostic |
| Handles effects of the ego's own actions (re-ranking its own list) | Nothing |
| Time decay of walls: re-publishes `d` or deletes the edge | No decay field for walls |
| Quantizes publication in time (probing mitigation) | Applies each change as it arrives |
| Friends always visible; ban cascade by invite genealogy; voucher alerts | Nothing |
| Forces `positive_only = true` in its own graph endpoints (defence in depth) | Filters negative edges from ego-facing reads (R14) |

Informational — Tentura's mapping:

| Tentura state of pair A→B | Weight published |
|---|---|
| ban | `−1` |
| trust > 0 and a contact-noise wall ("ты шумишь") | the trust weight; the wall is not published |
| no trust, contact-noise wall | `−d`, a quantized level strictly below 1 |
| "I just contacted B" ("я его дёргал") | nothing — handled inside Tentura |
| nothing | 0 (edge deleted) |

---

## 5. API contract

No function is added, removed or re-signatured.

| Function | Change |
|---|---|
| `mr_put_edge(src, dst, weight, context, index)` | A negative `weight` in the null context now means a wall (R1). A negative `weight` in a named context is rejected (R2) |
| `mr_delete_edge(src, dst, context)` | Deleting a negative edge removes the wall |
| `mr_node_score`, `mr_scores`, `mr_mutual_scores` | Semantics per §6; signatures unchanged |
| `mr_graph`, `mr_neighbors`, `mr_connected`, `mr_fetch_new_edges` | Never return negative edges, whatever `positive_only` or filters say (R14). `mr_fetch_new_edges` is ego-facing: Tentura exposes it to clients through Hasura |
| `mr_bulk_load_edges` | Applies R1–R3 and R19; rejects an invalid batch as a whole (R20) |
| `mr_sync` | Sync barrier (R21) |
| `mr_edgelist`, `mr_nodelist` | Unchanged; administrative, may carry negative edges. The only sanctioned way for tests to inspect walls (R22) |

---

## 6. Requirements

"MUST" is binding; "SHOULD" may be deviated from with a written reason in the journal.

### Encoding

- **R1.** A weight `w < 0` on the null-context edge `A→B` MUST mean `B ∈ D_A` with
  `d_A(B) = min(|w|, 1)`. Positive weights keep their meaning; weight 0 deletes the edge, and with
  it the wall.
- **R2.** `mr_put_edge` MUST reject a negative weight in a named context. (The null context is
  the sum of the contexted ones, so a contexted wall would be summed into trust.)
- **R3.** `mr_put_edge` MUST reject a negative self-edge `A→A`.
- **R19.** Walls MUST be stored exactly. A negative edge MUST be exempt from VSIDS: no magnitude
  scaling by `index`, no participation in the node's rescale, no pruning by the deletion
  threshold, and no contribution to the node's min/max tracking (`service/src/vsids.rs`,
  `service/src/aug_graph/edges.rs::set_edge_by_id`, `apply_edge_rescales_and_deletions`).
  `d_A(B)` MUST equal the published `min(|w|, 1)` whatever the `index`, across any rescale of A's
  positive edges and across reloads. A wall write MUST NOT change any positive edge weight.
  Reason: positive weights are normalised, so VSIDS scaling cancels for them; `d` is not
  normalised, so scaling would silently weaken a wall and pruning would delete a soft one (J-D15).
- **R20.** `mr_bulk_load_edges` and every other write path MUST apply R1–R3 and R19 exactly as
  `mr_put_edge` does. A batch containing an invalid wall MUST be rejected as a whole, before any
  mutation.

### Walk mechanics

- **R4.** Walk transitions MUST use positive edges only. Transition probabilities MUST be
  normalised over the node's positive out-weights only: a negative edge dilutes nothing. This
  fixes today's side effect where issuing a minus drains `|w⁻|/Σ|w|` of the accuser's own walk
  mass.
- **R5.** When a walk of ego A first steps into `B ∈ D_A`, it MUST be absorbed with probability
  `d_A(B)`: it terminates and records `absorbed_at = B`. Otherwise it continues along B's positive
  edges and passes B freely on any later arrival. **One absorption trial per walk per wall**
  (J-D13): a cycle through B cannot re-roll the trial, so B's own out-edges cannot raise the
  absorption probability of the nodes that route to B.
- **R6.** Phase 1: `D_A` MUST consist of A's own negative out-edges only. Negative edges of other
  nodes MUST have no effect on A's frame (A9).
- **R7.** Walls MUST apply in every context subgraph in which A's null-context user edges take
  part. The service fans those edges into the contexts (`service/src/state_manager.rs`, bulk path
  around line 440; test `context_aggregate_user_edges_dup`), and walls MUST follow the same
  fan-out, including into contexts created after the wall was written. An incremental write, a
  restart reload and a bulk load MUST produce the same walls in every context.

### Scoring

- **R8.** A walk that is not absorbed MUST credit `+1` to each distinct node it visits — once per
  walk, however many times the node is visited, as today (`Counter::increment_unique_counts`).
- **R9.** An absorbed walk MUST credit nothing to any node it visited (withholding). If `λ > 0`,
  every distinct node on its prefix — including B, excluding the ego — MUST receive `−λ·b`, once.
- **R10.** Blame weight MUST be `b = γ^k`, where k is the number of steps from the node's visit
  nearest to the absorbing arrival (so a node visited several times takes its largest `b`;
  `k = 0`, `b = 1` for B itself). In blame-radius mode *direct voucher only*, `b = 1` for B and
  for the node visited immediately before the absorbing arrival, and `b = 0` elsewhere. The limit
  `γ → 0` of the whole-prefix mode is **not** this mode: it concentrates blame on B alone.
- **R11.** `score_A(X)` MUST be `(credits_X − blame_X) / W`, where W is the number of walks per
  ego. The denominator MUST NOT depend on realised hits (J-D5). This replaces
  `rank.rs::get_node_score` lines 108–112.
- **R12.** λ (`≥ 0`), γ (`∈ [0, 1]`) and the blame-radius mode MUST be global configuration,
  never per edge or per ego. `λ = 0` means walls withhold only. Suggested starting point from the
  journal: `λ ≈ 0.3–0.5`. Final values are calibrated by the application on a real dump.
- **R13.** The ego MUST NOT receive blame.

Closed form for checking (one wall, `n_C` = probability that a walk visits C): for a node C
with `b = 1` on every absorbed walk, `score_A(C) = n_C·(1 − (1+λ)·q_C)`. In general,
`score_A(C) = n_C·(1 − q_C) − λ·E[b_C · 1{walk is absorbed and visits C}]`.

### Privacy

- **R14.** `mr_graph`, `mr_neighbors`, `mr_connected` and `mr_fetch_new_edges` MUST NOT return
  negative edges or count them, regardless of `positive_only`, `kind` or weight filters. Once a
  negative edge is a wall, reading it tells a third party who walls whom (J-D10).

### State and updates

- **R15.** The result MUST be a function of the current graph and the current walls only — state,
  not log (A5).
- **R16.** After any change to `D_A` (add, change `d`, remove), A's stored walks MUST be
  distributed exactly as if regenerated under the current `D_A`. Other egos' walks MUST NOT be
  touched (R6). One conforming scheme, applied at a walk's first arrival at B (R5):
  - `d` rises from `d₀` to `d₁`: a walk that passed its first arrival unabsorbed is absorbed there
    with probability `(d₁ − d₀)/(1 − d₀)`, and its remainder is dropped;
  - `d` falls from `d₀` to `d₁`: a walk absorbed at B stays absorbed with probability `d₁/d₀`;
    otherwise it is continued from B, passing B freely on later arrivals.
  `walk_storage::get_visits_through_node(B)` yields the candidate walks; accounting moves between
  `pos_hits` and `neg_hits` accordingly. A wall change MUST also invalidate A's cached scores
  (`service/src/aug_graph/scores.rs`, `cached_scores`) and any cluster data derived from them.
  A **sign transition** on `A→B` (trust replaced by a wall, or back) changes A's positive
  out-edges: it MUST be invalidated as an ordinary positive-edge change, in every frame, and is
  therefore outside A9.
- **R21.** A write acknowledgement means only "queued" (`service/src/state_manager.rs`,
  `FanoutSender::send`). After `mr_sync` returns, every write acknowledged before the call MUST be
  reflected in every score read, in every context. Applications that version their MR state
  (Tentura's publication epoch) advance the version only after `mr_sync`.

### Scores served by the API

- **R18.** Zero-opinion blending stays as it is and applies to every node, walled ones included:
  served score = `(1 − k)·score_A(X) + k·zero(X)`. R8–R13 and A1–A9 are stated on the core score
  `score_A(X)` (before blending). MR integration tests MUST either run with
  `MERITRANK_ZERO_OPINION_FACTOR=0` or assert on the core score (J-D14). A6 holds on served scores
  too, because the zero-opinion term does not depend on walls. The existing setting
  `MERITRANK_OMIT_NEG_EDGES_SCORES` (default `false`) now drops the ego's walled nodes from score
  lists; it stays off for Tentura, which decides visibility itself.

### Testing

- **R22.** MR MUST offer a seed setting (proposed name `MERITRANK_SEED`) under which walk
  generation is deterministic for a given sequence of writes. Tests that run without it MUST use
  statistical tolerances derived from the walk count W. Tests inspect walls only through the
  administrative reads (`mr_edgelist`), never through ego-facing ones, so R14 is not weakened.

### Rollout

- **R17.** The switch MUST happen in one release, with no mixed mode. Before it, the application
  MUST have removed every legacy negative edge (J-D12), because afterwards each is a wall with
  discredit.

---

## 7. Acceptance tests

Each axiom is a test on the core score (R18). The notebook already checks A1 and A6 live; the
remaining axioms need graph fixtures in `core` tests.

| # | Test |
|---|---|
| A1 | Hard wall: changing any out-edge of B leaves `score_A(X)` unchanged for every `X ≠ B`. Soft wall: unchanged for every X not reachable from B, provided no other wall is reachable from B. If one is, the nodes routing to B take ordinary backward blame for it, bounded by the `(1 − d)` share passing B (J-D13). Fixtures must include a cycle through B and a second wall behind B |
| A2 | Hard wall: a node reachable from A only through B scores 0. Soft wall: at most `(1 − d)` of its baseline score (exactly `(1 − d)` on the notebook's chain) |
| A3 | A node X with independent support and `q_X = 0` keeps a positive score (no collateral damage) |
| A4 | C's multiplier is monotone in `q_C`, and C's loss never exceeds `(1+λ)·n_C·q_C` — its investment in B, i.e. the baseline credit it would get from the walks later absorbed at B, plus at most λ times that as blame |
| A5 | Removing `C→B` restores C immediately |
| A6 | Walls never raise any score above its value in the same graph without walls. A per-step version ("adding one more wall never raises any score") is false: a wall upstream of an existing wall B makes B unreachable and lifts B's negative score toward 0 |
| A7 | Total loss in A's frame ≤ `(1+λ)·M_B` (walk mass reaching B, see glossary) |
| A8 | A's negative edge affects only frames that transitively trust A |
| A9 | Phase 1: adding, changing or removing a wall of A, with A's positive out-edges unchanged, changes no score in any frame other than A's. A sign transition is a positive-edge change and is exempt (R16). This also catches a regression of R4: a negative edge that entered normalisation would dilute A's out-flow in other frames |

Regression values from the notebook's default topology (α = 0.85, K = 6, m = 3, λ = 0.6,
γ = 0.8) that the implementation must reproduce qualitatively:

| Configuration | Expected |
|---|---|
| hard wall, fixed denominator | A1 shift 0.00%, A6 gain 0.00% |
| hard wall, normalisation trap | control chain gains +6.7% (A6 fails) |
| soft wall `d = 0.5`, fixed denominator | A1 shift 0.00% |
| soft wall `d = 0.5`, normalisation trap | A1 shift ≈ 1.5% (fails) — why R11 is mandatory |

Accounting and storage:

- A positive cycle through C counts C once per walk (R8), with and without walls.
- A node visited several times on an absorbed walk takes the `b` of its visit nearest to the wall
  (R10).
- A soft wall keeps its exact `d` after bumped positive edges of the same node trigger a VSIDS
  rescale, and a wall below the deletion threshold is not pruned (R19).
- Incremental writes, a restart reload and a bulk load give identical walls in every context,
  including a context created after the wall (R7, R20).
- A bulk batch with one invalid wall changes nothing (R20).
- After `mr_sync`, a score read reflects every earlier write (R21); cached scores do not survive
  a wall change (R16).
- With the seed set, two runs of the same write sequence give identical scores (R22).

Privacy and API:

- `mr_graph(..., positive_only = false)`, `mr_neighbors`, `mr_connected` and `mr_fetch_new_edges`
  over a graph containing walls return no negative edge (R14).
- `mr_put_edge` with a negative weight in a named context and a negative self-edge both fail
  (R2, R3).

---

## 8. Implementation map

| Concept | Code location |
|---|---|
| Transitions over positive edges only (R4) | `core/src/graph.rs::random_neighbor` — always `positive_only`; normalise over positive weights |
| Absorption on `D_A` (R5, R6) | `core/src/graph.rs::generate_walk_segment` — `negative_continuation_mode` goes away |
| Absorbed instead of a negative suffix | `core/src/random_walk.rs` — `negative_segment_start: Option<usize>` → `absorbed_at: Option<NodeId>`; `positive_subsegment` / `negative_subsegment` collapse |
| Blame weights (R9, R10) | new accounting step in `core/src/rank.rs::calculate` |
| Fixed denominator (R11) | `core/src/rank.rs::get_node_score`, lines 108–112 — `total_hits` → `walks_per_ego` |
| Invalidation (R16) | `core/src/walk_storage.rs::get_visits_through_node(B)` |
| Encoding checks (R1–R3) | `psql-connector/src/lib.rs::mr_put_edge` and the service write path |
| Read filtering (R14) | `psql-connector/src/lib.rs::mr_graph`, `mr_neighbors`, `mr_connected`, `mr_fetch_new_edges` and the service handlers behind them (`service/src/aug_graph/graph_read.rs`, `neighbors.rs`) |
| Exact wall storage (R19) | `service/src/vsids.rs`; `service/src/aug_graph/edges.rs::set_edge_by_id`, `apply_edge_rescales_and_deletions` — keep negative edges out of scaling, rescale, pruning and min/max |
| Bulk validation (R20), context fan-out (R7) | `service/src/state_manager.rs` (bulk path), `service/src/aug_graph/edges.rs` (bulk setter) |
| Cache invalidation (R16) | `service/src/aug_graph/scores.rs` — `cached_scores` |
| Sync barrier (R21) | `psql-connector/src/lib.rs::mr_sync`, `service/src/state_manager.rs` |
| Seed (R22) | `service/src/settings.rs`; the RNG used by `core/src/graph.rs::generate_walk_segment` |

---

## 9. Rollout order

1. Application: stop publishing negative trust weights and resync, so MR holds no negative edge
   (Tentura: clamp published weights to `≥ 0`).
2. Application: force `positive_only = true` in its own graph endpoints.
3. MR: release R1–R22 together.
4. Application: start publishing walls (bans as `−1`, then soft walls).

---

## 10. Open questions and risks

- **Hub problem.** `q` accumulates at a hub across many walls. Widespread soft walls make it more
  acute. Measure on `tentura_dump.sql.gz`.
- **λ and γ values.** Only a dump simulation can settle them.
- **Latency budget.** No number yet for publishing a daily batch of N wall changes (R16
  invalidation plus R21 barrier). Measure on the dump, then add an acceptance threshold.
- **Context subgraphs.** R7's verification item.
- **Distrust inheritance** — phase 2, `#86`. Requirements already fixed: computed in MR as a
  separate pass before the walks; weighted by inbound trust, never by the issuer's outgoing trust
  or a global aggregate; one step; capped; hard walls only or all walls proportionally to `d` is
  open. Inheriting soft walls is the only way to extend a spammer's loss of standing beyond his
  recipients' own frames to third parties who walk through them. With inheritance, A8 becomes a real boundary and needs three tests plus a separate
  one-step test (see the issue). Own positive edge versus inherited wall stays the application's
  concern.
- **Probing through one's own frame.** Irrelevant in phase 1 (A9). With inheritance, anyone who
  trusts A can read A's walls off diffs of their own frame. The application mitigates this by
  quantizing publication in time; inheriting hard walls only narrows it further.

---

## 11. References

- `NEGATIVE_EDGES_JOURNAL.md` — decisions J-D1…J-D15, axioms A1…A9, known attacks.
- `scripts/negative_edges_demo.ipynb` — linear-chain model, analytic vs Monte-Carlo.
- `tentura/docs/plans/post-request-closure-social-design.md` — application design, §17.4 layer
  split.
- Guha, Kumar, Raghavan, Tomkins, "Propagation of Trust and Distrust", WWW 2004.
