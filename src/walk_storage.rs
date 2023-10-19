use indexmap::IndexMap;
use std::collections::HashSet;

use rand::prelude::*;

use crate::constants::{ASSERT, OPTIMIZE_INVALIDATION};
use crate::edge::EdgeId;
use crate::node::{NodeId, Weight};
use crate::poswalk::PosWalk;
use crate::random_walk::RandomWalk;
use crate::walk::WalkId;

/// Represents a storage container for walks in the MeritRank graph.
pub struct WalkStorage {
    walks: IndexMap<NodeId, IndexMap<WalkId, PosWalk>>,
}

impl WalkStorage {
    /// Creates a new instance of WalkStorage.
    ///
    /// This method creates a new instance of `WalkStorage` with an empty collection of walks.
    ///
    /// # Returns
    ///
    /// A new `WalkStorage` instance.
    pub fn new() -> Self {
        WalkStorage {
            walks: IndexMap::new(),
        }
    }

    pub fn get_walks(&self) -> &IndexMap<NodeId, IndexMap<WalkId, PosWalk>> {
        &self.walks
    }

    /// Adds a walk to the storage.
    ///
    /// This method adds a `RandomWalk` object to the storage, starting from the specified `start_pos`.
    ///
    /// # Arguments
    ///
    /// * `walk` - The `RandomWalk` object to be added.
    /// * `start_pos` - The starting position to add the walk from.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk};
    ///
    /// let mut storage = WalkStorage::new();
    /// let walk = RandomWalk::new();
    /// let start_pos = 0;
    /// storage.add_walk(walk, start_pos);
    /// ```
    pub fn add_walk(&mut self, walk: RandomWalk, start_pos: usize) {
        // skip the first start_pos nodes
        for (pos, &node) in walk.get_nodes().iter().enumerate().skip(start_pos) {
            // add the walk to the node
            let walks_with_node = self.walks.entry(node).or_insert_with(IndexMap::new);
            if !walks_with_node.contains_key(&walk.get_walk_id()) {
                walks_with_node.insert(walk.get_walk_id(), PosWalk::new(walk.clone(), pos));
            }
        }
    }

    /// Implements the changes in the WalkStorage based on the invalidated walks.
    ///
    /// This method updates the WalkStorage based on the invalidated walks. It removes the invalidated
    /// segments from the existing walks and adds new walks for the updated segments.
    ///
    /// # Arguments
    ///
    /// * `invalidated_walks` - The vector of invalidated walks
    pub fn implement_changes(&mut self, invalidated_walks: Vec<(RandomWalk, RandomWalk)>) {
        for (updated_walk, invalidated_segment) in invalidated_walks {
            let updated_walk_id = updated_walk.get_walk_id();

            let mut nodes_to_update: HashSet<NodeId> = HashSet::new();
            nodes_to_update.extend(updated_walk.get_nodes().iter().copied());
            nodes_to_update.extend(invalidated_segment.iter().copied());

            for node in nodes_to_update {
                let updated_walk_present = updated_walk.contains(&node);
                let invalidated_segment_present = invalidated_segment.contains(&node);

                if updated_walk_present || invalidated_segment_present {
                    if let Some(pos_walks) = self.walks.get_mut(&node) {
                        if updated_walk_present {
                            // Update existing PosWalk entry
                            let start_pos = updated_walk
                                .get_nodes()
                                .iter()
                                .position(|&n| n == node)
                                .unwrap();
                            let updated_pos_walk = PosWalk::new(updated_walk.clone(), start_pos);
                            pos_walks.insert(updated_walk_id, updated_pos_walk);
                        } else {
                            // Remove invalidated PosWalk entry
                            pos_walks.remove(&updated_walk_id);
                        }
                    } else if updated_walk_present {
                        // Add new PosWalk entry
                        let start_pos = updated_walk
                            .get_nodes()
                            .iter()
                            .position(|&n| n == node)
                            .unwrap();
                        let new_pos_walk = PosWalk::new(updated_walk.clone(), start_pos);

                        let mut pos_walks = IndexMap::new();
                        pos_walks.insert(updated_walk_id, new_pos_walk);
                        self.walks.insert(node, pos_walks);
                    }
                }
            }

            // Retain only the nodes that are still present in the storage
            self.walks.retain(|_, pos_walks| !pos_walks.is_empty());
        }
    }

