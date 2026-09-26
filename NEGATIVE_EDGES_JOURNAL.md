# Design Journal: Negative Edges — Absorbing Wall

**Status: PROPOSED, NOT IMPLEMENTED.** The code in `core/` still implements the forward-penalty
semantics described under "Problem" below. This document records the design reasoning so it can
be reviewed before any code is touched.

Decision numbering is local to this document (`JOURNAL.md` covers the NNG→TCP migration and has
its own D1, D2, …).

Companion interactive demo: `scripts/negative_edges_demo.ipynb` (run via `scripts/run_demo.sh`).
Requirements, glossary and acceptance tests: `NEGATIVE_EDGES_FEATURE.md`. This journal records
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
relaxations" below). A1 is stated accordingly.

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

`n_C` = expected visits to C per walk, `q_C` = probability that a walk at C is later
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

`γ = 1` blames the whole prefix uniformly; `γ → 0` concentrates blame on the direct voucher.

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
  walls proportionally to `d`, under a cap — weakens the oracle but does not close it.
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
expected to tell vouchers when this is happening (Tentura design note, Q9); MR needs nothing
extra.

## D12 — Rollout: clear legacy negative edges before switching semantics

**Context**: after the switch every negative edge is a wall with discredit. Tentura publishes
negative weights today from the signed half of its review scale, so the curse attack (P1/P2) is
live in production under the current semantics.

**Decision**: before the MR switch, the application removes every legacy negative edge (Tentura:
clamp published trust weights to `≥ 0` and resync). Only then does it publish walls.

---

## Axioms

Candidates for the test suite. A1 and A6 are already checked live in the notebook. Test
formulations: `NEGATIVE_EDGES_FEATURE.md`, §7.

| | Axiom |
|---|---|
| A1 | Changing any outgoing edge of B leaves `score_A(X)` unchanged for all `X ≠ B` (hard wall); for a soft wall, for all X not reachable from B |
| A2 | A node reachable from A only through B scores 0 (hard wall; soft wall: at most `(1 − d)` of its baseline) |
| A3 | A node X with support independent of B and `q_X = 0` keeps a positive score (no collateral damage) |
| A4 | C's multiplier is monotone in `q_C`, and C's loss never exceeds its investment in B |
| A5 | Removing `C→B` restores C immediately (penalty is a function of the current graph) |
| A6 | Distrusting B must not increase anyone's score |
| A7 | Total loss in A's frame ≤ `(1+λ)` × flow into B, where flow into B is `M_B`: the baseline visit mass of the walks that reach B |
| A8 | A's negative edge affects only frames that transitively trust A |
| A9 | Phase 1 (no inheritance): adding, changing or removing a wall of A changes no score in any frame other than A's |

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
  minus should only affect frames that transitively trust its issuer. Read access is settled by
  D10. Probing through diffs of one's own frame remains possible once inheritance exists (D7);
  the application mitigates it by quantizing when walls are published.

---

## Implementation map

| Concept | Code location |
|---|---|
| Absorption on B, `q_i` | `core/src/graph.rs::generate_walk_segment` — `random_neighbor` always `positive_only`, absorb on the ego-relative `D_A` |
| `poisoned` instead of a negative suffix | `core/src/random_walk.rs` — `negative_segment_start: Option<usize>` → `absorbed_at: Option<NodeId>`; `positive_subsegment`/`negative_subsegment` collapse |
| Blame weights `b_i` | new accounting step in `core/src/rank.rs::calculate` |
| Fixed denominator | `core/src/rank.rs::get_node_score` (lines 108–112) — `total_hits` → `walks_per_ego` |
| Invalidation on adding a minus | `core/src/walk_storage.rs::get_visits_through_node(B)` already yields the affected walks: cut at B, move the prefix from `pos_hits` to `neg_hits` |
| Negative weight = wall, `d = min(|w|, 1)` (D8) | `mr_put_edge` unchanged; the core reads `D_A` from A's negative out-edges in the null context |
| No negative edge in ego-facing reads (D10) | `psql-connector/src/lib.rs::mr_graph`, `mr_neighbors` — drop negative edges regardless of `positive_only` |

---

## Not yet validated

The chain demo cannot show these; they need the real dump (`tentura_dump.sql.gz`):

- distrust inheritance from trusted nodes at realistic fan-out,
- the hub problem at realistic minus counts, and under widespread soft walls,
- the poisoned-honeypot attack,
- what `λ` and `γ` should actually be.
