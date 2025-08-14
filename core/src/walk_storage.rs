use rand::prelude::*;
use std::collections::VecDeque;

use integer_hasher::IntMap;

use crate::constants::OPTIMIZE_INVALIDATION;
use crate::graph::{EdgeId, NodeId, Weight};
use crate::random_walk::RandomWalk;
use crate::MeritRankError;

pub type WalkId = usize;

/// Represents a storage container for walks in the MeritRank graph.
#[derive(Clone)]
pub struct WalkStorage {
  visits:       Vec<IntMap<WalkId, usize>>,
  walks:        Vec<RandomWalk>,
  unused_walks: VecDeque<WalkId>,
}

impl WalkStorage {
  pub fn new() -> Self {
    WalkStorage {
      visits:       Vec::new(),
      walks:        Vec::new(),
      unused_walks: VecDeque::new(),
    }
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

  pub fn get_next_free_walkid(&mut self) -> WalkId {
    match self.unused_walks.pop_front() {
      Some(id) => id,
      None => {
        let id = self.walks.len() as WalkId;
        self.walks.push(RandomWalk::new());
        id
      },
    }
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

  pub fn drop_walks_from_node(
    &mut self,
    node: NodeId,
  ) -> Result<(), MeritRankError> {
    // Check if there are any visits for the given node
    if let Some(visits_for_node) = self.visits.get_mut(node) {
      // Identify the walks that start from the given node (i.e., position is 0)
      let walkids_to_remove: Vec<WalkId> = visits_for_node
        .iter()
        .filter_map(|(key, &pos)| if pos == 0 { Some(*key) } else { None })
        .collect();

      // Remove the identified walks
      for walk_id in walkids_to_remove {
        // Safely get the walk to remove using the walk_id
        if let Some(walk_to_remove) = self.walks.get(walk_id) {
          // Iterate over the nodes in the walk and remove the walk_id from their visits
          for node in walk_to_remove.iter() {
            if let Some(visits) = self.visits.get_mut(*node) {
              visits.remove(&walk_id);
            }
          }
        }
        self.unused_walks.push_back(walk_id);
        match self.walks.get_mut(walk_id) {
          Some(x) => x.clear(),
          None => return Err(MeritRankError::InternalFatalError),
        };
      }
    }

    Ok(())
  }

  pub fn assert_visits_consistency(&self) -> Result<(), MeritRankError> {
    for (node, visits) in self.visits.iter().enumerate() {
      for (walkid, pos) in visits.iter() {
        if self.walks[*walkid].nodes[*pos] != node {
          return Err(MeritRankError::InternalFatalError);
        }
      }
    }
    Ok(())
  }

  /// Returns a walk IDs and cut positions for the walks affected by introducing new outgoing
  /// edge at invalidated_node.
  pub fn find_affected_walkids(
    &self,
    invalidated_node: NodeId,
    dst_node: Option<NodeId>,
    step_recalc_probability: Option<Weight>,
  ) -> Result<Vec<(WalkId, usize)>, MeritRankError> {
    let mut invalidated_walks_ids = vec![];

    // Check if there are any walks passing through the invalidated node
    let walks = match self.visits.get(invalidated_node) {
      Some(walks) => walks,
      None => return Ok(invalidated_walks_ids),
    };

    for (walk_id, visit_pos) in walks {
      let _new_pos = if OPTIMIZE_INVALIDATION && dst_node.is_some() {
        let mut rng = thread_rng();
        let (may_skip, new_pos) = decide_skip_invalidation(
          match self.get_walk(*walk_id) {
            Some(x) => x,
            None => return Err(MeritRankError::InternalFatalError),
          },
          *visit_pos,
          (
            invalidated_node,
            match dst_node {
              Some(x) => x,
              None => return Err(MeritRankError::InternalFatalError),
            },
          ),
          step_recalc_probability,
          Some(&mut rng),
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
      None => return Err(MeritRankError::InternalFatalError),
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
  if let Some(probability) = step_recalc_probability {
    decide_skip_invalidation_on_edge_addition(walk, pos, edge, probability, rnd)
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
    return Err(MeritRankError::InternalFatalError);
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

pub fn decide_skip_invalidation_on_edge_addition<R>(
  walk: &RandomWalk,
  pos: usize,
  edge: EdgeId,
  step_recalc_probability: Weight,
  mut rnd: Option<R>,
) -> Result<(bool, usize), MeritRankError>
where
  R: RngCore,
{
  if pos >= walk.len() {
    return Err(MeritRankError::InternalFatalError);
  }

  let (invalidated_node, _dst_node) = edge;

  let mut thread_rng = thread_rng();
  let rng = rnd
    .as_mut()
    .map(|r| r as &mut dyn RngCore)
    .unwrap_or(&mut thread_rng);

  let mut new_pos = pos;
  let result =
    walk.get_nodes()[pos..]
      .iter()
      .enumerate()
      .find_map(|(i, &node)| {
        if node == invalidated_node {
          new_pos = pos + i;
          if rng.gen::<Weight>() < step_recalc_probability {
            Some(false) // may_skip = false, exit early
          } else {
            None // continue searching
          }
        } else {
          None // continue searching
        }
      });

  Ok((result.is_none(), new_pos))
}
