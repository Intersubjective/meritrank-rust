# Design Journal: Negative Edges — Absorbing Wall

**Status: DESIGN AGREED, NOT IMPLEMENTED.** The code in `core/` still implements the forward-penalty
semantics described under "Problem" below. D16–D25 record the implementation design review of
2026-09-26; the service-consistency track (D22) lands in `main` before the feature.

Decision numbering is local to this document (`JOURNAL.md` covers the NNG→TCP migration and has
its own D1, D2, …).

Companion interactive demo: `scripts/negative_edges_demo.ipynb` (run via `scripts/run_demo.sh`).
Requirements, glossary and acceptance tests: `NEGATIVE_EDGES_FEATURE.md` (decisions here: D1–D25). This journal records
*why*; the feature document records *what*. Where they differ, the feature document is current.

---

## Problem

Current semantics (`core/src/graph.rs::generate_walk_segment`, `core/src/random_walk.rs`): a
negative edge is a *traversable edge*. A walk picks among all outgoing edges weighted by `|w|`;
on taking a negative edge it enters `negative_continuation_mode`, follows **the distrusted node's
own positive edges** from there on, and everything from that point forward is counted into
`neg_hits` and subtracted in `rank.rs::get_node_score`.

Two distinct defects follow.

**P1 — Curse attack.** The set of penalised nodes is Bob's out-neighbourhood, and Bob chooses it.
By adding positive edges he directs "curse mass" at any target. Since endorsement is unilateral,
anyone can damage anyone: create an account, attract one negative edge from a well-connected
user, then endorse the victims.

**P2 — Distrust confers power.** The more users distrust Bob, the more walks enter negative mode
at Bob, and the more curse mass he can aim. A troll is rewarded with amplification. This also
propagates without bound: any walk reaching a negative edge anywhere in the graph is poisoned, so
Bob can curse inside the frame of anyone who transitively trusts anyone who distrusts him.

---

## D1 — Selection criterion for "Bob's subnetwork"

**Context**: the goal is for a negative edge to push Bob's whole subnetwork away from Alice while
denying Bob any influence over Alice. This requires deciding what counts as "Bob's subnetwork".

**Decision**: adopt the criterion — *any definition of Bob's subnetwork that Bob can enlarge by a
unilateral action is an attack vector*. This filters the candidates:

| Candidate | Controlled by | Verdict |
|---|---|---|
| Bob's outgoing edges (whom he praises) | Bob | ✗ — this is exactly P1 |
| Mutual edges / community cluster around Bob | Bob in part (can force reciprocity) | ✗ — same attack, new wrapper |
| Static set "everyone who ever gave Bob a plus" | not Bob | ~ safe but global, unbounded blast radius, not weighted by relevance to Alice |
| **Trust flow from Alice that enters Bob** | Alice and her network | ✓ |

**Consequence**: the subnetwork must not be defined explicitly at all. Make Bob an absorbing wall
and the correct set emerges: everyone whose standing in Alice's frame depends on paths through
Bob. Bob's sybils, reachable only through him, collapse to zero; honest nodes he also endorses
keep their independent support and are untouched. Bob endorsing Alice's friends changes nothing
for Alice — neither boost nor penalty.

---

## D2 — A negative edge is a node property in the ego's frame, not a traversable edge

**Decision**: distrust becomes an ego-relative set `D_A` with weights `d_A(B) ∈ (0,1]`
(absorption probability; `d = 1` is a hard wall). Walks follow positive edges only. On stepping
into `B ∈ D_A`, with probability `d_A(B)` the walk terminates and is marked `poisoned`.

This is the whole fix for P1: Bob's outgoing edges are never traversed in Alice's frame, so
`∂score_A(X)/∂(anything Bob controls) = 0` for all `X ≠ B` by construction.

