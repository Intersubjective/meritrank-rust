use integer_hasher::IntMap;
use rand::prelude::*;

use crate::constants::{ASSERT, EPSILON, OPTIMIZE_INVALIDATION};
use crate::counter::Counter;
use crate::errors::MeritRankError;
use crate::graph::{Graph, NodeId, Weight};
use crate::walk_storage::WalkStorage;

#[derive(Clone)]
pub struct MeritRank {
  pub graph: Graph,
  walks:     WalkStorage,
  pos_hits:  IntMap<NodeId, Counter>,
  neg_hits:  IntMap<NodeId, Counter>,
  pub alpha: Weight,
}

impl MeritRank {
  pub fn new(graph: Graph) -> Self {
    Self {
      graph,
      walks: WalkStorage::new(),
      pos_hits: IntMap::default(),
      neg_hits: IntMap::default(),
      alpha: 0.85,
    }
  }

  pub fn calculate(
    &mut self,
    ego: NodeId,
    num_walks: usize,
  ) -> Result<(), MeritRankError> {
    self.walks.drop_walks_from_node(ego)?;

    for _ in 0..num_walks {
      let new_walk_id = self.walks.get_next_free_walkid();
      let walk = match self.walks.get_walk_mut(new_walk_id) {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError),
      };
      assert_eq!(walk.len(), 0);
      walk.push(ego, true)?;

      self.graph.continue_walk(walk, self.alpha)?;

      self
        .pos_hits
        .entry(ego)
        .or_default()
        .increment_unique_counts(walk.positive_subsegment());
      self
        .neg_hits
        .entry(ego)
        .or_default()
        .increment_unique_counts(walk.negative_subsegment());

      self.walks.update_walk_bookkeeping(new_walk_id, 0);
    }
    if ASSERT {
      self.walks.assert_visits_consistency()?;
      self.assert_counters_consistency_after_edge_addition()?;
    }

