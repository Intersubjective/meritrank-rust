use integer_hasher::{IntMap, IntSet};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::constants::{ASSERT, EPSILON, OPTIMIZE_INVALIDATION};
use crate::errors::internal_fatal;
use crate::errors::MeritRankError;
use crate::frame::{
  distribution_tv, generate_walk_into, walk_contribution, FrameCounters, FrameSample, Mutations,
  SampleAccumulator, SourceChange,
};
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

#[derive(Clone)]
pub struct MeritRank {
  pub graph:        Graph,
  walks:            WalkStorage,
  /// Counters of every calculated ego's frame: credits and blame (λ is applied at read time).
  frames:           IntMap<NodeId, FrameCounters>,
  /// Egos whose walks exist. An ego can be calculated with empty counters.
  calculated:       IntSet<NodeId>,
  /// Egos whose walks or counters changed since the last `take_dirty_egos`.
  dirty_egos:       IntSet<NodeId>,
  /// Graph changes since the last `take_mutations`: changed positive sources (with their TV when
  /// tracked) and owners of changed walls.
  /// Per changed source: its positive out-edges before its first change (when tracking TV).
  changed_sources:  std::collections::BTreeMap<NodeId, Option<Vec<(NodeId, Weight)>>>,
  changed_walls:    std::collections::BTreeSet<NodeId>,
  tv_tracking:      bool,
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
      frames: IntMap::default(),
      calculated: IntSet::default(),
      dirty_egos: IntSet::default(),
      changed_sources: Default::default(),
      changed_walls: Default::default(),
      tv_tracking: false,
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

  /// Adds a stored walk's contribution to its ego's counters (`frame::walk_contribution`).
  fn add_contribution(
    &mut self,
    ego: NodeId,
    walk_id: WalkId,
  ) {
    self.apply_contribution(ego, walk_id, true);
  }

  /// Reverts a stored walk's contribution, as if the walk never existed.
  fn remove_contribution(
    &mut self,
    ego: NodeId,
    walk_id: WalkId,
  ) {
    self.apply_contribution(ego, walk_id, false);
  }

