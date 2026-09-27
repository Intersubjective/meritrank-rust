use rand::rand_core::RngCore;
use rand::Rng;

use integer_hasher::IntMap;

use crate::constants::OPTIMIZE_INVALIDATION;
use crate::errors::internal_fatal;
use crate::graph::{EdgeId, NodeId, Weight};
use crate::random_walk::RandomWalk;
use crate::MeritRankError;

pub type WalkId = usize;

/// Represents a storage container for walks in the MeritRank graph.
/// Each ego owns a fixed-size contiguous block of walk slots.
#[derive(Clone)]
pub struct WalkStorage {
  visits:         Vec<IntMap<WalkId, usize>>,
  walks:          Vec<RandomWalk>,
  walks_per_ego:  usize,
  ego_blocks:     IntMap<NodeId, WalkId>,
  /// Start ids of blocks released by evicted egos, reused before the storage grows.
  free_blocks:    Vec<WalkId>,
}

impl WalkStorage {
  pub fn new(walks_per_ego: usize) -> Self {
    WalkStorage {
      visits:        Vec::new(),
      walks:         Vec::new(),
      walks_per_ego,
      ego_blocks:    IntMap::default(),
      free_blocks:   Vec::new(),
    }
  }

  /// Number of walk slots allocated (occupied or free).
  pub fn allocated_walks(&self) -> usize {
    self.walks.len()
  }

  /// Total capacity of the visits index (entries it can hold without reallocating).
  pub fn visits_capacity(&self) -> usize {
    self.visits.iter().map(|m| m.capacity()).sum()
  }

  pub fn walks_per_ego(&self) -> usize {
    self.walks_per_ego
  }

  /// Returns the start walk ID for the given ego's block, if any.
  pub fn get_block_start(&self, ego: NodeId) -> Option<WalkId> {
    self.ego_blocks.get(&ego).copied()
  }

  pub fn get_walk(
    &self,
    uid: WalkId,
  ) -> Option<&RandomWalk> {
    self.walks.get(uid)
  }

  pub fn get_walk_mut(
    &mut self,
    uid: WalkId,
  ) -> Option<&mut RandomWalk> {
    self.walks.get_mut(uid)
  }

  pub fn get_walks(&self) -> &Vec<IntMap<WalkId, usize>> {
    &self.visits
  }

  pub fn get_visits_through_node(
    &self,
    node_id: NodeId,
  ) -> Option<&IntMap<WalkId, usize>> {
    self.visits.get(node_id)
  }

  /// Ensures the given ego has a block of walk slots; returns the start index.
  pub fn ensure_block_for_ego(
    &mut self,
    ego: NodeId,
  ) -> Result<WalkId, MeritRankError> {
    if let Some(&start) = self.ego_blocks.get(&ego) {
      return Ok(start);
    }
    let start = match self.free_blocks.pop() {
      Some(start) => start,
      None => {
        let start = self.walks.len() as WalkId;
        for _ in 0..self.walks_per_ego {
          self.walks.push(RandomWalk::new());
        }
        start
      },
    };
    self.ego_blocks.insert(ego, start);
    Ok(start)
  }


  /// Walk ids of the ego's block, if it has one.
  pub fn block_walk_ids(
    &self,
    ego: NodeId,
  ) -> Option<std::ops::Range<WalkId>> {
    let start = *self.ego_blocks.get(&ego)?;
    Some(start..start + self.walks_per_ego)
  }

  /// Empties every walk of the block starting at `start_id` and removes them from the visits
  /// index. Counters are the caller's (rank's) business: it removes the walks' contributions first.
  pub fn clear_block(
    &mut self,
    start_id: WalkId,
  ) -> Result<(), MeritRankError> {
    for walk_id in start_id..start_id + self.walks_per_ego {
      let walk = match self.walks.get_mut(walk_id) {
        Some(w) => w,
        None => {
          return Err(MeritRankError::InternalFatalError(Some(
            internal_fatal::WALK_STORAGE_SPLIT_GET_MUT,
          )));
        },
      };
      for &node in walk.nodes.iter() {
        if let Some(visits) = self.visits.get_mut(node) {
          visits.remove(&walk_id);
          // Release the allocation of a map that became empty, or the index keeps memory for
          // every ego ever calculated despite eviction.
          if visits.is_empty() && visits.capacity() > 0 {
            *visits = IntMap::default();
          }
        }
      }
      walk.clear();
    }
    Ok(())
  }

