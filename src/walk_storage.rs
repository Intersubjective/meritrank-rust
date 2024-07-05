use indexmap::IndexMap;

use rand::prelude::*;

use crate::constants::{ASSERT, OPTIMIZE_INVALIDATION};
use crate::graph::{NodeId, EdgeId, Weight};
use crate::random_walk::RandomWalk;

pub type WalkId = usize;

/// Represents a storage container for walks in the MeritRank graph.
pub struct WalkStorage {
    visits: IndexMap<NodeId, IndexMap<WalkId, usize>>,
    walks: Vec<RandomWalk>,
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
            visits: IndexMap::new(),
            walks: Vec::new(),
        }
    }

    pub fn get_walk(&self, uid: WalkId) -> Option<&RandomWalk>
    {
        self.walks.get(uid)
    }

    pub fn get_walk_mut(&mut self, uid: WalkId) -> Option<&mut RandomWalk>
    {
        self.walks.get_mut(uid)
    }

    pub fn get_walks(&self) -> &IndexMap<NodeId, IndexMap<WalkId, usize>> {
        &self.visits
    }

    pub fn get_visits_through_node(&self, node_id: NodeId) -> Option<&IndexMap<WalkId, usize>> {
        self.visits.get(&node_id)
    }




    /// Adds a walk to the storage.
    ///
    /// This method adds a `RandomWalk` object to the storage.
    ///
    /// # Arguments
    ///
    /// * `walk` - The `RandomWalk` object to be added.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{WalkStorage, RandomWalk};
    ///
    /// let mut storage = WalkStorage::new();
    /// let walk = RandomWalk::new();
    /// storage.add_walk(walk);
    /// ```
    pub fn add_walk(&mut self, walk: RandomWalk) -> WalkId {
        // skip the first start_pos nodes
        let new_walk_id = self.walks.len() as WalkId;
        self.walks.push(walk);
        new_walk_id
    }

    pub fn add_walk_to_bookkeeping(&mut self, walk_id: WalkId, start_pos: usize) {
         let walk = &self.walks[walk_id as usize];
        for (pos, &node) in walk.get_nodes().iter().enumerate().skip(start_pos) {
            // add the walk to the node
            let walks_with_node = self.visits.entry(node).or_insert_with(IndexMap::new);
            if !walks_with_node.contains_key(&walk_id) {
                walks_with_node.insert(walk_id, pos);
            }
        }
    }

    pub fn print_walks(&self) {
        for walk in &self.walks{
            println! ("{:?}", *walk);
        }

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
    /// use meritrank::{WalkStorage, NodeId, MeritRankError, Graph};
    ///
    /// let mut storage = WalkStorage::new();
    ///
    /// let node = 1;
    /// storage.drop_walks_from_node(node);
    /// ```
    ///
    pub fn drop_walks_from_node(&mut self, node: NodeId) {
        // Check if there are any visits for the given node
        if let Some(visits_for_node) = self.visits.get_mut(&node) {
            /// TODO: add cleaning matching .walks also
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
                        if let Some(visits) = self.visits.get_mut(node) {
                            visits.remove(&walk_id);
                        }
                    }
                }
            }
            // Remove any empty entries from the visits HashMap
            self.visits.retain(|_, visits_ref| !visits_ref.is_empty());
        }

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
    /// let walk = RandomWalk::from_nodes(vec![ 1, 2, 3, ]);
    /// let pos = 0;
    /// let edge: EdgeId = (1, 2);
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
    /// let walk = RandomWalk::from_nodes(vec![ 1, 2, 3, ]);
    /// let pos = 0;
    /// let edge: EdgeId = (1, 2);
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
    /// let walk = RandomWalk::from_nodes(vec![ 1, 2, ]);
    /// let pos = 0;
    /// let edge: EdgeId = (1, 2);
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
        //   .as_mut()
        //   .unwrap_or_else(|| &mut rand::thread_rng());

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

    pub fn assert_visits_consistency(&self)
    {
        for (node, visits) in self.visits.iter(){
            for (walkid,pos) in  visits.iter(){
               assert_eq!(self.walks[*walkid].nodes[*pos], *node);
            }

        }
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
    /// use meritrank::{WalkStorage, NodeId, MeritRankError, Graph};
    ///
    /// let mut storage = WalkStorage::new();
    ///
    /// let invalidated_node = 1;
    /// let dst_node = Some(2);
    /// let step_recalc_probability = 0.0;
    ///
    /// let invalidated_walks = storage.invalidate_walks_through_node(
    ///   invalidated_node,
    ///   dst_node,
    ///   step_recalc_probability,
    /// );
    ///
    /// for (walk, invalidated_segment) in invalidated_walks {
    ///   println!("Invalidated walk: {:?}", walk);
    ///   println!("Invalidated segment: {:?}", invalidated_segment);
    /// }
    /// ```
    pub fn invalidate_walks_through_node(
        &mut self,
        invalidated_node: NodeId,
        dst_node: Option<NodeId>,
        step_recalc_probability: Weight,
    ) -> Vec<(WalkId, usize)> {
        let mut invalidated_walks_ids = vec![];

        // Check if there are any walks passing through the invalidated node
        let walks= match self.visits.get(&invalidated_node) {
            Some(walks) => walks,
            None => return invalidated_walks_ids,
        };

        for (walk_id, visit_pos) in walks.iter() {

            let mut _new_pos = *visit_pos;
            let mut may_skip = false;
            // Optimize invalidation by skipping if possible
            if OPTIMIZE_INVALIDATION && dst_node.is_some() {
                let mut rng = thread_rng();
                (may_skip, _new_pos) = self.decide_skip_invalidation(
                    self.get_walk(*walk_id).unwrap(),
                    *visit_pos,
                    (invalidated_node, dst_node.unwrap()),
                    step_recalc_probability,
                    Some(&mut rng),
                );
                if may_skip {
                    // Skip invalidating this walk if it is determined to be unnecessary
                    continue;
                }
            }

            invalidated_walks_ids.push((*walk_id, _new_pos));
        }

        invalidated_walks_ids
    }

    pub fn remove_walk_segment_from_bookkeeping(&mut self, walk_id: &WalkId, cut_pos: usize) {
        // Cut position is the index of the first element of the invalidated segment
        // Split the walk and obtain the invalidated segment
        let walk = self.walks.get_mut(*walk_id).unwrap();
        let invalidated_segment = walk.split_from(cut_pos);

        // Remove affected nodes from bookkeeping, but ensure we don't accidentally remove references
        // if there are still copies of the affected node in the remaining walk
        for &affected_node in invalidated_segment
            .get_nodes()
            .iter()
            .filter(|&node| !walk.contains(node))
        {
            if let Some(affected_walks) = self.visits.get_mut(&affected_node) {
                if affected_walks.get(walk_id).is_some() {
                    // Remove the invalidated walk from affected nodes
                    affected_walks.remove(walk_id);
                }
            }
        }
    }
}
