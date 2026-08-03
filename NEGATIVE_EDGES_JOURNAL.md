# Design Journal: Negative Edges — Absorbing Wall

**Status: PROPOSED, NOT IMPLEMENTED.** The code in `core/` still implements the forward-penalty
semantics described under "Problem" below. This document records the design reasoning so it can
be reviewed before any code is touched.

Decision numbering is local to this document (`JOURNAL.md` covers the NNG→TCP migration and has
its own D1, D2, …).

Companion interactive demo: `scripts/negative_edges_demo.ipynb` (run via `scripts/run_demo.sh`).

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

`n_C` = expected visits to C per walk, `q_C` = probability that a walk at C later hits a
distrusted node. Two mechanisms live in that formula and must not be conflated:

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
trusted nodes, weighted by flow, with a cap and decay — Guha et al. 2004 report one-step distrust
outperforming propagated distrust, and each additional hop hands a trusted node the power to
zero out third parties.

Inheritance must be a **separate pass** (compute `D_A`, then run walks), not an in-walk state.
Otherwise a node that correctly flags Bob ends up on the poisoned prefix and is penalised for the
flag — the walk arrives at C, and C is the one who reported Bob.

Note the contrast with today's behaviour: because a minus is currently an ordinary edge, issuing
one *costs the accuser*. It diverts `|w⁻|/Σ|w|` of their walk mass, so their own downstream
friends lose standing merely because they reported someone. Under D2 accusing is free.

---

## Axioms

Candidates for the test suite. A1 and A6 are already checked live in the notebook.

| | Axiom |
|---|---|
| A1 | Changing any outgoing edge of B leaves `score_A(X)` unchanged for all `X ≠ B` |
| A2 | A node reachable from A only through B scores 0 |
| A3 | A node with support independent of B keeps a positive score (no collateral damage) |
| A4 | C's multiplier is monotone in `q_C`, and C's loss never exceeds its investment in B |
| A5 | Removing `C→B` restores C immediately (penalty is a function of the current graph) |
| A6 | Distrusting B must not increase anyone's score |
| A7 | Total loss in A's frame ≤ `(1+λ)` × flow into B |
| A8 | A's negative edge affects only frames that transitively trust A |

---

## Known attacks and tuning risks

- **Poisoned honeypot / suicide bomber**: Bob farms honest endorsements, then attracts a minus,
  dragging his endorsers down. Cost is his own account; damage is bounded by each endorser's
  own flow into him. Self-limiting but real.
- **Sybil-farm amplification**: with whole-prefix blame, a farm that collects honest endorsements
  and links to a poison node spreads blame to those endorsers. Mitigated by γ decay or by the
  1-step blame radius.
- **Hub problem**: `q` accumulates at a hub friend across many distrusted nodes, so an
  uninvolved hub can drift negative. Needs measurement on a real dump — the linear chain cannot
  show it.
- **Two safe relaxations that break A1 jointly**: a soft wall (`d < 1`) alone does not violate A1
  under a fixed denominator (shift measured at 0.00%). The normalisation trap alone does not
  either, at `d = 1`. Combined, they do (≈1.5%): Bob's protégés become reachable, enter the
  denominator, and Bob again moves third-party scores. Verified numerically in the notebook.
- **Visibility of negative edges**: currently global graph data usable by every ego. Under A8 a
  minus should only affect frames that transitively trust its issuer. Privacy and probing
  implications are unresolved.

---

## Implementation map

| Concept | Code location |
|---|---|
| Absorption on B, `q_i` | `core/src/graph.rs::generate_walk_segment` — `random_neighbor` always `positive_only`, absorb on the ego-relative `D_A` |
| `poisoned` instead of a negative suffix | `core/src/random_walk.rs` — `negative_segment_start: Option<usize>` → `absorbed_at: Option<NodeId>`; `positive_subsegment`/`negative_subsegment` collapse |
| Blame weights `b_i` | new accounting step in `core/src/rank.rs::calculate` |
| Fixed denominator | `core/src/rank.rs::get_node_score` (lines 108–112) — `total_hits` → `walks_per_ego` |
| Invalidation on adding a minus | `core/src/walk_storage.rs::get_visits_through_node(B)` already yields the affected walks: cut at B, move the prefix from `pos_hits` to `neg_hits` |

---

## Not yet validated

The chain demo cannot show these; they need the real dump (`tentura_dump.sql.gz`):

- distrust inheritance from trusted nodes at realistic fan-out,
- the hub problem at realistic minus counts,
- the poisoned-honeypot attack,
- what `λ` and `γ` should actually be.