  /// Clears the ego's block (as `clear_block`), frees the walks' memory and returns the block to
  /// the free list. Used when an ego is evicted.
  pub fn release_block_for_ego(
    &mut self,
    ego: NodeId,
  ) -> Result<(), MeritRankError> {
    let start = match self.ego_blocks.remove(&ego) {
      Some(start) => start,
      None => return Ok(()),
    };
    self.clear_block(start)?;
    for walk in &mut self.walks[start..start + self.walks_per_ego] {
      *walk = RandomWalk::new();
    }
    self.free_blocks.push(start);
    Ok(())
  }

  pub fn update_walk_bookkeeping(
    &mut self,
    walk_id: WalkId,
    start_pos: usize,
  ) {
    if let Some(walk) = self.walks.get(walk_id) {
      for (pos, &node) in walk.get_nodes().iter().enumerate().skip(start_pos) {
        if self.visits.len() < node + 1 {
          self.visits.resize(node + 1, IntMap::default());
        }
        self.visits[node].entry(walk_id).or_insert(pos);
      }
    }
  }

  pub fn print_walks(&self) {
    for walk in &self.walks {
      println!("{:?}", *walk);
    }
  }

  /// Clears all walks and visit bookkeeping. Used for bulk load cold start.
  pub fn clear(&mut self) {
    self.visits.clear();
    self.walks.clear();
    self.ego_blocks.clear();
    self.free_blocks.clear();
  }

  pub fn assert_visits_consistency(&self) -> Result<(), MeritRankError> {
    for (node, visits) in self.visits.iter().enumerate() {
      for (walkid, pos) in visits.iter() {
        if self.walks[*walkid].nodes[*pos] != node {
          return Err(MeritRankError::InternalFatalError(Some(
            internal_fatal::WALK_STORAGE_ASSERT_VISITS,
          )));
        }
      }
    }
    Ok(())
  }

  /// Returns a walk IDs and cut positions for the walks affected by introducing new outgoing
  /// edge at invalidated_node.
  pub fn find_affected_walkids<R: Rng + ?Sized>(
    &self,
    invalidated_node: NodeId,
    dst_node: Option<NodeId>,
    step_recalc_probability: Option<Weight>,
    rng: &mut R,
  ) -> Result<Vec<(WalkId, usize)>, MeritRankError> {
    let mut invalidated_walks_ids = vec![];

    // Check if there are any walks passing through the invalidated node
    let walks = match self.visits.get(invalidated_node) {
      Some(walks) => walks,
      None => return Ok(invalidated_walks_ids),
    };

    for (walk_id, visit_pos) in walks {
      let _new_pos = if OPTIMIZE_INVALIDATION && dst_node.is_some() {
        let (may_skip, new_pos) = decide_skip_invalidation(
          match self.get_walk(*walk_id) {
            Some(x) => x,
            None => return Err(MeritRankError::InternalFatalError(Some(
              internal_fatal::WALK_STORAGE_FIND_AFFECTED_GET_WALK,
            ))),
          },
          *visit_pos,
          (
            invalidated_node,
            match dst_node {
              Some(x) => x,
              None => return Err(MeritRankError::InternalFatalError(Some(
                internal_fatal::WALK_STORAGE_FIND_AFFECTED_DST_NONE,
              ))),
            },
          ),
          step_recalc_probability,
          Some(&mut *rng),
        )?;
        if may_skip {
          // Skip invalidating this walk if it is determined to be unnecessary
          continue;
        }
        new_pos
      } else {
        *visit_pos
      };

      invalidated_walks_ids.push((*walk_id, _new_pos));
    }

    Ok(invalidated_walks_ids)
  }