  fn apply_contribution(
    &mut self,
    ego: NodeId,
    walk_id: WalkId,
    add: bool,
  ) {
    let walk = match self.walks.get_walk(walk_id) {
      Some(w) if !w.is_empty() => w,
      _ => return,
    };
    let c = walk_contribution(walk, self.blame_radius, self.blame_decay);
    if let Err(e) = self.frames.entry(ego).or_default().apply(&c, add) {
      // Unreachable while the counters follow the walks; the consistency check reports it.
      log::error!("frame counters of ego {}: {}", ego, e);
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
    self.frames.remove(&ego);
    if self.calculated.remove(&ego) {
      self.dirty_egos.insert(ego);
    }
    Ok(())
  }

  pub fn calculate(
    &mut self,
    ego: NodeId,
  ) -> Result<(), MeritRankError> {
    // The resident stream is lent to the calculation and given back.
    let mut rng = std::mem::replace(&mut self.rng, StdRng::seed_from_u64(0));
    let result = self.calculate_with(ego, &mut rng);
    self.rng = rng;
    result
  }

  fn calculate_with(
    &mut self,
    ego: NodeId,
    rng: &mut StdRng,
  ) -> Result<(), MeritRankError> {
    if !self.graph.contains_node(ego) {
      return Err(MeritRankError::NodeNotFound);
    }
    self.calculated.insert(ego);
    self.dirty_egos.insert(ego);
    let start_id = self.walks.ensure_block_for_ego(ego)?;
    self.remove_block_contributions(ego);
    self.walks.clear_block(start_id)?;
    self.frames.insert(ego, FrameCounters::new());

    for walk_id in start_id..start_id + self.walks.walks_per_ego() {
      let walk = match self.walks.get_walk_mut(walk_id) {
        Some(x) => x,
        None => {
          return Err(MeritRankError::InternalFatalError(Some(
            internal_fatal::RANK_CALCULATE_GET_WALK_MUT,
          )));
        },
      };
      generate_walk_into(&self.graph, ego, self.alpha, rng, walk)?;
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
    let counters = self.frames.get(&ego).ok_or(MeritRankError::NodeIsNotCalculated)?;
    Ok(counters.score(target, self.discredit, self.blame_decay, self.walks.walks_per_ego()))
  }

  pub fn get_all_scores(
    &self,
    ego: NodeId,
    limit: Option<usize>,
  ) -> Result<Vec<(NodeId, Weight)>, MeritRankError> {
    let peers = self.frames.get(&ego).ok_or(MeritRankError::NodeIsNotCalculated)?.nodes();

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
    self.record_source_change(src);
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
    self.changed_walls.insert(ego);
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

  /// Recounts every calculated ego's counters from its walks and compares them, exactly (debug).
  fn assert_counters_consistency(&self) -> Result<(), MeritRankError> {
    for &ego in &self.calculated {
      let recount = self.recount(ego).transpose()?.unwrap_or_default();
      let kept = self.frames.get(&ego).cloned().unwrap_or_default();
      if recount != kept {
        return Err(MeritRankError::InternalFatalError(Some(
          internal_fatal::RANK_ASSERT_POS_HITS_COUNT,
        )));
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
    let mut rng = StdRng::seed_from_u64(seed);
    self.calculate_with(ego, &mut rng)
  }

  /// A frame of `n` fresh walks of `ego`, generated exactly as `calculate_seeded(ego, seed)`
  /// generates its first `n` walks, without touching the walk storage, the counters, the
  /// resident random stream or the dirty set. Only the graph's lazy distributions may be built.
  pub fn sample_frame(
    &self,
    ego: NodeId,
    n: usize,
    seed: u64,
  ) -> Result<FrameSample, MeritRankError> {
    if !self.graph.contains_node(ego) {
      return Err(MeritRankError::NodeNotFound);
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let mut walk = RandomWalk::new();
    let mut acc = SampleAccumulator::new(ego);
    for _ in 0..n {
      generate_walk_into(&self.graph, ego, self.alpha, &mut rng, &mut walk)?;
      acc.add(&walk, self.blame_radius, self.blame_decay)?;
    }
    Ok(acc.finish())
  }

  /// A copy of a resident frame (its counters and footprint), `None` if not calculated.
  pub fn frame_sample(
    &self,
    ego: NodeId,
  ) -> Option<FrameSample> {
    if !self.is_calculated(ego) {
      return None;
    }
    let walks = self.ego_walks(ego)?;
    let mut sample =
      FrameSample::from_walks(ego, walks, self.blame_radius, self.blame_decay).ok()?;
    sample.n = self.walks.walks_per_ego();
    sample.counters = self.frames.get(&ego).cloned().unwrap_or_default();
    Some(sample)
  }

  /// The counters maintained for a resident frame.
  pub fn frame_counters(
    &self,
    ego: NodeId,
  ) -> Option<&FrameCounters> {
    if !self.is_calculated(ego) {
      return None;
    }
    self.frames.get(&ego)
  }

  /// The counters of a resident frame recounted from its stored walks.
  pub fn recount_frame(
    &self,
    ego: NodeId,
  ) -> Option<FrameCounters> {
    self.recount(ego).and_then(|r| r.ok())
  }

  fn recount(
    &self,
    ego: NodeId,
  ) -> Option<Result<FrameCounters, MeritRankError>> {
    let walks = self.ego_walks(ego)?;
    Some(FrameCounters::from_walks(walks, self.blame_radius, self.blame_decay))
  }

  /// The stored walks of a resident frame, in slot order.
  pub fn ego_walks(
    &self,
    ego: NodeId,
  ) -> Option<Vec<&RandomWalk>> {
    let ids = self.walks.block_walk_ids(ego)?;
    Some(ids.filter_map(|id| self.walks.get_walk(id)).collect())
  }

  /// Credits of `node` in `ego`'s frame (0 if none).
  pub fn credits_of(
    &self,
    ego: NodeId,
    node: NodeId,
  ) -> u32 {
    self.frames.get(&ego).map_or(0, |c| c.credits(node))
  }

  /// Graph changes since the previous call (see `frame::Mutations`).
  pub fn take_mutations(&mut self) -> Mutations {
    let changed = std::mem::take(&mut self.changed_sources);
    let sources = changed
      .into_iter()
      .map(|(src, before)| {
        let tv = before.map(|before| {
          let after: Vec<(NodeId, Weight)> = self
            .graph
            .get_node_data(src)
            .map(|d| d.pos_edges.iter().map(|(&n, &w)| (n, w)).collect())
            .unwrap_or_default();
          distribution_tv(&before, &after)
        });
        SourceChange { src, tv }
      })
      .collect();
    let wall_owners = std::mem::take(&mut self.changed_walls).into_iter().collect();
    Mutations { sources, wall_owners }
  }

  /// Whether `take_mutations` reports the TV of each changed source (costs O(degree) per write).
  pub fn set_tv_tracking(
    &mut self,
    on: bool,
  ) {
    self.tv_tracking = on;
  }

  /// Records an effective change of a positive out-edge of `src` (before the graph changes):
  /// with TV tracking, its out-edges before its first change since the last `take_mutations`.
  fn record_source_change(
    &mut self,
    src: NodeId,
  ) {
    if self.changed_sources.contains_key(&src) {
      return;
    }
    let before = self.tv_tracking.then(|| {
      self
        .graph
        .get_node_data(src)
        .map(|d| d.pos_edges.iter().map(|(&n, &w)| (n, w)).collect())
        .unwrap_or_default()
    });
    self.changed_sources.insert(src, before);
  }

  /// Blame of `node` in `ego`'s frame: `Σ b` over the absorbed walks, before λ.
  pub fn blame_of(
    &self,
    ego: NodeId,
    node: NodeId,
  ) -> f64 {
    self.frames.get(&ego).map_or(0.0, |c| c.blame_sum(node, self.blame_decay))
  }

  /// Clears all walks and hit counters; graph structure is preserved. Used for bulk load cold start.
  pub fn clear_walks(&mut self) {
    self.walks.clear();
    self.frames.clear();
    self.dirty_egos.extend(self.calculated.drain());
  }
}
