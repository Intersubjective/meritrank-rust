use rand::distributions::WeightedIndex;
use rand::prelude::*;
use integer_hasher::IntMap;
use tinyset::SetUsize;
use crate::constants::{EPSILON, ASSERT, OPTIMIZE_INVALIDATION};
use crate::common::sign;
use crate::errors::MeritRankError;
use crate::graph::{Graph, NodeId, Weight};
use crate::random_walk::RandomWalk;
use crate::walk_storage::{WalkId, WalkStorage};
use crate::counter::Counter;

#[derive(PartialEq, Eq)]
pub enum Neighbors {
    All,
    Positive,
    Negative,
}

pub struct MeritRank<NodeData: Copy + Default> {
    pub graph: Graph<NodeData>,
    walks: WalkStorage,
    personal_hits: IntMap<NodeId, Counter>,
    neg_hits: IntMap<NodeId, IntMap<NodeId, Weight>>,
    pub alpha: Weight,
}

impl<NodeData: Copy + Default> MeritRank<NodeData> {
    pub fn new(graph: Graph<NodeData>) -> Result<Self, MeritRankError> {
        graph.check_self_reference()?;
        Ok(Self {
            graph,
            walks: WalkStorage::new(),
            personal_hits: IntMap::default(),
            neg_hits: IntMap::default(),
            alpha: 0.85,
        })
    }

    pub fn neighbors_weighted(&self, node: NodeId, mode: Neighbors) -> Option<IntMap<NodeId, Weight>> {
        let neighbors: IntMap<_, _> = self.graph
            .neighbors(node)
            .into_iter()
            .filter_map(|nbr| {
                let weight = self.graph.edge_weight(node, nbr)?;
                match mode {
                    Neighbors::All => Some((nbr, weight)),
                    Neighbors::Positive if weight > 0.0 => Some((nbr, weight)),
                    Neighbors::Negative if weight < 0.0 => Some((nbr, weight)),
                    _ => None,
                }
            })
            .collect();

        (!neighbors.is_empty()).then_some(neighbors)
    }

    pub fn calculate(&mut self, ego: NodeId, num_walks: usize) -> Result<(), MeritRankError> {
        if !self.graph.contains_node(ego) {
            return Err(MeritRankError::NodeDoesNotExist);
        }

        self.walks.drop_walks_from_node(ego);
        let negs = self.neighbors_weighted(ego, Neighbors::Negative).unwrap_or_default();
        self.personal_hits.insert(ego, Counter::new());

        for _ in 0..num_walks {
            let new_walk_id = self.walks.get_next_free_walkid();
            self.perform_walk(new_walk_id, ego);
            let walk = self.walks.get_walk(new_walk_id).unwrap();

            self.personal_hits.entry(ego)
                .and_modify(|counter| counter.increment_unique_counts(walk.iter().cloned()));

            update_negative_hits(&mut self.neg_hits, walk, &negs, false);
            self.walks.add_walk_to_bookkeeping(new_walk_id, 0);
        }

        Ok(())
    }

    pub fn get_node_score(&self, ego: NodeId, target: NodeId) -> Result<Weight, MeritRankError> {
        let counter = self.personal_hits.get(&ego)
            .ok_or(MeritRankError::NodeIsNotCalculated)?;

        let hits = counter.get_count(&target).copied().unwrap_or(0.0);

        if ASSERT && hits > 0.0 && !self.graph.is_connecting(ego, target) {
            return Err(MeritRankError::NoPathExists);
        }

        let default_int_map = IntMap::default();  // Create a longer-lived binding

        let neg_hits = self.neg_hits.get(&ego).unwrap_or(&default_int_map);
        let hits_penalized = hits + neg_hits.get(&target).copied().unwrap_or(0.0);

        Ok(hits_penalized / counter.total_count())
    }

