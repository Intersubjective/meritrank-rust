# Feature: Negative Edges as Absorbing Walls

**Status: SPECIFIED, NOT IMPLEMENTED.** Phase 1 below is the agreed scope. The code in `core/`
still implements the forward-penalty semantics.

This document states *what* MeritRank must do: requirements, API contract and acceptance tests.
The reasoning behind each decision lives in `NEGATIVE_EDGES_JOURNAL.md`, cited here as
**J-P1, J-P2** (problems), **J-D1…J-D12** (decisions) and **A1…A9** (axioms). The interactive model is
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
| visits, hits | визиты, хиты | `n_C` | Expected visits to C per walk. In code: the `pos_hits` / `neg_hits` counters. Not Tentura's *engagement* (вовлечение, доля вовлечения) |
| negative edge ("minus") | отрицательное ребро, минус | `w < 0` | Encodes a wall (J-D8) |
| wall | стена | `B ∈ D_A` | A node that absorbs walks in A's frame |
| wall set | множество стен | `D_A` | A's walls. Phase 1: exactly A's own negative out-edges |
| wall strength, absorption probability | сила стены, вероятность поглощения | `d_A(B)` | `min(|w|, 1)`. The notebook's slider `s` is `|w|` |
| hard / soft wall | жёсткая / мягкая стена | `d = 1` / `d < 1` | |
| absorbed walk, poisoned walk | поглощённое блуждание, отравленное блуждание | `absorbed_at` | One walk under two names: *absorbed* names the event, *poisoned* names its accounting status |
| absorption-ahead probability | — | `q_C` | Probability that a walk at C is later **absorbed** at a wall. Not merely that it reaches a walled node: for a soft wall these differ |
| prefix | префикс | | The visits of an absorbed walk up to and including the absorbing arrival, ego excluded |
| withholding | неначисление | (`λ = 0`) | An absorbed walk credits nothing |
| backward discredit | обратная дискредитация | (`λ > 0`) | An absorbed walk also subtracts blame along its prefix |
| discredit weight | вес обратной дискредитации | λ | Global |
| blame | вина | `−λ·b` | What one prefix visit receives |
| blame weight | вес вины | `b` | `γ^k`, k = steps from the visit to the absorbing arrival; `b = 1` at the wall itself |
| blame decay | затухание вины | γ | Global |
| blame radius | радиус вины | | *Whole prefix* (γ-decayed) or *direct voucher only* |
| voucher, direct voucher | поручитель, прямой поручитель | | A node that routes flow to B; the direct voucher is the last node before B. In Tentura an inviter is a voucher by construction (an invite creates a bidirectional edge) |
| accuser, issuer | обвинитель | | The node whose negative edge created the wall |
| accusation | обвинение | | Issuing a negative edge. Distinct from blame (вина) |
| distrust inheritance | наследование недоверия | | Walls taken over from trusted nodes. Phase 2 (§10) |
| forward penalty | штраф вперёд | | The current semantics |
| curse attack | curse-attack | | J-P1 |
| normalisation trap | ловушка нормировки | | Dividing by realised hits instead of a fixed denominator |
| fixed denominator | фиксированный знаменатель | W | Number of walks per ego |
| walk mass reaching B | масса блужданий, доходящих до B | `M_B` | Expected visits (baseline, no wall, ego excluded) summed over walks that reach B at least once |
| poisoned honeypot | отравленная приманка | | See J-D11 |
| hub problem | проблема хаба | | `q` accumulating at a hub across many walls |

---

## 3. Scope

**Phase 1 (this document):**

- Negative weight = wall; absorption; withholding; backward discredit with γ and blame radius.
- Fixed denominator.
- Negative edges excluded from walk transitions and from out-weight normalisation.
- Walls are private in the read API.
- Walk-storage invalidation for wall changes.

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
| `mr_graph`, `mr_neighbors`, `mr_connected` | Never return negative edges, whatever `positive_only` or filters say (R14) |
| `mr_edgelist`, `mr_nodelist`, `mr_fetch_new_edges`, `mr_bulk_load_edges`, sync | Unchanged; administrative, may carry negative edges |

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