    Ok(())
  }

  pub fn get_node_score(
    &self,
    ego: NodeId,
    target: NodeId,
  ) -> Result<Weight, MeritRankError> {
    let ego_positive_hits = self
      .pos_hits
      .get(&ego)
      .ok_or(MeritRankError::NodeIsNotCalculated)?;

    let target_hits = ego_positive_hits.get_count(&target);

    //if ASSERT && hits > 0.0 && !self.graph.is_connecting(ego, target) { return Err(MeritRankError::NoPathExists); }

    let default_counter = Counter::default();

    let ego_neg_hits = self.neg_hits.get(&ego).unwrap_or(&default_counter);
    let total_hits =
      ego_positive_hits.total_count() + ego_neg_hits.total_count();
    let hits_penalized: Weight =
      target_hits as Weight - ego_neg_hits.get_count(&target) as Weight;
    Ok(hits_penalized / total_hits as Weight)
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

    let neg_counter = self
      .neg_hits
      .get(&ego)
      .ok_or(MeritRankError::NodeIsNotCalculated)?;

    let combined_counter = pos_counter
      .keys()
      .chain(neg_counter.keys())
      .collect::<std::collections::HashSet<_>>();

    let mut peer_scores: Vec<_> = combined_counter
      .into_iter()
      .map(|&peer| self.get_node_score(ego, peer).map(|score| (peer, score)))
      .collect::<Result<_, _>>()?;

    peer_scores.sort_unstable_by(|(_, score1), (_, score2)| {
      score2
        .partial_cmp(score1)
        .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(
      peer_scores
        .clone()
        .into_iter()
        .take(limit.unwrap_or(peer_scores.len()))
        .collect(),
    )
  }

  pub fn get_new_nodeid(&mut self) -> NodeId {
    self.graph.get_new_nodeid()
  }

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

    if old_weight.abs() > EPSILON && new_weight.abs() > EPSILON {
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
    let deletion_mode = new_weight.abs() <= EPSILON;
    let mut step_recalc_probability = None;

    if OPTIMIZE_INVALIDATION && !deletion_mode {
      step_recalc_probability = Some(
        new_weight.abs()
          / (match self.graph.get_node_data(src) {
            Some(x) => x.abs_sum(),
            None => return Err(MeritRankError::InternalFatalError),
          } + new_weight.abs()),
      );
    }

    if deletion_mode {
      self.graph.remove_edge(src, dest)?;
    } else {
      self.graph.set_edge(src, dest, new_weight)?;
    }

    let affected_walkids = self.walks.find_affected_walkids(
      src,
      Some(dest),
      step_recalc_probability,
    )?;

    for (walk_id, visit_pos) in &affected_walkids {
      // Revert the counters associated with the affected walks, as if the walks never existed
      let walk = match self.walks.get_walk(*walk_id) {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError),
      };
      let ego = match walk.first_node() {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError),
      };
      self
        .pos_hits
        .entry(ego)
        .or_default()
        .decrement_unique_counts(walk.positive_subsegment());
      self
        .neg_hits
        .entry(ego)
        .or_default()
        .decrement_unique_counts(walk.negative_subsegment());

      let cut_position = visit_pos + 1;
      self
        .walks
        .split_and_remove_from_bookkeeping(walk_id, cut_position)?;

      let walk = match self.walks.get_walk_mut(*walk_id) {
        Some(x) => x,
        None => return Err(MeritRankError::InternalFatalError),
      };

      let mut skip_continuation = false;
      //#[cfg(optimize_invalidation)]
      if OPTIMIZE_INVALIDATION {
        if deletion_mode {
          self.graph.extend_walk_in_case_of_edge_deletion(walk)?;
        } else if random::<f64>() < self.alpha {
          walk.push(dest, new_weight > 0.0)?;
        } else {
          skip_continuation = true;
        }
      }
      if !skip_continuation {
        self.graph.continue_walk(walk, self.alpha)?;
      }

      // Update counters associated with the updated walks
      self
        .pos_hits
        .entry(ego)
        .or_default()
        .increment_unique_counts(walk.positive_subsegment());
      self
        .neg_hits
        .entry(ego)
        .or_default()
        .increment_unique_counts(walk.negative_subsegment());

      self.walks.update_walk_bookkeeping(*walk_id, cut_position);
    }

    if ASSERT {
      self.walks.assert_visits_consistency()?;
      self.assert_counters_consistency_after_edge_addition()?;
    }

    Ok(())
  }

  fn assert_counters_consistency_after_edge_addition(
    &self
  ) -> Result<(), MeritRankError> {
    for (ego, hits) in &self.pos_hits {
      for (peer, count) in hits {
        let visits = match self.walks.get_visits_through_node(*peer) {
          Some(x) => x,
          None => return Err(MeritRankError::InternalFatalError),
        };
        let walks: Vec<_> = visits
          .iter()
          .filter(|&(walkid, pos)| {
            if let Some(walk) = self.walks.get_walk(*walkid) {
              walk.get_nodes().first() == Some(ego)
                && walk
                  .negative_segment_start
                  .map_or(true, |seg_start| *pos < seg_start)
            } else {
              false
            }
          })
          .collect();

        if walks.len() != *count as usize {
          return Err(MeritRankError::InternalFatalError);
        }
        // if !(*count == 0.0 || weight <= EPSILON || self.graph.is_connecting(*ego, *peer)) {
        //   return Err(MeritRankError::InternalFatalError);
        // }
      }
    }
    for (ego, hits) in &self.neg_hits {
      for (peer, count) in hits {
        let visits = self.walks.get_visits_through_node(*peer).unwrap();
        let walks: Vec<_> = visits
          .iter()
          .filter(|&(walkid, _)| {
            if let Some(walk) = self.walks.get_walk(*walkid) {
              walk.get_nodes().first() == Some(ego)
                && walk.negative_subsegment().any(|&x| x == *peer)
            } else {
              false
            }
          })
          .collect();

        if walks.len() != *count as usize {
          return Err(MeritRankError::InternalFatalError);
        }
        // if !(*count == 0.0 || weight <= EPSILON || self.graph.is_connecting(*ego, *peer)) {
        //   return Err(MeritRankError::InternalFatalError);
        // }
      }
    }
    Ok(())
  }

  pub fn print_walks(&self) {
    self.walks.print_walks();
  }

  pub fn get_personal_hits(&self) -> &IntMap<NodeId, Counter> {
    &self.pos_hits
  }
}