    pub fn get_ranks(&self, ego: NodeId, limit: Option<usize>) -> Result<Vec<(NodeId, Weight)>, MeritRankError> {
        let counter = self.personal_hits.get(&ego)
            .ok_or(MeritRankError::NodeIsNotCalculated)?;

        let mut peer_scores: Vec<_> = counter.keys().iter()
            .map(|&peer| self.get_node_score(ego, peer).map(|score| (peer, score)))
            .collect::<Result<_, _>>()?;

        peer_scores.sort_unstable_by(|(_, score1), (_, score2)| {
            score2.partial_cmp(score1).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(peer_scores.clone().into_iter().take(limit.unwrap_or(peer_scores.len())).collect())
    }

    pub fn perform_walk(&mut self, walk_id: WalkId, start_node: NodeId) {
        let new_segment = self.generate_walk_segment(start_node, false).unwrap();
        let walk = self.walks.get_walk_mut(walk_id).unwrap();
        assert_eq!(walk.len(), 0);
        walk.push(start_node);
        walk.extend(&new_segment);
    }

    pub fn generate_walk_segment(&self, start_node: NodeId, mut skip_alpha: bool) -> Result<Vec<NodeId>, MeritRankError> {
        let mut node = start_node;
        let mut segment = Vec::new();
        let mut rng = thread_rng();

        while let Some(neighbors) = self.neighbors_weighted(node, Neighbors::Positive) {
            if skip_alpha || rng.gen::<f64>() <= self.alpha {
                skip_alpha = false;
                let (peers, weights): (Vec<_>, Vec<_>) = neighbors.iter().unzip();
                let next_step = Self::random_choice(&peers, &weights, &mut rng)
                    .ok_or(MeritRankError::RandomChoiceError)?;
                segment.push(next_step);
                node = next_step;
            } else {
                break;
            }
        }
        Ok(segment)
    }

    fn random_choice<T: Copy>(values: &[T], weights: &[f64], rng: &mut impl Rng) -> Option<T> {
        WeightedIndex::new(weights)
            .ok()
            .and_then(|dist| values.get(dist.sample(rng)).copied())
    }


    pub fn update_penalties_for_edge(&mut self, src: NodeId, dest: NodeId, remove_penalties: bool) {
        let weight = self.graph.edge_weight(src, dest).unwrap();
        let ego_neg_hits = self.neg_hits.entry(src).or_default();
        let neg_weights: IntMap<NodeId, Weight> = [(dest, weight)].iter().cloned().collect();

        // Create a default IntMap to use if get_visits_through_node returns None
        let default_int_map = IntMap::default();

        let affected_walks = self.walks
            .get_visits_through_node(dest)
            .unwrap_or(&default_int_map)
            .iter()
            .filter_map(|(&id, &_)| {
                let walk = self.walks.get_walk(id)?;
                (walk.nodes[0] == src).then_some(walk)
            });

        for walk in affected_walks {
            for (node, penalty) in walk.calculate_penalties(&neg_weights) {
                let adjusted_penalty = if remove_penalties { -penalty } else { penalty };
                *ego_neg_hits.entry(node).or_default() += adjusted_penalty;
            }
        }
    }


    pub fn recalc_invalidated_walk(
        &mut self,
        walk_id: &WalkId,
        force_first_step: Option<NodeId>,
        mut skip_alpha: bool,
    ) -> Result<(), MeritRankError> {
        // Borrow mutable `walk` from `self.walks`
        let walk = self.walks.get_walk_mut(*walk_id).ok_or(MeritRankError::WalkNotFound)?;
        let new_segment_start = walk.len();
        let first_step = force_first_step.unwrap_or_else(|| walk.last_node().unwrap());

        if force_first_step.is_some() {
            if skip_alpha {
                skip_alpha = false;
            } else if random::<f64>() >= self.alpha {
                return Ok(());
            }
        }

        // Borrow `self` immutably for `generate_walk_segment`
        let mut new_segment = self.generate_walk_segment(first_step, skip_alpha)?;

        if let Some(force_first_step) = force_first_step {
            new_segment.insert(0, force_first_step);
        }

        // Borrow `self` immutably for `ego`
        let walk = self.walks.get_walk(*walk_id).ok_or(MeritRankError::WalkNotFound)?;
        let ego = walk.first_node().ok_or(MeritRankError::InvalidWalkLength)?;

        let counter = self.personal_hits.entry(ego).or_insert_with(Counter::new);
        let diff = SetUsize::from_iter(new_segment.iter().cloned()) - &SetUsize::from_iter(walk.get_nodes().iter().cloned());
        counter.increment_unique_counts(diff.iter());

        // Borrow mutable `walk` again for `extend`
        let walk = self.walks.get_walk_mut(*walk_id).ok_or(MeritRankError::WalkNotFound)?;
        walk.extend(&new_segment);
        self.walks.add_walk_to_bookkeeping(*walk_id, new_segment_start);

        Ok(())
    }


    pub fn add_node(&mut self, node: NodeId, data: NodeData) {
        self.graph.add_node(node, data);
    }

    pub fn add_edge(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        assert_ne!(src, dest, "Self reference not allowed");

        let old_weight = self.graph.edge_weight(src, dest).unwrap_or(0.0);
        if old_weight == weight {
            return;
        }

        match (sign(old_weight), sign(weight)) {
            (0, 1) => self.zp(src, dest, weight),
            (0, -1) => self.zn(src, dest, weight),
            (1, 0) => self.pz(src, dest, weight),
            (1, 1) => self.pp(src, dest, weight),
            (1, -1) => self.pn(src, dest, weight),
            (-1, 0) => self.nz(src, dest, weight),
            (-1, 1) => self.np(src, dest, weight),
            (-1, -1) => self.nn(src, dest, weight),
            _ => {}
        }
    }

    fn zp(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        assert!(weight >= 0.0);

        let step_recalc_probability = if OPTIMIZE_INVALIDATION && weight > EPSILON && self.graph.contains_node(src) {
            let sum_of_weights: f64 = self.neighbors_weighted(src, Neighbors::Positive)
                .unwrap_or_default()
                .values()
                .sum();
            weight / (sum_of_weights + weight)
        } else {
            0.0
        };

        let invalidated_walks_ids = self.walks.invalidate_walks_through_node(src, Some(dest), step_recalc_probability);
        let mut negs_cache: IntMap<NodeId, IntMap<NodeId, f64>> = IntMap::default();

        for (uid, visit_pos) in &invalidated_walks_ids {
            let walk = self.walks.get_walk(*uid).unwrap();
            let negs = negs_cache
                .entry(walk.first_node().unwrap())
                .or_insert_with(|| self.neighbors_weighted(walk.first_node().unwrap(), Neighbors::Negative).unwrap_or_default());

            let cut_position = *visit_pos + 1;
            revert_counters_for_walk_from_pos(&mut self.personal_hits, walk, cut_position);

            if !negs.is_empty() {
                update_negative_hits(&mut self.neg_hits, walk, negs, true);
            }
        }

        if weight <= EPSILON {
            self.graph.remove_edge(src, dest);
        } else {
            self.graph.add_edge(src, dest, weight);
        }

        for (walk_id, visit_pos) in &invalidated_walks_ids {
            let cut_position = visit_pos + 1;
            self.walks.remove_walk_segment_from_bookkeeping(walk_id, cut_position);
            let force_first_step = (step_recalc_probability > 0.0).then_some(dest);

            let _ = self.recalc_invalidated_walk(walk_id, force_first_step, OPTIMIZE_INVALIDATION && weight <= EPSILON);
            let walk_updated = self.walks.get_walk(*walk_id).unwrap();
            let first_node = walk_updated.first_node().unwrap();

            if let Some(negs) = negs_cache.get(&first_node) {
                if !negs.is_empty() {
                    update_negative_hits(&mut self.neg_hits, walk_updated, negs, false);
                }
            }
        }

        if ASSERT {
            self.walks.assert_visits_consistency();
            self.assert_counters_consistency_after_edge_addition(weight);
        }
    }

    fn assert_counters_consistency_after_edge_addition(&self, weight: f64) {
        for (ego, hits) in &self.personal_hits {
            for (peer, count) in hits {
                let visits = self.walks.get_visits_through_node(*peer).unwrap();
                let walks: Vec<_> = visits.iter()
                    .filter(|&(walkid, _)| self.walks.get_walk(*walkid).unwrap().get_nodes().first() == Some(ego))
                    .collect();

                assert_eq!(walks.len(), *count as usize);
                assert!(*count == 0.0 || weight <= EPSILON || self.graph.is_connecting(*ego, *peer));
            }
        }
    }

    fn zn(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        self.graph.add_edge(src, dest, weight);
        self.update_penalties_for_edge(src, dest, false);
    }

    fn pz(&mut self, src: NodeId, dest: NodeId, _weight: f64) {
        self.zp(src, dest, 0.0);
    }

    fn pp(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        self.zp(src, dest, weight);
    }

    fn pn(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        self.pz(src, dest, weight);
        self.zn(src, dest, weight);
    }

    fn nz(&mut self, src: NodeId, dest: NodeId, _weight: f64) {
        self.update_penalties_for_edge(src, dest, true);
        self.graph.remove_edge(src, dest);
    }
    fn np(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        self.nz(src, dest, weight);
        self.zp(src, dest, weight);
    }

    fn nn(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        self.nz(src, dest, weight);
        self.zn(src, dest, weight);
    }

    pub fn get_edge(&self, src: NodeId, dest: NodeId) -> Option<Weight> {
        self.graph.edge_weight(src, dest)
    }

    pub fn get_node_data(&self, ego: NodeId) -> Result<NodeData, MeritRankError> {
        self.graph.get_node_info(ego)
            .map(|(_, data)| data)
            .ok_or(MeritRankError::NodeDoesNotExist)
    }

    pub fn print_walks(&self) {
        self.walks.print_walks();
    }

    pub fn get_personal_hits(&self) -> &IntMap<NodeId, Counter> {
        &self.personal_hits
    }
}

fn update_negative_hits(
    neg_hits: &mut IntMap<NodeId, IntMap<NodeId, Weight>>,
    walk: &RandomWalk,
    negs: &IntMap<NodeId, Weight>,
    subtract: bool,
) {
    if walk.intersects_nodes(negs.keys()) {
        let ego_neg_hits = neg_hits
            .entry(walk.first_node().unwrap())
            .or_default();

        for (node, penalty) in walk.calculate_penalties(negs) {
            let adjusted_penalty = if subtract { -penalty } else { penalty };
            *ego_neg_hits.entry(node).or_default() += adjusted_penalty;
        }
    }
}

fn revert_counters_for_walk_from_pos(
    personal_hits: &mut IntMap<NodeId, Counter>,
    walk: &RandomWalk,
    pos: usize,
) {
    let ego = walk.first_node().unwrap();
    let counter = personal_hits.entry(ego).or_insert_with(Counter::new);

    let nodes_before_pos: SetUsize = walk.get_nodes()[..pos].iter().cloned().collect();
    let nodes_to_remove: SetUsize = walk.get_nodes()[pos..]
        .iter()
        .cloned()
        .filter(|&node| !nodes_before_pos.contains(node))
        .collect();

    if !nodes_to_remove.is_empty() {
        for node_to_remove in nodes_to_remove {
            *counter.get_mut_count(&node_to_remove) -= 1.0;
        }

        #[cfg(debug_assertions)]
        for &c in counter.count_values() {
            assert!(c >= 0.0);
        }
    }
}