### Walk mechanics

- **R4.** Walk transitions MUST use positive edges only. Transition probabilities MUST be
  normalised over the node's positive out-weights only: a negative edge dilutes nothing. This
  fixes today's side effect where issuing a minus drains `|w⁻|/Σ|w|` of the accuser's own walk
  mass.
- **R5.** When a walk of ego A steps into `B ∈ D_A`, it MUST be absorbed with probability
  `d_A(B)`: it terminates and records `absorbed_at = B`. Otherwise it continues along B's positive
  edges. Every arrival at a wall is an independent absorption trial.
- **R6.** Phase 1: `D_A` MUST consist of A's own negative out-edges only. Negative edges of other
  nodes MUST have no effect on A's frame (A9).
- **R7.** Walls apply in every subgraph in which A's user edges take part. *Verify* whether the
  null-context user edges are copied into context subgraphs
  (`service/src/state_manager.rs`, test `context_aggregate_user_edges_dup`). If they are, walls
  follow automatically; if not, decide explicitly and record the decision in the journal.

### Scoring

- **R8.** A walk that is not absorbed MUST credit `+1` per visit, as today.
- **R9.** An absorbed walk MUST credit nothing for any of its visits (withholding). If `λ > 0`,
  every visit on its prefix — including the absorbing arrival at B, excluding the ego — MUST
  receive `−λ·b`.
- **R10.** Blame weight per visit MUST be `b = γ^k`, where k is the number of steps from that
  visit to the absorbing arrival (`k = 0`, `b = 1` for B itself). In blame-radius mode *direct
  voucher only*, `b = 1` for B and for the visit immediately before the absorbing arrival, and
  `b = 0` elsewhere.
- **R11.** `score_A(X)` MUST be `(credits_X − blame_X) / W`, where W is the number of walks per
  ego. The denominator MUST NOT depend on realised hits (J-D5). This replaces
  `rank.rs::get_node_score` lines 108–112.
- **R12.** λ (`≥ 0`), γ (`∈ [0, 1]`) and the blame-radius mode MUST be global configuration,
  never per edge or per ego. `λ = 0` means walls withhold only. Suggested starting point from the
  journal: `λ ≈ 0.3–0.5`. Final values are calibrated by the application on a real dump.
- **R13.** The ego MUST NOT receive blame.

Closed form for checking (whole-prefix mode, one wall): for a node C whose visits on absorbed
walks all carry `b = 1`, `score_A(C) = n_C·(1 − (1+λ)·q_C)`. In general,
`score_A(C) = n_C·(1 − q_C) − λ·E[Σ b over C's visits on absorbed walks]`.

### Privacy

- **R14.** `mr_graph`, `mr_neighbors` and `mr_connected` MUST NOT return negative edges or count
  them, regardless of `positive_only`, `kind` or weight filters. Once a negative edge is a wall,
  reading it tells a third party who walls whom (J-D10).

### State and updates

- **R15.** The result MUST be a function of the current graph and the current walls only — state,
  not log (A5).
- **R16.** After any change to `D_A` (add, change `d`, remove), A's stored walks MUST be
  distributed exactly as if regenerated under the current `D_A`. Other egos' walks MUST NOT be
  touched (R6). One conforming scheme, applied at a walk's successive arrivals at B:
  - `d` rises from `d₀` to `d₁`: a walk that passed an arrival unabsorbed is absorbed there with
    probability `(d₁ − d₀)/(1 − d₀)`, and its remainder is dropped;
  - `d` falls from `d₀` to `d₁`: a walk absorbed at an arrival stays absorbed with probability
    `d₁/d₀`; otherwise it is continued from B.
  `walk_storage::get_visits_through_node(B)` yields the candidate walks; accounting moves between
  `pos_hits` and `neg_hits` accordingly.

### Rollout