  pub fn split_and_remove_from_bookkeeping(
    &mut self,
    walk_id: &WalkId,
    cut_pos: usize,
  ) -> Result<(), MeritRankError> {
    // Cut position is the index of the first element of the invalidated segment
    // Split the walk and obtain the invalidated segment
    let walk = match self.walks.get_mut(*walk_id) {
      Some(x) => x,
      None => return Err(MeritRankError::InternalFatalError(Some(
        internal_fatal::WALK_STORAGE_SPLIT_GET_MUT,
      ))),
    };
    let invalidated_segment = walk.split_from(cut_pos);

    // Remove affected nodes from bookkeeping, but ensure we don't accidentally remove references
    // if there are still copies of the affected node in the remaining walk
    for &affected_node in invalidated_segment
      .get_nodes()
      .iter()
      .filter(|&node| !walk.contains(node))
    {
      if let Some(affected_walks) = self.visits.get_mut(affected_node) {
        if affected_walks.get(walk_id).is_some() {
          // Remove the invalidated walk from affected nodes
          affected_walks.remove(walk_id);
        }
      }
    }

    Ok(())
  }
}

pub fn decide_skip_invalidation<R>(
  walk: &RandomWalk,
  pos: usize,
  edge: EdgeId,
  step_recalc_probability: Option<Weight>,
  rnd: Option<R>,
) -> Result<(bool, usize), MeritRankError>
where
  R: RngCore,
{
  if let Some(prob) = step_recalc_probability {
    decide_skip_invalidation_on_edge_addition(walk, pos, edge, prob, rnd)
  } else {
    decide_skip_invalidation_on_edge_deletion(walk, pos, edge)
  }
}
pub fn decide_skip_invalidation_on_edge_deletion(
  walk: &RandomWalk,
  pos: usize,
  edge: EdgeId,
) -> Result<(bool, usize), MeritRankError> {
  if pos >= walk.len() {
    return Err(MeritRankError::InternalFatalError(Some(
      internal_fatal::WALK_DECIDE_SKIP_DELETION_POS,
    )));
  }

  let (invalidated_node, dst_node) = edge;

  if pos == walk.len() - 1 {
    return Ok((true, pos));
  }

  Ok(
    walk.get_nodes()[pos..walk.len() - 1]
      .iter()
      .enumerate()
      .find_map(|(i, &node)| {
        if node == invalidated_node && walk.get_nodes()[pos + i + 1] == dst_node
        {
          Some((false, pos + i))
        } else {
          None
        }
      })
      .unwrap_or((true, pos)),
  )
}

/// Edge addition at `invalidated_node`: every visit of the node re-decides its step, taking the
/// new edge with probability `prob` (the coupling: stop stays `1 - alpha`, the new edge gets
/// `alpha * prob`, old edges keep their share). The last position of an absorbed walk is not a
/// visit that decided a step — the walk was absorbed on arrival — so it is never re-coupled
/// (otherwise the out-edges of a wall would reach the nodes before it, breaking A1).
pub fn decide_skip_invalidation_on_edge_addition<R>(
  walk: &RandomWalk,
  pos: usize,
  edge: EdgeId,
  prob: Weight,
  mut rnd: Option<R>,
) -> Result<(bool, usize), MeritRankError>
where
  R: RngCore,
{
  if pos >= walk.len() {
    return Err(MeritRankError::InternalFatalError(Some(
      internal_fatal::WALK_DECIDE_SKIP_ADDITION_POS,
    )));
  }

  let (invalidated_node, _dst_node) = edge;

  let mut fallback_rng = rand::rng();
  let rng = rnd
    .as_mut()
    .map(|r| r as &mut dyn RngCore)
    .unwrap_or(&mut fallback_rng);

  let decided_steps = if walk.absorbed {
    walk.len() - 1
  } else {
    walk.len()
  };

  let mut new_pos = pos;
  let result = walk.get_nodes()[pos..decided_steps.max(pos)]
    .iter()
    .enumerate()
    .find_map(|(i, &node)| {
      if node == invalidated_node {
        new_pos = pos + i;
        if prob > 0.0 && rng.random::<Weight>() < prob {
          Some(false) // invalidate at this position
        } else {
          None // skip this occurrence, keep scanning
        }
      } else {
        None
      }
    });

  Ok((result.is_none(), new_pos))
}