    /// Updates the WalkStorage based on a new RandomWalk and an optional old RandomWalk.
    ///
    /// This method updates the WalkStorage by removing invalidated segments from the existing walks
    /// and adding new walks for the updated segments.
    ///
    /// # Arguments
    ///
    /// * `new_walk` - The new RandomWalk to be updated or added.
    /// * `old_walk` - An optional old RandomWalk to be updated or removed.
    pub fn update_walk(&mut self, new_walk: RandomWalk, old_walk: Option<RandomWalk>) {
        let old_walk_id = old_walk.as_ref().map(|w| w.get_walk_id());
        let new_walk_id = new_walk.get_walk_id();

        // Step 1: Collect nodes to update
        let old_nodes: HashSet<NodeId> = old_walk
            .as_ref()
            .map(|w| w.get_nodes().iter().copied().collect())
            .unwrap_or_else(HashSet::new);
        let new_nodes: HashSet<NodeId> = new_walk.get_nodes().iter().copied().collect();
        let nodes_to_update: HashSet<NodeId> = old_nodes.union(&new_nodes).cloned().collect();

        // Step 2: Process updates for each node
        for node in nodes_to_update {
            let walks_with_node = self.walks.entry(node).or_insert_with(IndexMap::new);

            match (old_walk_id, new_nodes.contains(&node)) {
                (Some(old_id), true) => {
                    // Update existing walk
                    if let Some(pos_walk) = walks_with_node.remove(&old_id) {
                        let mut updated_pos_walk = pos_walk;
                        updated_pos_walk.set_walk(new_walk.clone());
                        updated_pos_walk.set_pos(
                            new_walk
                                .get_nodes()
                                .iter()
                                .position(|&n| n == node)
                                .unwrap(),
                        );
                        walks_with_node.insert(new_walk_id.clone(), updated_pos_walk);
                    } else {
                        // Insert new walk if it doesn't exist
                        let start_pos = new_walk
                            .get_nodes()
                            .iter()
                            .position(|&n| n == node)
                            .unwrap();
                        walks_with_node.insert(
                            new_walk_id.clone(),
                            PosWalk::new(new_walk.clone(), start_pos),
                        );
                    }
                }
                (Some(old_id), false) => {
                    // Remove walk if not in new nodes
                    walks_with_node.remove(&old_id);
                }
                (None, true) => {
                    // Add new walk or update existing walk
                    if walks_with_node.contains_key(&new_walk_id) {
                        if let Some(pos_walk) = walks_with_node.get_mut(&new_walk_id) {
                            pos_walk.set_walk(new_walk.clone());
                            pos_walk.set_pos(
                                new_walk
                                    .get_nodes()
                                    .iter()
                                    .position(|&n| n == node)
                                    .unwrap(),
                            );
                        }
                    } else {
                        let start_pos = new_walk
                            .get_nodes()
                            .iter()
                            .position(|&n| n == node)
                            .unwrap();
                        walks_with_node.insert(
                            new_walk_id.clone(),
                            PosWalk::new(new_walk.clone(), start_pos),
                        );
                    }
                }
                (None, false) => {
                    // Remove walk if not in new nodes
                    walks_with_node.remove(&new_walk_id);
                }
            }
        }

        // Step 3: Remove empty nodes
        self.walks.retain(|_, v| !v.is_empty());
    }

    /// Retrieves the walks associated with a given node.
    ///
    /// This method returns the walks associated with the specified node.
    ///
    /// # Arguments
    ///
    /// * `node` - The node identifier.
    ///
    /// # Returns
    ///
    /// An option containing a reference to the walks associated with the node. Returns `None` if no walks are found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, NodeId};
    ///
    /// let storage = WalkStorage::new();
    /// let node = NodeId::UInt(1);
    /// let walks = storage._get_walks_throw_node(node);
    /// ```
    pub fn _get_walks_throw_node(&self, node: NodeId) -> Option<&IndexMap<WalkId, PosWalk>> {
        self.walks.get(&node)
    }

