use integer_hasher::{IntMap, IntSet};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::constants::{ASSERT, EPSILON, OPTIMIZE_INVALIDATION};
use crate::counter::Counter;
use crate::errors::internal_fatal;
use crate::errors::MeritRankError;
use crate::graph::{Graph, NodeId, Weight};
use crate::random_walk::RandomWalk;
use crate::walk_storage::{WalkId, WalkStorage};

/// Who takes blame for an absorbed walk (NEGATIVE_EDGES_FEATURE.md, R10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlameRadius {
  /// Every distinct node of the prefix, `b = γ^k` with k the steps from its visit nearest to the
  /// absorbing arrival.
  Prefix,
  /// Only the wall and the node visited immediately before the absorbing arrival, `b = 1`.
  Voucher,
}

/// Blame of one node in one frame: the float sum of `b` and the number of absorbed walks that
/// contribute to it. The entry goes away when the last contributor does — never because the sum
/// is small (with a tiny γ, legitimate blame can be arbitrarily small).
#[derive(Clone, Copy, Default, Debug)]
struct Blame {
  sum:   f64,
  walks: u32,
}

#[derive(Clone)]
pub struct MeritRank {
  pub graph:        Graph,
  walks:            WalkStorage,
  /// Credits: for every ego, the number of its unabsorbed walks that visited each node.
  pos_hits:         IntMap<NodeId, Counter>,
  /// Blame: for every ego, `Σ b` over its absorbed walks, per node (λ is applied at read time).
  blame:            IntMap<NodeId, IntMap<NodeId, Blame>>,
  /// Egos whose walks exist. An ego can be calculated with empty counters.
  calculated:       IntSet<NodeId>,
  /// Egos whose walks or counters changed since the last `take_dirty_egos`.
  dirty_egos:       IntSet<NodeId>,
  /// Source of every random draw. Callers that need reproducible walks reseed it (`reseed`),
  /// e.g. before each operation.
  rng:              StdRng,
  pub alpha:        Weight,
  /// λ: weight of blame in the score (0 = walls withhold only).
  pub discredit:    Weight,
  /// γ: blame decay along the prefix.
  pub blame_decay:  Weight,
  pub blame_radius: BlameRadius,
}

impl MeritRank {
  pub fn new(
    graph: Graph,
    walks_per_ego: usize,
  ) -> Self {
    Self {
      graph,
      walks: WalkStorage::new(walks_per_ego),
      pos_hits: IntMap::default(),
      blame: IntMap::default(),
      calculated: IntSet::default(),
      dirty_egos: IntSet::default(),
      rng: StdRng::from_rng(&mut rand::rng()),
      alpha: 0.85,
      discredit: 0.0,
      blame_decay: 0.8,
      blame_radius: BlameRadius::Prefix,
    }
  }

  /// Restarts the random stream from `seed`.
  pub fn reseed(
    &mut self,
    seed: u64,
  ) {
    self.rng = StdRng::seed_from_u64(seed);
  }

  pub fn is_calculated(
    &self,
    ego: NodeId,
  ) -> bool {
    self.calculated.contains(&ego)
  }

  /// The calculated egos, sorted.
  pub fn calculated_egos(&self) -> Vec<NodeId> {
    let mut egos: Vec<NodeId> = self.calculated.iter().copied().collect();
    egos.sort_unstable();
    egos
  }

  /// Returns, sorted, the egos whose walks or counters changed since the previous call, and
  /// forgets them. Covers edge changes (every repaired walk's ego), wall changes (their owner),
  /// `calculate`, `clear_ego` and `clear_walks`.
  pub fn take_dirty_egos(&mut self) -> Vec<NodeId> {
    let mut egos: Vec<NodeId> = self.dirty_egos.drain().collect();
    egos.sort_unstable();
    egos
  }

  /// Number of walk slots allocated, occupied or free.
  pub fn allocated_walks(&self) -> usize {
    self.walks.allocated_walks()
  }

  /// Total capacity of the visits index (memory accounting, tests).
  pub fn visits_capacity(&self) -> usize {
    self.walks.visits_capacity()
  }

  pub fn walks_per_ego(&self) -> usize {
    self.walks.walks_per_ego()
  }

  // ------------------------------------------------------------------
  // Accounting
  // ------------------------------------------------------------------