- **R17.** The switch MUST happen in one release, with no mixed mode. Before it, the application
  MUST have removed every legacy negative edge (J-D12), because afterwards each is a wall with
  discredit.

---

## 7. Acceptance tests

Each axiom is a test. The notebook already checks A1 and A6 live; the remaining axioms need
graph fixtures in `core` tests.

| # | Test |
|---|---|
| A1 | Hard wall: changing any out-edge of B leaves `score_A(X)` unchanged for every `X ≠ B`. Soft wall: unchanged for every X not reachable from B |
| A2 | Hard wall: a node reachable from A only through B scores 0. Soft wall: at most `(1 − d)` of its baseline score (exactly `(1 − d)` on the notebook's chain) |
| A3 | A node X with independent support and `q_X = 0` keeps a positive score (no collateral damage) |
| A4 | C's multiplier is monotone in `q_C`, and C's loss never exceeds its investment in B |
| A5 | Removing `C→B` restores C immediately |
| A6 | Adding a wall never increases any score |
| A7 | Total loss in A's frame ≤ `(1+λ)·M_B` (walk mass reaching B, see glossary) |
| A8 | A's negative edge affects only frames that transitively trust A |
| A9 | Phase 1: adding, changing or removing a wall of A changes no score in any frame other than A's. This also catches a regression of R4: a negative edge that entered normalisation would dilute A's out-flow in other frames |

Regression values from the notebook's default topology (α = 0.85, K = 6, m = 3, λ = 0.6,
γ = 0.8) that the implementation must reproduce qualitatively:

| Configuration | Expected |
|---|---|
| hard wall, fixed denominator | A1 shift 0.00%, A6 gain 0.00% |
| hard wall, normalisation trap | control chain gains +6.7% (A6 fails) |
| soft wall `d = 0.5`, fixed denominator | A1 shift 0.00% |
| soft wall `d = 0.5`, normalisation trap | A1 shift ≈ 1.5% (fails) — why R11 is mandatory |

Privacy and API:

- `mr_graph(..., positive_only = false)` over a graph containing walls returns no negative edge
  (R14).
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
| Read filtering (R14) | `psql-connector/src/lib.rs::mr_graph`, `mr_neighbors`, `mr_connected` and the service handlers behind them |

---

## 9. Rollout order

1. Application: stop publishing negative trust weights and resync, so MR holds no negative edge
   (Tentura: clamp published weights to `≥ 0`).
2. Application: force `positive_only = true` in its own graph endpoints.
3. MR: release R1–R17 together.
4. Application: start publishing walls (bans as `−1`, then soft walls).

---

## 10. Open questions and risks

- **Hub problem.** `q` accumulates at a hub across many walls. Widespread soft walls make it more
  acute. Measure on `tentura_dump.sql.gz`.
- **λ and γ values.** Only a dump simulation can settle them.
- **Context subgraphs.** R7's verification item.
- **Distrust inheritance** — phase 2, `#86`. Requirements already fixed: computed in MR as a
  separate pass before the walks; weighted by inbound trust, never by the issuer's outgoing trust
  or a global aggregate; one step; capped; hard walls only or all walls proportionally to `d` is
  open. With inheritance, A8 becomes a real boundary and needs three tests plus a separate
  one-step test (see the issue). Own positive edge versus inherited wall stays the application's
  concern.
- **Probing through one's own frame.** Irrelevant in phase 1 (A9). With inheritance, anyone who
  trusts A can read A's walls off diffs of their own frame. The application mitigates this by
  quantizing publication in time; inheriting hard walls only narrows it further.

---

## 11. References

- `NEGATIVE_EDGES_JOURNAL.md` — decisions J-D1…J-D12, axioms A1…A9, known attacks.
- `scripts/negative_edges_demo.ipynb` — linear-chain model, analytic vs Monte-Carlo.
- `tentura/docs/plans/post-request-closure-social-design.md` — application design, §17.4 layer
  split.
- Guha, Kumar, Raghavan, Tomkins, "Propagation of Trust and Distrust", WWW 2004.