    /// Retrieves the walks starting from the given node.
    ///
    /// This method retrieves the walks that start from the specified `src` node.
    ///
    /// # Arguments
    ///
    /// * `src` - The starting node for the walks.
    ///
    /// # Returns
    ///
    /// A vector of `RandomWalk` objects that start from the given node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, NodeId};
    ///
    /// let storage = WalkStorage::new();
    /// let src_node = NodeId::UInt(1);
    /// let walks = storage._get_walks_starting_from_node(src_node);
    /// ```
    pub fn _get_walks_starting_from_node(&self, src: NodeId) -> Vec<RandomWalk> {
        self.walks
            .get(&src)
            .map(|pos_walks| {
                pos_walks
                    .values()
                    .filter(|pos_walk| pos_walk.get_pos() == 0)
                    .map(|pos_walk| pos_walk.get_walk().clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Drops all walks starting from the specified node.
    ///
    /// This method removes the walks starting from the given `node` and removes any references to
    /// these walks from other nodes in the bookkeeping.
    ///
    /// # Arguments
    ///
    /// * `node` - The node from which the walks should be dropped.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, NodeId, MeritRankError, MyGraph};
    ///
    /// let mut storage = WalkStorage::new();
    ///
    /// let node = NodeId::UInt(1);
    /// storage.drop_walks_from_node(node);
    /// ```
    pub fn drop_walks_from_node(&mut self, node: NodeId) {
        for (_, pos_walks) in &mut self.walks {
            pos_walks.retain(|_, pos_walk| pos_walk.get_walk().first_node().unwrap() != node);
        }

        if let Some(pos_walks) = self.walks.get_mut(&node) {
            pos_walks.clear();
        }

        self.walks.retain(|_, pos_walks| !pos_walks.is_empty());
    }

    /// Returns the walks passing through the specified node based on a given filter.
    ///
    /// This method retrieves the walks passing through the given `node` that satisfy the provided `filter`.
    ///
    /// # Arguments
    ///
    /// * `node` - The node for which to retrieve the walks.
    /// * `filter` - A closure that takes a reference to a `PosWalk` and returns a boolean value indicating whether the walk should be included.
    ///
    /// # Returns
    ///
    /// A vector of `RandomWalk` objects passing through the specified `node` and satisfying the provided `filter`, or an empty vector if no walks
    /// exist for the node or if no walks satisfy the filter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, NodeId, MeritRankError, MyGraph};
    ///
    /// let storage = WalkStorage::new();
    ///
    /// let node = NodeId::UInt(1);
    ///
    /// // Example filter: only include walks with a certain length
    /// let walks = storage.get_walks_through_node(node, |pos_walk| pos_walk.get_walk().len() > 5);
    ///
    /// // Example filter: only include walks with a specific starting node
    /// let walks = storage.get_walks_through_node(node, |pos_walk| pos_walk.get_current_node() == NodeId::Int(2));
    /// ```
    pub fn get_walks_through_node<F>(&self, node: NodeId, filter: F) -> Vec<RandomWalk>
        where
            F: Fn(&PosWalk) -> bool,
    {
        self.walks
            .get(&node)
            .map(|pos_walks| {
                pos_walks
                    .values()
                    .filter(|pos_walk| filter(pos_walk))
                    .map(|pos_walk| pos_walk.get_walk())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Determines whether to skip invalidation of a walk based on the edge change.
    ///
    /// This method decides whether to skip the invalidation of a walk based on the edge change. It uses the provided `step_recalc_probability` to determine the probability of skipping the invalidation.
    ///
    /// # Arguments
    ///
    /// * `walk` - The walk to be considered for invalidation.
    /// * `pos` - The position in the walk to be considered for invalidation.
    /// * `edge` - The edge that has changed.
    /// * `step_recalc_probability` - The probability of recalculating the step.
    ///
    /// # Returns
    ///
    /// A tuple containing a boolean value indicating whether to skip invalidation and the updated position in the walk.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, EdgeId, NodeId, Weight};
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    ///
    /// let storage = WalkStorage::new();
    /// let walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3)]);
    /// let pos = 0;
    /// let edge: EdgeId = (NodeId::UInt(1), NodeId::UInt(2));
    /// let step_recalc_probability = 0.5;
    /// // Create a deterministic random number generator
    /// let rng_seed = 1234;
    /// let mut rng = StdRng::seed_from_u64(rng_seed);
    /// let (may_skip, new_pos) = storage.decide_skip_invalidation(&walk, pos, edge, step_recalc_probability, Some(&mut rng));
    /// ```
    pub fn decide_skip_invalidation<R>(
        &self,
        walk: &RandomWalk,
        pos: usize,
        edge: EdgeId,
        step_recalc_probability: Weight,
        rnd: Option<R>,
    ) -> (bool, usize)
        where
            R: RngCore,
    {
        if step_recalc_probability == 0.0 {
            self.decide_skip_invalidation_on_edge_deletion(walk, pos, edge)
        } else {
            self.decide_skip_invalidation_on_edge_addition(
                walk,
                pos,
                edge,
                step_recalc_probability,
                rnd,
            )
        }
    }

    /// Determines whether to skip invalidation on edge deletion.
    ///
    /// This method decides whether to skip the invalidation of a walk based on the edge deletion.
    ///
    /// # Arguments
    ///
    /// * `walk` - The walk to be considered for invalidation.
    /// * `pos` - The position in the walk to be considered for invalidation.
    /// * `edge` - The edge that has been deleted.
    ///
    /// # Returns
    ///
    /// A tuple containing a boolean value indicating whether to skip invalidation and the updated position in the walk.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, EdgeId, NodeId};
    ///
    /// let storage = WalkStorage::new();
    /// let walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2), NodeId::UInt(3)]);
    /// let pos = 0;
    /// let edge: EdgeId = (NodeId::UInt(1), NodeId::UInt(2));
    /// let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_deletion(&walk, pos, edge);
    /// ```
    pub fn decide_skip_invalidation_on_edge_deletion(
        &self,
        walk: &RandomWalk,
        pos: usize,
        edge: EdgeId,
    ) -> (bool, usize) {
        assert!(pos < walk.len()); // Assert pos < len(walk)
        let (invalidated_node, dst_node) = edge;

        let mut may_skip = true;
        let mut new_pos = pos;

        if pos == walk.len() - 1 {
            may_skip = true;
        } else {
            for i in pos..walk.len() - 1 {
                if walk.get_nodes()[i] == invalidated_node && walk.get_nodes()[i + 1] == dst_node {
                    new_pos = i;
                    may_skip = false;
                    break;
                }
            }
        }

        (may_skip, new_pos)
    }

    /// Determines whether to skip invalidation on edge addition.
    ///
    /// This method decides whether to skip the invalidation of a walk based on the edge addition.
    ///
    /// # Arguments
    ///
    /// * `walk` - The walk to be considered for invalidation.
    /// * `pos` - The position in the walk to be considered for invalidation.
    /// * `edge` - The edge that has been added.
    /// * `step_recalc_probability` - The probability of recalculating the step.
    ///
    /// # Returns
    ///
    /// A tuple containing a boolean value indicating whether to skip invalidation and the updated position in the walk.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk, EdgeId, Weight, NodeId};
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    ///
    /// let storage = WalkStorage::new();
    /// let walk = RandomWalk::from_nodes(vec![NodeId::UInt(1), NodeId::UInt(2)]);
    /// let pos = 0;
    /// let edge: EdgeId = (NodeId::UInt(1), NodeId::UInt(2));
    /// let step_recalc_probability = 0.5;
    /// // Create a deterministic random number generator
    /// let rng_seed = 1234;
    /// let mut rng = StdRng::seed_from_u64(rng_seed);
    /// let (may_skip, new_pos) = storage.decide_skip_invalidation_on_edge_addition(&walk, pos, edge, step_recalc_probability, Some(&mut rng));
    /// ```
    pub fn decide_skip_invalidation_on_edge_addition<R>(
        &self,
        walk: &RandomWalk,
        pos: usize,
        edge: EdgeId,
        step_recalc_probability: Weight,
        mut rnd: Option<R>,
    ) -> (bool, usize)
        where
            R: RngCore,
    {
        assert!(pos < walk.len()); // Assert pos < len(walk)
        let (invalidated_node, _dst_node) = edge;

        // Use the provided generator or fall back to thread_rng()
        let mut binding = rand::thread_rng();
        let rng: &mut dyn RngCore = match rnd {
            Some(ref mut r) => r,
            None => &mut binding,
        };

        // let mut rng: &mut dyn RngCore = rnd
        //     .as_mut()
        //     .unwrap_or_else(|| &mut rand::thread_rng());

        let mut may_skip = true;
        let mut new_pos = pos;

        for i in pos..walk.len() {
            if walk.get_nodes()[i] == invalidated_node {
                new_pos = i;
                if rng.gen::<Weight>() < step_recalc_probability {
                    may_skip = false;
                    break;
                }
            }
        }

        (may_skip, new_pos)
    }

    /// Invalidates walks passing through a specific node and returns the invalidated walks.
    ///
    /// The walks that pass through the `invalidated_node` are modified by removing the subsequence
    /// starting from the `pos + 1` position. The affected walks are tracked, and any references to
    /// nodes in the invalidated subsequence are removed from the bookkeeping.
    ///
    /// # Arguments
    ///
    /// * `invalidated_node` - The node through which the walks should be invalidated.
    /// * `dst_node` - The destination node (optional) used for optimization purposes.
    /// * `step_recalc_probability` - The probability of recalculating the step (optional).
    ///
    /// # Returns
    ///
    /// * `Vec<(RandomWalk, RandomWalk)>` - A vector of invalidated walks, each represented by a tuple
    ///   containing the original walk and the invalidated segment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, NodeId, MeritRankError, MyGraph};
    ///
    /// let mut storage = WalkStorage::new();
    ///
    /// let invalidated_node = NodeId::UInt(1);
    /// let dst_node = Some(NodeId::UInt(2));
    /// let step_recalc_probability = 0.0;
    ///
    /// let invalidated_walks = storage.invalidate_walks_through_node(
    ///     invalidated_node,
    ///     dst_node,
    ///     step_recalc_probability,
    /// );
    ///
    /// for (walk, invalidated_segment) in invalidated_walks {
    ///     println!("Invalidated walk: {:?}", walk);
    ///     println!("Invalidated segment: {:?}", invalidated_segment);
    /// }
    /// ```
    pub fn invalidate_walks_through_node(
        &mut self,
        invalidated_node: NodeId,
        dst_node: Option<NodeId>,
        step_recalc_probability: Weight,
    ) -> Vec<(RandomWalk, RandomWalk)> {
        let mut invalidated_walks = vec![];

        // Check if there are any walks passing through the invalidated node
        let mut walks: IndexMap<WalkId, PosWalk> = match self.walks.remove(&invalidated_node) {
            Some(walks) => walks,
            None => return invalidated_walks,
        };

        for (uid, pos_walk) in &mut walks {
            let pos = pos_walk.get_pos();

            if ASSERT {
                assert_eq!(uid, &pos_walk.get_walk().get_walk_id());
            }

            // Optimize invalidation by skipping if possible
            if OPTIMIZE_INVALIDATION && dst_node.is_some() {
                let mut rng = thread_rng();
                let (may_skip, _new_pos) = self.decide_skip_invalidation(
                    pos_walk.get_walk(),
                    pos,
                    (invalidated_node, dst_node.unwrap()),
                    step_recalc_probability,
                    Some(&mut rng),
                );
                if may_skip {
                    // Skip invalidating this walk if it is determined to be unnecessary
                    continue;
                }
            }

            // Split the walk and obtain the invalidated segment
            let invalidated_segment = pos_walk.get_walk_mut().split_from(pos + 1);

            // Remove affected nodes from bookkeeping, but ensure we don't accidentally remove references
            // if there are still copies of the affected node in the remaining walk
            for &affected_node in invalidated_segment
                .get_nodes()
                .iter()
                .filter(|&node| !pos_walk.get_walk().contains(node))
            {
                if let Some(affected_walks) = self.walks.get_mut(&affected_node) {
                    if affected_walks.get(uid).is_some() {
                        // Remove the invalidated walk from affected nodes
                        affected_walks.remove(uid);
                    }
                }
            }

            invalidated_walks.push((pos_walk.get_walk().clone(), invalidated_segment));
        }

        // Insert the walks back into self.walks
        self.walks.insert(invalidated_node, walks);

        invalidated_walks
    }
}