  /// Blame weights of an absorbed walk's prefix (empty for an unabsorbed walk). The ego is never
  /// blamed (R13); each node appears once, with the `b` of its visit nearest to the wall.
  fn walk_blame(
    &self,
    walk: &RandomWalk,
  ) -> Vec<(NodeId, f64)> {
    if !walk.absorbed || walk.len() < 2 {
      return vec![];
    }
    let nodes = walk.get_nodes();
    let ego = nodes[0];
    let last = nodes.len() - 1;
    match self.blame_radius {
      BlameRadius::Voucher => {
        let wall = nodes[last];
        let mut out = vec![(wall, 1.0)];
        let voucher = nodes[last - 1];
        if voucher != ego && voucher != wall {
          out.push((voucher, 1.0));
        }
        out
      },
      BlameRadius::Prefix => {
        let mut seen: IntSet<NodeId> = IntSet::default();
        let mut out = vec![];
        let mut b = 1.0;
        for i in (1..=last).rev() {
          let node = nodes[i];
          if node != ego && seen.insert(node) && b > 0.0 {
            out.push((node, b));
          }
          b *= self.blame_decay;
        }
        out
      },
    }
  }

  fn add_contribution(
    &mut self,
    ego: NodeId,
    walk_id: WalkId,
  ) {
    let walk = match self.walks.get_walk(walk_id) {
      Some(w) if !w.is_empty() => w,
      _ => return,
    };
    if walk.absorbed {
      let blame = self.walk_blame(walk);
      let map = self.blame.entry(ego).or_default();
      for (node, b) in blame {
        let entry = map.entry(node).or_default();
        entry.sum += b;
        entry.walks += 1;
      }
    } else {
      self
        .pos_hits
        .entry(ego)
        .or_default()
        .increment_unique_counts(walk.get_nodes());
    }
  }

  fn remove_contribution(
    &mut self,
    ego: NodeId,
    walk_id: WalkId,
  ) {
    let walk = match self.walks.get_walk(walk_id) {
      Some(w) if !w.is_empty() => w,
      _ => return,
    };
    if walk.absorbed {
      let blame = self.walk_blame(walk);
      if let Some(map) = self.blame.get_mut(&ego) {
        for (node, b) in blame {
          if let Some(entry) = map.get_mut(&node) {
            entry.walks = entry.walks.saturating_sub(1);
            if entry.walks == 0 {
              map.remove(&node);
            } else {
              entry.sum -= b;
            }
          }
        }
      }
    } else if let Some(counter) = self.pos_hits.get_mut(&ego) {
      counter.decrement_unique_counts(walk.get_nodes());
    }
  }

  fn remove_block_contributions(
    &mut self,
    ego: NodeId,
  ) {
    if let Some(ids) = self.walks.block_walk_ids(ego) {
      for walk_id in ids {
        self.remove_contribution(ego, walk_id);
      }
    }
  }

  // ------------------------------------------------------------------
  // Frames
  // ------------------------------------------------------------------

  /// Drops the ego's walks and counters and frees their memory. Used when evicting an ego from
  /// cache.
  pub fn clear_ego(
    &mut self,
    ego: NodeId,
  ) -> Result<(), MeritRankError> {
    self.remove_block_contributions(ego);
    self.walks.release_block_for_ego(ego)?;
    self.pos_hits.remove(&ego);
    self.blame.remove(&ego);
    if self.calculated.remove(&ego) {
      self.dirty_egos.insert(ego);
    }
    Ok(())
  }

  pub fn calculate(
    &mut self,
    ego: NodeId,
  ) -> Result<(), MeritRankError> {
    self.calculated.insert(ego);
    self.dirty_egos.insert(ego);
    let start_id = self.walks.ensure_block_for_ego(ego)?;
    self.remove_block_contributions(ego);
    self.walks.clear_block(start_id)?;
    self.pos_hits.entry(ego).or_default();

    for walk_id in start_id..start_id + self.walks.walks_per_ego() {
      let walk = match self.walks.get_walk_mut(walk_id) {
        Some(x) => x,
        None => {
          return Err(MeritRankError::InternalFatalError(Some(
            internal_fatal::RANK_CALCULATE_GET_WALK_MUT,
          )));
        },
      };
      walk.push(ego)?;
      self.graph.continue_walk(walk, self.alpha, &mut self.rng)?;
      self.add_contribution(ego, walk_id);
      self.walks.update_walk_bookkeeping(walk_id, 0);
    }
    if ASSERT {
      self.walks.assert_visits_consistency()?;
      self.assert_counters_consistency()?;
    }
    Ok(())
  }