That holds exactly for a hard wall. A soft wall (`d < 1`) lets a `(1 − d)` share of the flow
through, so Bob still influences the nodes reachable from him — within that share. Nodes not
reachable from Bob stay unaffected, but only under a fixed denominator (D5; see "Two safe
relaxations" below), and — since every entry into B is a trial (D16) — apart from extra absorption
that B can cause through a cycle back to himself. A1 is stated accordingly.

Separating "distrust" (a node-level property) from "walk mechanics" (an in-walk state flag) is
the core refactor. The present conflation of the two is what makes P2 possible.

---

## D3 — Backward discredit (λ) and blame radius (γ)

**Context**: the absorbing wall alone withholds credit but does not actively punish. Whether it
should is a separate, tunable decision.

**Decision**: a poisoned walk assigns `−λ·b_i` to the nodes on its prefix instead of `+1`, where
`b_i = γ^(distance from i to B)`. Blame flows *backward along the walk that reached Bob* — to
whoever routed Alice there — not forward from Bob. The ego itself is exempt.

Closed form for node C:

```
score_A(C) = n_C · (1 − (1+λ)·q_C)
```

`n_C` = probability that a walk visits C (a node counts once per walk), `q_C` = probability that a walk at C is later
**absorbed** at a wall. Not merely that it reaches a walled node: for a soft wall the two
differ, and only absorption poisons (the notebook computes `q = d·α^distance`). The walled node
itself sits at distance 0, so `b_B = 1`. Two mechanisms live in that formula and must not be
conflated:

1. **Withholding** (`λ = 0`, the wall alone): the poisoned walk simply credits nobody. C loses
   the `q_C` share of its hits — "you spent part of your influence on a scammer". Passive,
   with no attack surface at all.
2. **Discredit** (`λ > 0`): the same walks actively subtract on top. Hence `(1+λ)`, not `(1−λ)`:
   one unit of withheld credit plus λ of added penalty.

`q_C` is C's own routing choice and is invariant to everything Bob does. A node goes negative at
`q_C > 1/(1+λ)`; at `λ = 1` that is `q_C > 0.5`. Recommended starting point: `λ ≈ 0.3–0.5`.

`γ = 1` blames the whole prefix uniformly; `γ → 0` concentrates blame on the wall node itself
(`b = γ⁰ = 1`). Blaming only the direct voucher is a separate mode (blame radius 1), not a limit
of γ. The closed form above holds for a node with `b = 1`; in general see the feature document,
R10.

---

## D4 — Terminology: two different "one step"

**Context**: "1-step distrust" was used for two unrelated things during design, which caused a
genuine mistake in an earlier draft of the notebook.

**Decision**: keep the axes named apart. They are orthogonal and combine freely.

| | **Distrust inheritance** (Guha et al. 2004) | **Blame radius** |
|---|---|---|
| Question | *whose* minuses enter Alice's frame | *whom* to punish for routing to Bob |
| Direction | forward along trust: Alice → C, C ⊣ Bob | backward along the walk: … → aK → Bob |
| "one step" means | inherit minuses from those you trust, and stop there | penalise only the direct voucher, not the whole prefix |

---

## D5 — Fixed denominator

*Refined by D17: the denominator is exactly W.*

**Context**: `rank.rs::get_node_score` normalises by realised hits (`pos_total + neg_total`).

**Decision**: normalise by walk count (`walks_per_ego`), or equivalently a fixed
`W·α/(1−α)`. Under realised-hit normalisation, walks dying at the wall shrink the denominator, so
distrusting Bob *raises* the score of uninvolved third parties — trust burned on Bob is silently
redistributed to everyone else, and his backers lose nothing. This violates A6 and breaks the
model regardless of λ.

Measured on the notebook's two-chain topology: an uninvolved control chain gains **+6.7%** purely
from Alice blocking Bob.

---

## D6 — Distrust inheritance: one-step only in phase 1

**Decision**: phase 1 ships `D_A` = Alice's own negative targets. Phase 2 may add inheritance from
trusted nodes, weighted by flow, with a cap (requirements: D7; time decay is not MR's — the
application re-publishes `d`) — Guha et al. 2004 report one-step distrust
outperforming propagated distrust, and each additional hop hands a trusted node the power to
zero out third parties.

Inheritance must be a **separate pass** (compute `D_A`, then run walks), not an in-walk state.
Otherwise a node that correctly flags Bob ends up on the poisoned prefix and is penalised for the
flag — the walk arrives at C, and C is the one who reported Bob.

Note the contrast with today's behaviour: because a minus is currently an ordinary edge, issuing
one *costs the accuser*. It diverts `|w⁻|/Σ|w|` of their walk mass, so their own downstream
friends lose standing merely because they reported someone. Under D2 accusing is free.

---

## D7 — Inheritance deferred; requirements fixed for when it is built

**Context** (2026-09-26): Tentura's episode-closure design note
(`tentura/docs/plans/post-request-closure-social-design.md`) wants inherited distrust, but it is
too complex for the first MR iteration. Tracked in
[Intersubjective/meritrank-rust#86](https://github.com/Intersubjective/meritrank-rust/issues/86)
(Tentura side: beads `tentura-dla`).

**Decision**: phase 1 ships own walls only (`D_A` = A's own negative targets), as D6 already
allows. When inheritance is built, it must satisfy:

- Computed inside MR (it needs A's frame scores); no new API — its sources are the existing
  negative edges `C→B`.
- **Separate pass**, per D6: the node that flagged B must not pay for the report.
- **Weighted by inbound trust** (the inheriting frame trusts the issuer). Never by the issuer's
  outgoing trust, and never by a global aggregate such as the number of flaggers or in-degree.
- **One step**: inherited walls are not inherited further.
- **Which walls — open.** Preferably hard walls only (`|w| = 1`, explicit bans). Soft walls are
  behavioural traces, and inheriting them opens a read-receipt oracle: X can trust A
  unilaterally and read A's walls off diffs of X's own frame. The alternative — inherit all
  walls proportionally to `d`, under a cap — weakens the oracle but does not close it. Against
  hard-only: inheriting soft walls is the only way a spammer's standing drops in the frames of
  third parties who walk through his recipients; phase 1 confines the effect to the recipients'
  own frames (Tentura design note, §13).
- **Own positive edge vs inherited wall is not MR's concern.** The application compensates
  (Tentura keeps friends always visible); MR stays generic.

With inheritance, A8 stops holding trivially and becomes a real boundary. Test it three ways:
`score_C(A) = 0` ⇒ no change in C's frame; `score_C(A) > 0` ⇒ change `≤ cap · score_C(A)`;
A trusts C but C does not trust A ⇒ no change in C's frame. One-step needs its own test: under
multi-hop inheritance a wall reaching D through C still satisfies A8, because D transitively
trusts A.

---

## D8 — A wall is encoded as a negative edge weight

**Context**: MR's storage and API carry one edge type with one signed value per `(src, dst,
context)`; 0 deletes the edge. A "complex" weight (trust + wall) would mean a through-refactor
of every interface from the application to the Rust core.

**Decision**: a negative weight on `A→B` means `B ∈ D_A` with `d_A(B) = min(|w|, 1)`. This is
exactly D2 + D6 phase 1, so no new function and no signature change is needed. The application
folds its own two-dimensional state (trust + wall) into one signed scalar per pair; MR never
sees both on the same pair and never reconciles them. Named contexts cannot carry the second
dimension — the null context is the sum of the contexted ones (`null_context_is_sum`), so a
wall would be summed into trust. Walls exist in the null context only.

**Consequence**: a pair cannot be "trusted and walled" at once inside MR. Tentura resolves the
conflict in favour of trust: the explicit relation beats the automatic signal, and replacing
trust with `−d` would also withdraw A's endorsement from every other frame.

## D9 — Only walls that justify discredit reach MR; one global λ

**Context**: in Tentura's episode-closure design a wall has two possible causes: the walled
node's own behaviour (B keeps contacting A unanswered) or the ego's own action (A just contacted
B). With `λ > 0`, blame for the second lands on whoever routed A to B, although nobody vouched
for anything bad.

**Decision**: effects of the ego's own actions stay in the application (it re-ranks A's own
list; no walk is poisoned). MR receives only walls caused by the walled node's behaviour, plus
explicit bans. Discredit is justified for both, so one global `λ` (and one `γ`) suffices;
severity lives in `d`. MR is kind-agnostic. The only kind distinction it can see is magnitude —
hard (`|w| = 1`, bans) versus soft (`|w| < 1`) — which D7 may use.

## D10 — Walls are private: no ego-facing read returns a negative edge

*Superseded by D23: read privacy is not MR's concern.*

**Context**: once a negative edge is a wall, reading it tells a third party who walls whom — in
Tentura, who ignores whom, which is a read-receipt the walled party cannot switch off. Today
`mr_graph(..., positive_only)` returns negative edges when asked, and Tentura's Hasura function
`graph()` forwards a client-chosen `positive_only`.

**Decision**: the ego-facing read functions (`mr_graph`, `mr_neighbors`) never return negative
edges, whatever `positive_only` says. Administrative and sync functions (`mr_edgelist`,
`mr_fetch_new_edges`, bulk load) may. The application additionally forces `positive_only = true`
(defence in depth).

## D11 — Backer drag is intended (reclassifies the "poisoned honeypot")

**Context**: "Known attacks" lists the poisoned honeypot — Bob farms endorsements, then attracts
a minus and drags his endorsers down.

**Decision**: for Tentura this is joint liability by design, not an attack to defend against. If
I vouched for someone who starts spamming, I lose standing with him. The loss is bounded by my
own flow into him (A4) and repaired the moment I weaken or cut the edge (A5). The application is
expected to tell vouchers when this is happening (Tentura design note, Q9). Phase 1 alert is
qualitative and computed by the application from its own walls, the voucher's edges and
`mr_node_score(s, X) > 0`; MR needs nothing extra. A loss-attribution function
(`mr_wall_loss(ego, target, wall)`, from `absorbed_at`) is a possible later extension.

## D12 — Rollout: clear legacy negative edges before switching semantics

**Context**: after the switch every negative edge is a wall with discredit. Tentura publishes
negative weights today from the signed half of its review scale, so the curse attack (P1/P2) is
live in production under the current semantics.

**Decision**: before the MR switch, the application removes every legacy negative edge (Tentura:
clamp published trust weights to `≥ 0` and resync). Only then does it publish walls.

## D13 — One absorption trial per walk per wall

*Superseded by D16: a trial on every entry.*

**Context** (cross-check, 2026-09-26): if every arrival at a soft wall is a new trial, B can add
a cycle `B→D→B` and raise the absorption probability of everyone who routes to him — B's own
out-edges then change the scores of nodes upstream of him, which A1 forbids.

**Decision**: a walk takes exactly one absorption trial at a given wall, at its first arrival.
If it passes, later arrivals at the same wall are free.

**Consequence**: B's out-edges no longer affect nodes upstream of him through re-trials. One
effect remains and is intended: if a *second* wall E is reachable from B, walks that pass B and
are absorbed at E blame their whole prefix, including those who routed to B. That is ordinary
backward blame along the path — vouching for B covers where B routes — and it is bounded by the
`(1 − d)` share that passes B. A1 is stated with this proviso.

## D14 — Zero opinion is left alone; axioms are about the core score

**Context**: served scores blend in a global zero opinion, `(1 − k)·score + k·zero` (Tentura
runs `k = 0.02`). A node behind a hard wall, and everything reachable only through it,
therefore gets `k·zero(X)` instead of 0 in the served output.

**Decision**: keep the blend for all nodes. Zero opinion is a deliberately ego-independent prior;
walls act on walks inside one frame, and mixing the two layers to zero out one case — which the
application hides anyway (bans) — is not worth it. Suppressing the blend only for `D_A` would
not even be complete: the subtree behind the wall would still get `k·zero`. Axioms and tests are
stated on the core score; MR integration tests run with the factor at 0 or read the core score.
A6 holds on served scores as well, since the zero-opinion term does not depend on walls.

## D15 — Cross-check fixes: accounting, exact storage, contexts, barrier, tests

**Context** (2026-09-26): an independent cross-check (codex, GPT-6 Astra) of the feature spec
against the service code and the Tentura design found gaps between the spec and the service as
it actually runs.

**Decisions**:

- **Once per walk.** The core counts a node once per walk (`increment_unique_counts`); the spec
  had said "per visit". Blame is also once per walk, with the `b` of the node's visit nearest to
  the wall. `n_C` means "probability of being visited".
- **Walls are exempt from VSIDS.** The service stores `w·bump^(index − mag_scale)` and rescales
  and prunes a node's out-edges together. Positive weights are normalised, so this cancels for
  them; `d` is not, so scaling would weaken walls and pruning would delete soft ones. Walls keep
  the exact published `d`.
- **Contexts are binding.** Walls follow the service's fan-out of null-context user edges into
  every context, including later ones; bulk and incremental writes must agree; bulk validates.
- **Barrier and caches.** A write ack means "queued"; `mr_sync` is the barrier. Wall changes
  invalidate cached scores. A sign transition (trust ↔ wall) is a positive-edge change.
- **Deterministic tests.** A seed setting; otherwise statistical tolerances; walls are inspected
  in tests only through administrative reads.
- **Axioms re-scoped.** A6 compares against the wall-free graph (the per-step form is false: a
  wall upstream of wall B lifts B's negative score toward 0). A9 excludes sign transitions. A4's
  "investment" is `n_C·q_C`.

---

## D16 — An absorption trial on every entry into a wall (replaces D13)

**Context** (implementation design review, 2026-09-26): D13's "one trial per walk per wall" makes
the walk non-Markov: whether an arrival at B is a trial depends on the walk's history, so every
repair path (generation, the optimizer's forced steps, R16 wall updates, `clear()`) must restore
that hidden state correctly. This is the same class of state the current design carries in
`negative_segment_start`, and exactly that state produced a live bug (D20).

**Decision**: every arrival at `B ∈ D_A` is an independent absorption trial with probability
`d_A(B)`. The walk state is just the current node. Stepping needs only "is this node a wall of
the walk's ego"; no prefix scan. R16 wall updates get the same shape as the optimizer: scan the
walk's arrivals at B from the first one (its position is already in the `visits` index) and
re-couple each passing arrival.

**Cost, accepted**: A1 weakens for soft walls. B can add a cycle back to itself, so a walk that
passed B returns and is tried again. With return probability `r`, the absorption probability of
a walk entering B becomes `d_eff = d / (1 − (1 − d)·r)` (at `d = 0.5`, `r = α² ≈ 0.72`:
`d_eff ≈ 0.78`). B's out-edges can therefore *strengthen* the wall against B, so nodes routing to
B lose more. This only goes down, only affects nodes on walks that reach B, never exceeds the
hard-wall loss (`d_eff ≤ 1`), keeps A4 (`q_C` stays bounded by the probability of reaching B)
and gives B nothing. It is a bounded variant of the suicide-bomber drag already accepted in D11.
Hard walls are unaffected.

## D17 — The denominator is exactly W

**Decision**: `score_A(X) = (credits_X − λ·blame_X) / W`, where W is `walks_per_ego`
(`MERITRANK_NUM_WALKS`). Without walls a score is the probability that a walk of A visits X. The
alternative "`W·α/(1−α)`" in D5 is dropped: it differs from W only by a constant factor and has
no meaning of its own.

**Scale check**: every score grows by roughly `L̄` (≈6 at α = 0.85) against today's
realised-hits normalisation, and the ego scores ≈ 1. Tentura is insensitive to this: it uses MR
scores only through per-ego quantile clusters (`service/src/aug_graph/scores.rs:25-76`), ordering
within one viewer, and sign checks `forward_mr > 0` / `reverse_mr > 0`
(`tentura/.../m0193.dart:3807-3813`). No absolute score threshold exists in its SQL. MR tests
with hard-coded score values must be re-derived.

Realised-hits normalisation also skews comparisons between egos: an ego whose only friend is a
dead end gets that friend's score inflated ~3× relative to an ego whose only friend has a large
network behind him, only because of the walk length. Under W both get `α`.

## D18 — Walls are User→User only

**Context**: current Tentura publishes only `U` nodes to MR; beacon, comment and opinion
triggers are defined but not attached, and there is no ownership (Tentura `m0193.dart`,
`trust_rebuild_effective_edge` at `:4311`; poll edges exist only in `meritrank_init` and carry no
nodes in practice). Every wall in Tentura's §17.4 plan is user-to-user.

**Decision**: a negative weight is accepted only on a User→User edge in the null context. MR
rejects any other negative write with an error (`mr_put_edge`, bulk load, service), rather than
clamping it silently. Removing ownership logic from MR is a separate track; walls do not depend
on it.

## D19 — Walls inside the optimized incremental invalidation

**Context**: the optimizer (`OPTIMIZE_INVALIDATION`) is mandatory: without it a hub edge change
invalidates every walk through the hub, which does not scale. Statistical tests confirmed it is
unbiased for the current semantics (`core/tests/test_incremental_bias.rs`: chain and multi-ego
star vs fresh generation and analytic values; two independent adversarial suites found no bias in
the coupling itself, D20).

**Decisions**:

- **Positive edge change at X** (all egos' walks): the current algorithm, collapsed to one regime
  with `p = w / (pos_sum + w)`; the `neg_start` logic disappears. The terminal position of a walk
  absorbed at X is never re-coupled: that walk never departed X. Every place that extends a walk
  (fresh generation, the `random < α` push after invalidation, the forced step on deletion)
  calls one function that performs the absorption trial.
- **Wall change `A⊣B`, `d₀ → d₁`** (only A's walks, R16): per-arrival re-coupling as in D16.
  Candidates come from `visits[B]` filtered by A's walk-id block, or from scanning A's W walks,
  whichever is smaller (adaptive; protects against hubs). The whole contribution of a touched walk
  (credits and blame) is removed and re-added. `|w| > 1` changes are no-ops for walks.
- **Sign transition** `+w ↔ −d` on one pair: two exact operations in sequence, a positive-edge
  deletion (all frames) then a wall addition (A's frame), or the reverse. The intermediate
  "no edge" graph is valid; composition of exact steps is exact. This is how `set_edge` already
  replaces a weight.
- **Cached sums are recomputed exactly** (`pos_sum` over the node's positive edges, O(degree)) on
  every change: the `WeightedIndex` cache is rebuilt at the same cost on the next sample anyway.
  Incremental `+=`/`−=` cancels catastrophically for weight ratios > 2^53 (D20).
- **Blame is stored as `Σ b` without λ**; λ is applied at read time. Debug builds check it
  against a from-scratch recount after every repair.
- **Clearing a walk resets all of its metadata** (`absorbed_at`), the lesson of D20.

## D20 — Findings of the adversarial optimizer tests

Two independent adversarial suites (codex GPT-6 Astra: `core/tests/test_incremental_adversarial.rs`;
a Fable subagent: `core/tests/test_incremental_adversarial_fable.rs`) tried to break the optimizer
for the current signed semantics. Both found the same three defects and no bias in the coupling:

1. **Stale `negative_segment_start` after `RandomWalk::clear`** (live in production: Tentura runs
   walk eviction, `MERITRANK_WALKS_CACHE_SIZE=200`). Recalculating an ego, or `clear_ego` +
   `calculate`, reused walk slots that kept the marker, so regenerated walks ran positive-only: a
   negative edge out of the ego was taken with probability `(1−α)·α ≈ 0.13` instead of `α`.
   **Fixed in `main`** (`8285bde`, meritrank_core 0.10.1), with regression tests.
2. **Catastrophic cancellation in `pos_sum`/`neg_sum`** (`graph.rs:232, 294`): after adding and
   removing a weight 2^54 times larger than another, the cached sum is 0 while an edge remains,
   and the invalidation probability becomes 1. Numeric edge case; fixed by D19.
3. **Panic on `set_edge` with `|w| ≤ ε` on an absent edge** (`rank.rs:205` treats it as a
   deletion, `graph.rs:310` panics). Fixed in the feature: such a write is a no-op; a wall is
   deleted only by an exact 0 (R19).

## D21 — The ego on absorbed walks

**Decision**: an absorbed walk credits nobody, the ego included, and the ego never takes blame.
Hence `score_A(A) = 1 − P(a walk of A is absorbed)`, and the closed form `n_C·(1 − q_C)` holds for
the ego too (`n = 1`, `q` = absorption probability) without a special case. Tentura never reads
the ego's own score.

## D22 — Service consistency is a separate track that lands before walls

**Context** (2026-09-26, verified against the code, second opinion by codex GPT-6 Astra): the
service's double buffer, caches and sync barrier do not meet R16/R21/R22 today, independently of
walls:

- **Two non-replica copies.** Each subgraph keeps two `AugGraph` copies fed by
  `FanoutSender::send`, which awaits the two queues separately (`state_manager.rs:25-45`). Two
  concurrent senders can interleave (`a1, a2, b2, b1`), so the copies can apply non-commuting
  writes in different orders and diverge in graph state. Their walks are different Monte Carlo
  samples, so reads jitter across swaps (±0.016 at W = 1000, Tentura's production value).
- **`mr_sync` is not a barrier.** The connector's stamp counter starts at 0 in every PostgreSQL
  backend process (`psql-connector/src/rpc.rs:38`); the service waits for published stamp `≥`
  requested (`state_manager.rs:295`), and `Stamp` assigns rather than maximises
  (`aug_graph/absorb.rs:85`). A new backend's `sync(1)` returns at once if any stamp ≥ 1 was ever
  published. `mr_sync` is also declared `IMMUTABLE` (`psql-connector/src/lib.rs:286`).
- **Reader stall.** `process_read` loads the published Arc, then takes its read lock
  (`state_manager.rs:253-261`); if a swap happens in between, the worker holds that copy's write
  lock while blocking for the next op, so the read waits until the next write arrives.
- **Liveness.** With `min_ops_before_swap > 1` a barrier cannot publish without further traffic;
  `queue_len = 1, min_ops = 2` deadlocks.
- **Caches.** `cached_scores` (TTL 1 h) and `cached_score_clusters` (TTL 6 h) are never
  invalidated and are shared by both copies (moka clones share storage). Reverse scores (the
  viewer's score in a peer's frame, Tentura's `reverse_mr`) are read cache-first
  (`scores.rs:219-241`): for `mr_mutual_scores` a fresh value exists but a stale cached one wins;
  for an evicted peer the reverse score lives only in the cache.
- `mr_mutual_scores` calculates every user, but only the ego is tracked for eviction, so the walk
  cache is not bounded.

**Decisions** (the track):

1. **One ordered operation sequence** with sequence numbers and a replay log that both copies
   apply; the copies are exact replicas (same seed, same order). Publication only from a copy at
   least as advanced as the published position. Ordering also holds across contexts for fanned-out
   User→User writes. Readers acquire a copy safely (no stall behind the worker).
2. **Server-owned barrier**: a marker in the sequence with a completion handle, forced
   publication at a barrier regardless of batching, errors propagated. `mr_sync` becomes
   `VOLATILE`. This is R21.
3. **No score cache.** A score read is two counter lookups. Cluster bounds stay cached, keyed by
   `(ego, kind, generation(ego), zero-opinion revision)`, one cache per copy. The core reports the
   egos whose walks changed during an operation (union over all nested `set_edge` calls, including
   VSIDS rescales), plus `calculate`/`clear_ego`; the service bumps their generations. A wall
   change always bumps its owner, even an evicted one. Bulk load and reset start a new epoch.
4. **Reverse scores are computed, not remembered**: before answering, the service calculates the
   peers it actually needs (positive-forward candidates, deduplicated, inside the worker, protected
   until the response is built). A "last known" value never grants visibility.
5. **RNG owned by `MeritRank`**, every random draw through it, one seed for both copies
   (`MERITRANK_SEED`, or drawn once at start-up); deterministic bulk ordering and score-tie
   ordering (R22).

The wall feature then needs only "a wall change marks its owner dirty".

## D23 — Read privacy is not MR's concern (replaces D10)

**Decision**: MR does not filter negative edges out of `mr_graph`, `mr_neighbors`, `mr_connected`
or `mr_fetch_new_edges`; users have no direct access to MR, and hiding walls from clients is the
application's job (Tentura forces `positive_only = true` in its own `graph()`, §17.4 checklist).
R14 is withdrawn.

One read change is kept for consistency, not privacy: `mr_graph` normalises positive weights by the
positive out-weight (`pos_sum`), matching the transition probabilities of R4, instead of by
`abs_sum` (`service/src/aug_graph/graph_read.rs:184`). It ships with the feature, since under the
current semantics `abs_sum` is correct.

## D24 — Configuration

| Setting | Meaning | Valid | Default |
|---|---|---|---|
| `MERITRANK_DISCREDIT_LAMBDA` | λ | finite, ≥ 0 | **0** |
| `MERITRANK_BLAME_DECAY` | γ | [0, 1] | 0.8 |
| `MERITRANK_BLAME_RADIUS` | `prefix` (γ-decayed whole prefix) or `voucher` (the wall and the direct voucher) | enum | `prefix` |
| `MERITRANK_SEED` | RNG seed (R22) | u64 | unset: random at start-up, shared by both copies |

λ defaults to 0: walls ship as withholding only, which has no attack surface; the application
enables discredit after calibrating λ on the dump (journal recommendation 0.3–0.5 stands as a
starting point). γ and the radius default to the notebook's values so that enabling λ reproduces
the verified behaviour. All four are start-up only. An invalid value refuses start-up. Start-up
also rejects `α ≥ 1` (`service/src/settings.rs:96` accepts 1 today; on a cyclic graph a walk then
never ends).

## D25 — Specification corrections from the implementation review

- **R2's rationale was wrong**: MR's null context is last-write-wins, not a sum
  (`state_manager.rs` test `context_aggregate_null_context_last_write_wins`; even the connector
  test named `null_context_is_sum` asserts last-write-wins). User→User writes fan out to every
  context whatever their `context` field says (`state_manager.rs:833`). The rule stands for a
  different reason: walls belong to no context by construction.
- **R15 is distributional**: different histories that reach the same graph give the same
  distribution, not the same finite sample. R22 promises identical results only for the same seed
  and the same ordered operation sequence, lazy calculations and evictions included.
- **R19**: a wall write changes no positive weight other than the replaced positive edge of the
  same pair in a sign transition.
- **R7**: seeding a new context (`seed_context_from_aggregate`) copies walls with their exact `d`,
  bypassing VSIDS.
- **R18**: Tentura sets no zero opinion (no `mr_set_zero_opinion` call; `zerorec` exists only in
  `service/src/legacy/`), so blending reduces to `0.98·score` there. The rule is unchanged.
- **`mr_fetch_new_edges`** is not implemented in the service (`state_manager.rs:543` returns
  `NotImplemented`).
- **Bans in Tentura** currently publish 0, not −1 (`m0193.dart:4304`); −1 is part of their §17.4
  checklist.

---

## Axioms

Candidates for the test suite. A1 and A6 are already checked live in the notebook. Test
formulations: `NEGATIVE_EDGES_FEATURE.md`, §7.

| | Axiom |
|---|---|
| A1 | Changing any outgoing edge of B leaves `score_A(X)` unchanged for all `X ≠ B` (hard wall). Soft wall: nodes not reachable from B change only through extra absorption (re-entry into B, D16, or a wall reachable from B), hence only downward and never beyond the hard-wall loss |
| A2 | A node reachable from A only through B scores 0 (hard wall; soft wall: at most `(1 − d)` of its baseline) |
| A3 | A node X with support independent of B and `q_X = 0` keeps a positive score (no collateral damage) |
| A4 | C's multiplier is monotone in `q_C`, and C's loss never exceeds `(1+λ)` × its investment in B (`n_C·q_C`) |
| A5 | Removing `C→B` restores C immediately (penalty is a function of the current graph) |
| A6 | Walls must not raise anyone's score above its value in the wall-free graph (D15) |
| A7 | Total loss in A's frame ≤ `(1+λ)` × flow into B, where flow into B is `M_B`: the baseline visit mass of the walks that reach B |
| A8 | A's negative edge affects only frames that transitively trust A |
| A9 | Phase 1 (no inheritance): adding, changing or removing a wall of A, with A's positive out-edges unchanged, changes no score in any frame other than A's |

---

## Known attacks and tuning risks

- **Poisoned honeypot / suicide bomber**: Bob farms honest endorsements, then attracts a minus,
  dragging his endorsers down. Cost is his own account; damage is bounded by each endorser's
  own flow into him. Self-limiting but real. Tentura treats the drag as intended (D11).
- **Sybil-farm amplification**: with whole-prefix blame, a farm that collects honest endorsements
  and links to a poison node spreads blame to those endorsers. Mitigated by γ decay or by the
  1-step blame radius.
- **Hub problem**: `q` accumulates at a hub friend across many distrusted nodes, so an
  uninvolved hub can drift negative. Needs measurement on a real dump — the linear chain cannot
  show it. More acute when soft walls are widespread (Tentura's contact-noise walls), because
  `q` then collects many small `d` values.
- **Two safe relaxations that break A1 jointly**: a soft wall (`d < 1`) alone does not violate A1
  under a fixed denominator (shift measured at 0.00%). The normalisation trap alone does not
  either, at `d = 1`. Combined, they do (≈1.5%): Bob's protégés become reachable, enter the
  denominator, and Bob again moves third-party scores. Verified numerically in the notebook.
- **Visibility of negative edges**: currently global graph data usable by every ego. Under A8 a
  minus should only affect frames that transitively trust its issuer. Read access is the
  application's concern (D23). Probing through diffs of one's own frame remains possible once inheritance exists (D7);
  the application mitigates it by quantizing when walls are published.

---

## Implementation map

The authoritative map is `NEGATIVE_EDGES_FEATURE.md`, §8. In short:

| Concept | Code location |
|---|---|
| One stepping function with the absorption trial (D16, D19) | `core/src/graph.rs::generate_walk_segment`, the push in `core/src/rank.rs::set_edge_`, `Graph::extend_walk_in_case_of_edge_deletion` |
| `absorbed_at` instead of a negative suffix | `core/src/random_walk.rs`; `clear()` resets it (D20) |
| Blame `Σ b`, λ at read (D19) | `core/src/rank.rs` accounting; `get_node_score` divides by W (D17) |
| Positive-edge invalidation, one regime, absorbed terminal excluded (D19) | `core/src/walk_storage.rs::decide_skip_invalidation_on_edge_addition`, `rank.rs::set_edge_` |
| Wall change, per-arrival re-coupling, adaptive candidates (D16, D19) | new path in `rank.rs`; `walk_storage.rs::get_visits_through_node(B)` or A's walk block |
| Exact cached sums (D19, D20) | `core/src/graph.rs::set_edge`, `remove_edge` |
| User→User only, null context (D18) | `psql-connector/src/lib.rs::mr_put_edge`, bulk, `service/src/state_manager.rs` |
| `mr_graph` normalised by `pos_sum` (D23) | `service/src/aug_graph/graph_read.rs:184` |
| Consistency track (D22) | `service/src/state_manager.rs`, `aug_graph/scores.rs`, `aug_graph/mod.rs`, `psql-connector/src/{rpc,lib}.rs` |
| Settings (D24) | `service/src/settings.rs` |

---

## Not yet validated

The chain demo cannot show these; they need the real dump (`tentura_dump.sql.gz`):

- distrust inheritance from trusted nodes at realistic fan-out,
- the hub problem at realistic minus counts, and under widespread soft walls,
- the poisoned-honeypot attack,
- what `λ` and `γ` should actually be.
