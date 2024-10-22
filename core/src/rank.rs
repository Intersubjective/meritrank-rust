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
    walks: WalkStorage,
    pos_hits: IntMap<NodeId, Counter>,
    neg_hits: IntMap<NodeId, Counter>,
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

    pub fn calculate(&mut self, ego: NodeId, num_walks: usize) -> Result<(), MeritRankError> {
        self.walks.drop_walks_from_node(ego);

        for _ in 0..num_walks {
            let new_walk_id = self.walks.get_next_free_walkid();
            let walk = self.walks.get_walk_mut(new_walk_id).unwrap();
            assert_eq!(walk.len(), 0);
            walk.push(ego, true);

            self.graph.continue_walk(walk, self.alpha);

            self.pos_hits
                .entry(ego)
                .or_default()
                .increment_unique_counts(walk.positive_subsegment());
            self.neg_hits
                .entry(ego)
                .or_default()
                .increment_unique_counts(walk.negative_subsegment());

            self.walks.update_walk_bookkeeping(new_walk_id, 0);
        }
        if ASSERT {
            self.walks.assert_visits_consistency();
            self.assert_counters_consistency_after_edge_addition();
        }

        Ok(())
    }

    pub fn get_node_score(&self, ego: NodeId, target: NodeId) -> Result<Weight, MeritRankError> {
        let counter = self
            .pos_hits
            .get(&ego)
            .ok_or(MeritRankError::NodeIsNotCalculated)?;

        let hits = counter.get_count(&target);

        //if ASSERT && hits > 0.0 && !self.graph.is_connecting(ego, target) { return Err(MeritRankError::NoPathExists); }

        let default_counter = Counter::default();

        let ego_neg_hits = self.neg_hits.get(&ego).unwrap_or(&default_counter);
        let hits_penalized: Weight = hits as Weight - ego_neg_hits.get_count(&target) as Weight;
        Ok(hits_penalized / counter.total_count() as Weight)
    }

    pub fn get_ranks(
        &self,
        ego: NodeId,
        limit: Option<usize>,
    ) -> Result<Vec<(NodeId, Weight)>, MeritRankError> {
        let counter = self
            .pos_hits
            .get(&ego)
            .ok_or(MeritRankError::NodeIsNotCalculated)?;

        let mut peer_scores: Vec<_> = counter
            .keys()
            .map(|&peer| self.get_node_score(ego, peer).map(|score| (peer, score)))
            .collect::<Result<_, _>>()?;

        peer_scores.sort_unstable_by(|(_, score1), (_, score2)| {
            score2
                .partial_cmp(score1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(peer_scores
            .clone()
            .into_iter()
            .take(limit.unwrap_or(peer_scores.len()))
            .collect())
    }

    pub fn get_new_nodeid(&mut self) -> NodeId {
        self.graph.get_new_nodeid()
    }

    pub fn set_edge(&mut self, src: NodeId, dest: NodeId, new_weight: f64) {
        let old_weight = self
            .graph
            .edge_weight(src, dest)
            .expect("Node should exist!")
            .unwrap_or(0.0);
        if old_weight.abs() > EPSILON && new_weight.abs() > EPSILON {
            self.set_edge_(src, dest, 0.0);
        }
        self.set_edge_(src, dest, new_weight);
    }

    pub fn set_edge_(&mut self, src: NodeId, dest: NodeId, new_weight: f64) {
        assert_ne!(src, dest, "Self reference not allowed");

        let old_weight = self
            .graph
            .edge_weight(src, dest)
            .expect("Node should exist!")
            .unwrap_or(0.0);
        if old_weight == new_weight {
            return;
        }
        let deletion_mode = new_weight.abs() <= EPSILON;
        let step_recalc_probability: Option<f64> =
            (OPTIMIZE_INVALIDATION && !deletion_mode).then(|| {
                new_weight.abs()
                    / (self.graph.get_node_data(src).unwrap().abs_sum() + new_weight.abs())
            });

        if deletion_mode {
            self.graph.remove_edge(src, dest).unwrap();
        } else {
            self.graph.set_edge(src, dest, new_weight).unwrap();
        }

        let affected_walkids =
            self.walks
                .find_affected_walkids(src, Some(dest), step_recalc_probability);

        for (walk_id, visit_pos) in &affected_walkids {
            // Revert the counters associated with the affected walks, as if the walks never existed
            let walk = self.walks.get_walk(*walk_id).unwrap();
            let ego = walk.first_node().unwrap();
            self.pos_hits
                .entry(ego)
                .or_default()
                .decrement_unique_counts(walk.positive_subsegment());
            self.neg_hits
                .entry(ego)
                .or_default()
                .decrement_unique_counts(walk.negative_subsegment());

            let cut_position = visit_pos + 1;
            self.walks
                .split_and_remove_from_bookkeeping(walk_id, cut_position);

            let walk = self.walks.get_walk_mut(*walk_id).unwrap();

            let mut skip_continuation = false;
            //#[cfg(optimize_invalidation)]
            if OPTIMIZE_INVALIDATION {
                if deletion_mode {
                    self.graph.extend_walk_in_case_of_edge_deletion(walk);
                } else if random::<f64>() < self.alpha {
                    walk.push(dest, new_weight > 0.0);
                } else {
                    skip_continuation = true;
                }
            }
            if !skip_continuation {
                self.graph.continue_walk(walk, self.alpha);
            }

            // Update counters associated with the updated walks
            self.pos_hits
                .entry(ego)
                .or_default()
                .increment_unique_counts(walk.positive_subsegment());
            self.neg_hits
                .entry(ego)
                .or_default()
                .increment_unique_counts(walk.negative_subsegment());

            self.walks.update_walk_bookkeeping(*walk_id, cut_position);
        }

        if ASSERT {
            self.walks.assert_visits_consistency();
            self.assert_counters_consistency_after_edge_addition();
        }
    }

    fn assert_counters_consistency_after_edge_addition(&self) {
        for (ego, hits) in &self.pos_hits {
            for (peer, count) in hits {
                let visits = self.walks.get_visits_through_node(*peer).unwrap();
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

                assert_eq!(walks.len(), *count as usize);
                //assert!(*count == 0.0 || weight <= EPSILON || self.graph.is_connecting(*ego, *peer));
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

                assert_eq!(walks.len(), *count as usize);
                //assert!(*count == 0.0 || weight <= EPSILON || self.graph.is_connecting(*ego, *peer));
            }
        }
    }

    pub fn print_walks(&self) {
        self.walks.print_walks();
    }

    pub fn get_personal_hits(&self) -> &IntMap<NodeId, Counter> {
        &self.pos_hits
    }
}