  /// `score_A(X) = (credits_X − λ·blame_X) / W` (R11).
  pub fn get_node_score(
    &self,
    ego: NodeId,
    target: NodeId,
  ) -> Result<Weight, MeritRankError> {
    let credits = self
      .pos_hits
      .get(&ego)
      .ok_or(MeritRankError::NodeIsNotCalculated)?
      .get_count(&target) as Weight;
    let blame = self.blame_of(ego, target);
    Ok((credits - self.discredit * blame) / self.walks.walks_per_ego() as Weight)
  }

  pub fn get_all_scores(
    &self,
    ego: NodeId,
    limit: Option<usize>,
  ) -> Result<Vec<(NodeId, Weight)>, MeritRankError> {
    let pos_counter = self
      .pos_hits
      .get(&ego)
      .ok_or(MeritRankError::NodeIsNotCalculated)?;
    let mut peers: IntSet<NodeId> = pos_counter.keys().copied().collect();
    if let Some(blame) = self.blame.get(&ego) {
      peers.extend(blame.keys().copied());
    }

    let mut peer_scores: Vec<_> = peers
      .into_iter()
      .map(|peer| self.get_node_score(ego, peer).map(|score| (peer, score)))
      .collect::<Result<_, _>>()?;

    peer_scores.sort_unstable_by(|(id1, score1), (id2, score2)| {
      score2
        .partial_cmp(score1)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(id1.cmp(id2))
    });
    peer_scores.truncate(limit.unwrap_or(peer_scores.len()));
    Ok(peer_scores)
  }

  pub fn get_new_nodeid(&mut self) -> NodeId {
    self.graph.get_new_nodeid()
  }

  // ------------------------------------------------------------------
  // Edges
  // ------------------------------------------------------------------

  /// Sets the weight of `src → dest`: positive = trust, negative = a wall of `src` on `dest`
  /// with strength `min(|w|, 1)`, 0 = delete. A change of an existing weight (including a sign
  /// transition) is a deletion followed by an addition.
  pub fn set_edge(
    &mut self,
    src: NodeId,
    dest: NodeId,
    new_weight: f64,
  ) -> Result<(), MeritRankError> {
    let old_weight = self
      .graph
      .edge_weight(src, dest)
      .expect("Node should exist!")
      .unwrap_or(0.0);

    if new_weight.is_nan() {
      panic!("Trying to set NaN weight for edge from {} to {}", src, dest);
    }
    if new_weight.is_infinite() {
      panic!(
        "Trying to set infinite weight for edge from {} to {}",
        src, dest
      );
    }

    // Wall → wall is a change of strength, re-coupled in place (R16). Anything else that
    // replaces an existing weight — trust → trust, and sign transitions — is a deletion followed
    // by an addition.
    let wall_to_wall = old_weight < 0.0 && new_weight < 0.0;
    if old_weight != 0.0 && new_weight != 0.0 && old_weight != new_weight && !wall_to_wall {
      self.set_edge_(src, dest, 0.0)?;
    }
    self.set_edge_(src, dest, new_weight)
  }

  pub fn set_edge_(
    &mut self,
    src: NodeId,
    dest: NodeId,
    new_weight: f64,
  ) -> Result<(), MeritRankError> {
    if src == dest {
      return Err(MeritRankError::SelfReferenceNotAllowed);
    }
    let old_weight = self
      .graph
      .edge_weight(src, dest)
      .expect("Node should exist!")
      .unwrap_or(0.0);
    if old_weight == new_weight {
      return Ok(());
    }
    if new_weight < 0.0 || (new_weight == 0.0 && old_weight < 0.0) {
      return self.set_wall(src, dest, new_weight);
    }
    self.set_positive_edge(src, dest, new_weight, old_weight)
  }

