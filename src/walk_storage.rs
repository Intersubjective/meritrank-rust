use std::collections::VecDeque;
use rand::prelude::*;

use integer_hasher::IntMap;

use crate::constants::{OPTIMIZE_INVALIDATION};
use crate::graph::{NodeId, EdgeId, Weight};
use crate::random_walk::RandomWalk;

pub type WalkId = usize;

/// Represents a storage container for walks in the MeritRank graph.
pub struct WalkStorage {
    visits: IntMap<NodeId, IntMap<WalkId, usize>>,
    walks: Vec<RandomWalk>,
    unused_walks: VecDeque<WalkId>,
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
            visits: IntMap::default(),
            walks: Vec::new(),
            unused_walks: VecDeque::new(),

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

    pub fn get_walks(&self) -> &IntMap<NodeId, IntMap<WalkId, usize>> {
        &self.visits
    }

    pub fn get_visits_through_node(&self, node_id: NodeId) -> Option<&IntMap<WalkId, usize>> {
        self.visits.get(&node_id)
    }


    pub fn get_next_free_walkid(&mut self) -> WalkId {
        match self.unused_walks.pop_front(){
            Some(id) => id,
            None => {
                let id = self.walks.len() as WalkId;
                self.walks.push(RandomWalk::new());
                id
            }
        }
    }

    pub fn add_walk_to_bookkeeping(&mut self, walk_id: WalkId, start_pos: usize) {
         let walk = &self.walks[walk_id as usize];
        for (pos, &node) in walk.get_nodes().iter().enumerate().skip(start_pos) {
            // add the walk to the node
            let walks_with_node = self.visits.entry(node).or_insert_with(IntMap::default);
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
                self.unused_walks.push_back(walk_id);
                self.walks.get_mut(walk_id).unwrap().clear();
            }
            // Remove any empty entries from the visits HashMap
            self.visits.retain(|_, visits_ref| !visits_ref.is_empty());
        }

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

        walks.iter().for_each(|(walk_id, visit_pos)| {
            let _new_pos = if OPTIMIZE_INVALIDATION && dst_node.is_some() {
                let mut rng = thread_rng();
                let (may_skip, new_pos) = decide_skip_invalidation(
                    self.get_walk(*walk_id).unwrap(),
                    *visit_pos,
                    (invalidated_node, dst_node.unwrap()),
                    step_recalc_probability,
                    Some(&mut rng),
                );
                if may_skip {
                    // Skip invalidating this walk if it is determined to be unnecessary
                    return;
                }
                new_pos
            } else {
                *visit_pos
            };

            invalidated_walks_ids.push((*walk_id, _new_pos));
        });

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

pub fn decide_skip_invalidation<R>(
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
        decide_skip_invalidation_on_edge_deletion(walk, pos, edge)
    } else {
        decide_skip_invalidation_on_edge_addition(
            walk,
            pos,
            edge,
            step_recalc_probability,
            rnd,
        )
    }
}
pub fn decide_skip_invalidation_on_edge_deletion(
    walk: &RandomWalk,
    pos: usize,
    edge: EdgeId,
) -> (bool, usize) {
    assert!(pos < walk.len());
    let (invalidated_node, dst_node) = edge;

    if pos == walk.len() - 1 {
        return (true, pos);
    }

    walk.get_nodes()[pos..walk.len() - 1]
        .iter()
        .enumerate()
        .find_map(|(i, &node)| {
            if node == invalidated_node && walk.get_nodes()[pos + i + 1] == dst_node {
                Some((false, pos + i))
            } else {
                None
            }
        })
        .unwrap_or((true, pos))
}





pub fn decide_skip_invalidation_on_edge_addition<R>(
    walk: &RandomWalk,
    pos: usize,
    edge: EdgeId,
    step_recalc_probability: Weight,
    mut rnd: Option<R>,
) -> (bool, usize)
where
    R: RngCore,
{
    assert!(pos < walk.len(), "Position must be within walk length");
    let (invalidated_node, _dst_node) = edge;

    let mut thread_rng = thread_rng();
    let rng = rnd.as_mut().map(|r| r as &mut dyn RngCore).unwrap_or(&mut thread_rng);

    let mut new_pos = pos;
    let result = walk.get_nodes()[pos..]
        .iter()
        .enumerate()
        .find_map(|(i, &node)| {
            if node == invalidated_node {
                new_pos = pos + i;
                if rng.gen::<Weight>() < step_recalc_probability {
                    Some(false)  // may_skip = false, exit early
                } else {
                    None  // continue searching
                }
            } else {
                None  // continue searching
            }
        });

    (result.is_none(), new_pos)
}