  /// A positive edge write: the optimized incremental invalidation over every ego's walks.
  fn set_positive_edge(
    &mut self,
    src: NodeId,
    dest: NodeId,
    new_weight: f64,
    old_weight: f64,
  ) -> Result<(), MeritRankError> {
    let deletion_mode = new_weight <= EPSILON;
    if deletion_mode && old_weight == 0.0 {
      // Deleting an absent edge (including a tiny weight on one) changes nothing.
      return Ok(());
    }
    // Without walks through `src` nothing is invalidated, so the probability (which forces the
    // node's lazy distribution to be built) is not needed; this keeps bulk loads O(edges).
    let src_is_visited = self
      .walks
      .get_visits_through_node(src)
      .map_or(false, |v| !v.is_empty());

    let mut step_recalc_probability: Option<Weight> = None;
    if OPTIMIZE_INVALIDATION && !deletion_mode && src_is_visited {
      let node_data = match self.graph.get_node_data(src) {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::RANK_SET_EDGE_GET_NODE_DATA_SRC,
        ))),
      };
      // Walks choose among positive edges only (R4): P(new edge) = w / (Σ w_pos + w).
      step_recalc_probability =
        Some(new_weight / (node_data.pos_sum() + new_weight));
    }

    if deletion_mode {
      self.graph.remove_edge(src, dest)?;
    } else {
      self.graph.set_edge(src, dest, new_weight)?;
    }

    let affected_walkids = if src_is_visited {
      self.walks.find_affected_walkids(
        src,
        Some(dest),
        step_recalc_probability,
        &mut self.rng,
      )?
    } else {
      vec![]
    };

    for (walk_id, visit_pos) in &affected_walkids {
      let ego = match self.walks.get_walk(*walk_id).and_then(|w| w.first_node()) {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::RANK_SET_EDGE_FIRST_NODE,
        ))),
      };
      self.dirty_egos.insert(ego);
      // Revert the walk's contribution, as if the walk never existed.
      self.remove_contribution(ego, *walk_id);

      let cut_position = visit_pos + 1;
      self
        .walks
        .split_and_remove_from_bookkeeping(walk_id, cut_position)?;

      let walk = match self.walks.get_walk_mut(*walk_id) {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::RANK_SET_EDGE_GET_WALK_MUT,
        ))),
      };

      let mut skip_continuation = false;
      if OPTIMIZE_INVALIDATION {
        if deletion_mode {
          self
            .graph
            .extend_walk_in_case_of_edge_deletion(walk, &mut self.rng)?;
        } else if self.rng.random::<f64>() < self.alpha {
          self.graph.step_into(walk, dest, &mut self.rng)?;
        } else {
          skip_continuation = true;
        }
      }
      if !skip_continuation {
        self.graph.continue_walk(walk, self.alpha, &mut self.rng)?;
      }

      self.add_contribution(ego, *walk_id);
      self.walks.update_walk_bookkeeping(*walk_id, cut_position);
    }

    if ASSERT {
      self.walks.assert_visits_consistency()?;
      self.assert_counters_consistency()?;
    }
    Ok(())
  }

  /// A wall write `ego ⊣ wall` (R16): only the owner's walks are touched, and they end up
  /// distributed as if regenerated under the new strength. Every arrival at the wall is an
  /// independent trial (D16), so each arrival is re-coupled:
  /// - strength rises `d₀ → d₁`: a passing arrival is absorbed with probability
  ///   `(d₁ − d₀)/(1 − d₀)`; the first success truncates the walk there;
  /// - strength falls: a walk absorbed at the wall stays absorbed with probability `d₁/d₀`,
  ///   otherwise it continues from the wall.
  fn set_wall(
    &mut self,
    ego: NodeId,
    wall: NodeId,
    new_weight: f64,
  ) -> Result<(), MeritRankError> {
    let d0 = self
      .graph
      .get_node_data(ego)
      .map_or(0.0, |d| d.wall_strength(wall));
    if new_weight == 0.0 {
      self.graph.remove_edge(ego, wall)?;
    } else {
      self.graph.set_edge(ego, wall, new_weight)?;
    }
    let d1 = if new_weight == 0.0 {
      0.0
    } else {
      new_weight.abs().min(1.0)
    };
    if d0 == d1 {
      return Ok(());
    }
    // An effective wall change marks its owner even when evicted (R16).
    self.dirty_egos.insert(ego);
    if !self.calculated.contains(&ego) {
      return Ok(());
    }

    // Candidates: the owner's walks that visit the wall, with their first arrival. Take them from
    // the wall's visits or from the owner's block, whichever is smaller (a hub wall is visited by
    // many walks of other egos).
    let block = match self.walks.block_walk_ids(ego) {
      Some(b) => b,
      None => return Ok(()),
    };
    let mut candidates: Vec<(WalkId, usize)> =
      match self.walks.get_visits_through_node(wall) {
        None => vec![],
        Some(visits) if visits.len() <= block.len() => visits
          .iter()
          .filter(|(id, _)| block.contains(id))
          .map(|(id, pos)| (*id, *pos))
          .collect(),
        Some(visits) => block
          .clone()
          .filter_map(|id| visits.get(&id).map(|pos| (id, *pos)))
          .collect(),
      };
    candidates.sort_unstable();

    for (walk_id, first_pos) in candidates {
      if d1 > d0 {
        let p = (d1 - d0) / (1.0 - d0);
        let walk = self.walks.get_walk(walk_id).ok_or(
          MeritRankError::InternalFatalError(Some(
            internal_fatal::RANK_SET_WALL_GET_WALK,
          )),
        )?;
        // Arrivals that passed; an absorbing arrival (the last node) stays absorbed.
        let passing_end = if walk.absorbed {
          walk.len() - 1
        } else {
          walk.len()
        };
        let mut cut = None;
        for i in first_pos..passing_end {
          if walk.get_nodes()[i] == wall && self.rng.random::<f64>() < p {
            cut = Some(i);
            break;
          }
        }
        if let Some(i) = cut {
          self.remove_contribution(ego, walk_id);
          self.walks.split_and_remove_from_bookkeeping(&walk_id, i + 1)?;
          if let Some(w) = self.walks.get_walk_mut(walk_id) {
            w.absorbed = true;
          }
          self.add_contribution(ego, walk_id);
        }
      } else {
        let walk = self.walks.get_walk(walk_id).ok_or(
          MeritRankError::InternalFatalError(Some(
            internal_fatal::RANK_SET_WALL_GET_WALK,
          )),
        )?;
        let absorbed_here = walk.absorbed && walk.last_node() == Some(wall);
        let old_len = walk.len();
        if !absorbed_here || self.rng.random::<f64>() < d1 / d0 {
          continue;
        }
        self.remove_contribution(ego, walk_id);
        let walk = self.walks.get_walk_mut(walk_id).ok_or(
          MeritRankError::InternalFatalError(Some(
            internal_fatal::RANK_SET_WALL_GET_WALK,
          )),
        )?;
        walk.absorbed = false;
        self.graph.continue_walk(walk, self.alpha, &mut self.rng)?;
        self.add_contribution(ego, walk_id);
        self.walks.update_walk_bookkeeping(walk_id, old_len);
      }
    }

    if ASSERT {
      self.walks.assert_visits_consistency()?;
      self.assert_counters_consistency()?;
    }
    Ok(())
  }

  /// Full internal consistency check: the visits index against the walks, and every calculated
  /// ego's credits and blame recounted from its walks. Debug builds run it after every change.
  pub fn verify(&self) -> Result<(), MeritRankError> {
    self.walks.assert_visits_consistency()?;
    self.assert_counters_consistency()
  }

  /// Recounts every calculated ego's credits and blame from its walks and compares (debug).
  fn assert_counters_consistency(&self) -> Result<(), MeritRankError> {
    for &ego in &self.calculated {
      let mut credits = Counter::default();
      let mut blame: IntMap<NodeId, (f64, u32)> = IntMap::default();
      if let Some(ids) = self.walks.block_walk_ids(ego) {
        for walk_id in ids {
          let walk = match self.walks.get_walk(walk_id) {
            Some(w) if !w.is_empty() => w,
            _ => continue,
          };
          if walk.absorbed {
            for (node, b) in self.walk_blame(walk) {
              let e = blame.entry(node).or_insert((0.0, 0));
              e.0 += b;
              e.1 += 1;
            }
          } else {
            credits.increment_unique_counts(walk.get_nodes());
          }
        }
      }
      let stored = self.pos_hits.get(&ego);
      let mut nodes: IntSet<NodeId> = credits.keys().copied().collect();
      if let Some(s) = stored {
        nodes.extend(s.keys().copied());
      }
      for node in nodes {
        let a = credits.get_count(&node);
        let b = stored.map_or(0, |s| s.get_count(&node));
        if a != b {
          return Err(MeritRankError::InternalFatalError(Some(
            internal_fatal::RANK_ASSERT_POS_HITS_COUNT,
          )));
        }
      }
      let empty = IntMap::default();
      let stored_blame = self.blame.get(&ego).unwrap_or(&empty);
      let mut nodes: IntSet<NodeId> = blame.keys().copied().collect();
      nodes.extend(stored_blame.keys().copied());
      for node in nodes {
        let (a, a_walks) = blame.get(&node).copied().unwrap_or((0.0, 0));
        let (b, b_walks) = stored_blame.get(&node).map_or((0.0, 0), |x| (x.sum, x.walks));
        if a_walks != b_walks || (a - b).abs() > 1e-9 * a.abs().max(1e-300) + 1e-12 * a.abs() {
          return Err(MeritRankError::InternalFatalError(Some(
            internal_fatal::RANK_ASSERT_BLAME,
          )));
        }
      }
    }
    Ok(())
  }

  pub fn print_walks(&self) {
    self.walks.print_walks();
  }

  // ------------------------------------------------------------------
  // D14: frames outside the walk storage, canonical counters, mutations
  // ------------------------------------------------------------------

  /// `calculate` with its own random stream seeded by `seed` (the resident stream is untouched):
  /// a fresh frame is a pure function of the graph, the ego, the walk count and the seed.
  pub fn calculate_seeded(
    &mut self,
    ego: NodeId,
    seed: u64,
  ) -> Result<(), MeritRankError> {
    let _ = (ego, seed);
    todo!("D14: calculate_seeded")
  }

  /// A frame of `n` fresh walks of `ego`, generated exactly as `calculate_seeded(ego, seed)`
  /// generates its first `n` walks, without touching the walk storage, the counters, the
  /// resident random stream or the dirty set. Only the graph's lazy distributions may be built.
  pub fn sample_frame(
    &self,
    ego: NodeId,
    n: usize,
    seed: u64,
  ) -> Result<crate::frame::FrameSample, MeritRankError> {
    let _ = (ego, n, seed);
    todo!("D14: sample_frame")
  }

  /// A copy of a resident frame (its counters and footprint), `None` if not calculated.
  pub fn frame_sample(
    &self,
    ego: NodeId,
  ) -> Option<crate::frame::FrameSample> {
    let _ = ego;
    todo!("D14: frame_sample")
  }

  /// The counters maintained for a resident frame.
  pub fn frame_counters(
    &self,
    ego: NodeId,
  ) -> Option<&crate::frame::FrameCounters> {
    let _ = ego;
    todo!("D14: frame_counters")
  }

  /// The counters of a resident frame recounted from its stored walks.
  pub fn recount_frame(
    &self,
    ego: NodeId,
  ) -> Option<crate::frame::FrameCounters> {
    let _ = ego;
    todo!("D14: recount_frame")
  }

  /// The stored walks of a resident frame, in slot order.
  pub fn ego_walks(
    &self,
    ego: NodeId,
  ) -> Option<Vec<&RandomWalk>> {
    let _ = ego;
    todo!("D14: ego_walks")
  }

  /// Credits of `node` in `ego`'s frame (0 if none).
  pub fn credits_of(
    &self,
    ego: NodeId,
    node: NodeId,
  ) -> u32 {
    let _ = (ego, node);
    todo!("D14: credits_of")
  }

  /// Graph changes since the previous call (see `frame::Mutations`).
  pub fn take_mutations(&mut self) -> crate::frame::Mutations {
    todo!("D14: take_mutations")
  }

  /// Whether `take_mutations` reports the TV of each changed source (costs O(degree) per write).
  pub fn set_tv_tracking(
    &mut self,
    on: bool,
  ) {
    let _ = on;
    todo!("D14: set_tv_tracking")
  }

  /// Credits per ego: the number of unabsorbed walks that visited each node.
  pub fn get_personal_hits(&self) -> &IntMap<NodeId, Counter> {
    &self.pos_hits
  }

  /// Blame of `node` in `ego`'s frame: `Σ b` over the absorbed walks, before λ.
  pub fn blame_of(
    &self,
    ego: NodeId,
    node: NodeId,
  ) -> f64 {
    self
      .blame
      .get(&ego)
      .and_then(|m| m.get(&node))
      .map_or(0.0, |b| b.sum)
  }

  /// Clears all walks and hit counters; graph structure is preserved. Used for bulk load cold start.
  pub fn clear_walks(&mut self) {
    self.walks.clear();
    self.pos_hits.clear();
    self.blame.clear();
    self.dirty_egos.extend(self.calculated.drain());
  }
}
