use rand::distributions::WeightedIndex;
use rand::prelude::*;

use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

use crate::common::sign;
use crate::constants::{ASSERT, OPTIMIZE_INVALIDATION};
use crate::counter::Counter;
use crate::errors::MeritRankError;
use crate::graph::MyGraph;
use crate::node::{NodeId, Weight};
use crate::poswalk::PosWalk;
use crate::random_walk::RandomWalk;
use crate::walk::WalkId;
use crate::node::Node;
use crate::walk_storage::WalkStorage;

pub struct MeritRank {
    graph: MyGraph,
    walks: WalkStorage,
    personal_hits: HashMap<NodeId, Counter>,
    neg_hits: HashMap<NodeId, HashMap<NodeId, Weight>>,
    alpha: Weight,
}

// #[allow(dead_code)]
impl MeritRank {
    /// Creates a new `MeritRank` instance with the given graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - A `MyGraph` instance representing the underlying graph.
    ///
    /// # Returns
    ///
    /// * `Result<Self, MeritRankError>` - A `Result` indicating success (`Ok`) or an error (`Err`) if the graph contains a self-reference.
    pub fn new(graph: MyGraph) -> Result<Self, MeritRankError> {
        // Check if the graph contains a self-reference
        if let Err(err) = graph.check_self_reference() {
            return Err(err);
        }

        Ok(MeritRank {
            graph,
            walks: WalkStorage::new(),
            personal_hits: HashMap::new(),
            neg_hits: HashMap::new(),
            alpha: 0.85,
        })
    }

    pub fn add_walk(&mut self, walk: RandomWalk, start_pos: usize) {
        self.walks.add_walk(walk, start_pos);
    }

    fn _get_walks(&self) -> &IndexMap<NodeId, IndexMap<WalkId, PosWalk>> {
        self.walks.get_walks()
    }

    fn _get_neg_hits(&self) -> &HashMap<NodeId, HashMap<NodeId, Weight>> {
        &self.neg_hits
    }

    fn _get_personal_hits(&self) -> &HashMap<NodeId, Counter> {
        &self.personal_hits
    }

    fn _get_graph(&self) -> &MyGraph {
        &self.graph
    }

    fn _get_graph_mut(&mut self) -> &mut MyGraph {
        &mut self.graph
    }

    fn _get_alpha(&self) -> Weight {
        self.alpha
    }

    fn _set_alpha(&mut self, alpha: Weight) {
        self.alpha = alpha;
    }

    // Get the hit count for a specific node
    fn _get_hit_counts(&self, node: &NodeId) -> Option<f64> {
        self.personal_hits
            .get(node)
            .and_then(|counter| counter.get_count(node))
            .map(|&count| count as f64)
    }

    /// Retrieves the weighted neighbors of a node.
    ///
    /// This method returns a hashmap of the neighbors of the specified `node`, along with their weights.
    /// Only neighbors with positive weights are returned if `positive` is `true`, and only neighbors with negative
    /// weights are returned if `positive` is `false`.
    ///
    /// # Arguments
    ///
    /// * `node` - The node for which to retrieve the neighbors.
    /// * `positive` - A boolean value indicating whether to return positive neighbors.
    ///
    /// # Returns
    ///
    /// A hashmap of the neighbors of the specified `node` and their weights, or `None` if no neighbors exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use meritrank::{MyGraph, NodeId, MeritRankError, MeritRank};
    ///
    /// let graph = MyGraph::new();
    /// let merit_rank = MeritRank::new(graph).unwrap();
    ///
    /// let node = NodeId::UInt(1);
    /// let positive = true;
    ///
    /// if let Some(neighbors) = merit_rank.neighbors_weighted(node, positive) {
    ///     for (neighbor, weight) in neighbors {
    ///         println!("Neighbor: {:?}, Weight: {:?}", neighbor, weight);
    ///     }
    /// } else {
    ///     println!("No neighbors found for the node.");
    /// }
    /// ```
    pub fn neighbors_weighted(
        &self,
        node: NodeId,
        positive: bool,
    ) -> Option<HashMap<NodeId, Weight>> {
        let neighbors: HashMap<_, _> = self
            .graph
            .neighbors(node)
            .into_iter()
            .filter_map(|nbr| {
                let weight = self.graph.edge_weight(node, nbr).unwrap_or_else(|| 0.0);
                if (positive && weight > 0.0) || (!positive && weight < 0.0) {
                    Some((nbr, weight))
                } else {
                    None
                }
            })
            .collect();

        if neighbors.is_empty() {
            None
        } else {
            Some(neighbors)
        }
    }

    /// Calculates the MeritRank from the perspective of the given node.
    ///
    /// If there are already walks for the node, they are dropped, and a new calculation is performed.
    ///
    /// # Arguments
    ///
    /// * `ego` - The source node to calculate the MeritRank for.
    /// * `num_walks` - The number of walks that should be used.
    ///
    /// # Returns
    ///
    /// * `Result<(), MeritRankError>` - A `Result` indicating success (`Ok`) or an error (`Err`) if the node does not exist.
    ///
    /// # Errors
    ///
    /// An error is returned if the specified node does not exist in the graph.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{MeritRank, MeritRankError, MyGraph, NodeId};
    ///
    /// let graph = MyGraph::new();
    /// let mut merit_rank = MeritRank::new(graph).unwrap();
    ///
    /// let ego = NodeId::UInt(1);
    /// let num_walks = 1000;
    ///
    /// if let Err(err) = merit_rank.calculate(ego, num_walks) {
    ///     match err {
    ///         MeritRankError::NodeDoesNotExist => {
    ///             println!("Error: The specified node does not exist.");
    ///         }
    ///         // Handle other error cases...
    ///     _ => {}}
    /// }
    /// ```
    pub fn calculate(&mut self, ego: NodeId, num_walks: usize) -> Result<(), MeritRankError> {
        self.walks.drop_walks_from_node(ego);

        if !self.graph.contains_node(ego) {
            return Err(MeritRankError::NodeDoesNotExist);
        }

        let mut negs = self
            .neighbors_weighted(ego, false)
            .unwrap_or(HashMap::new());

        self.personal_hits.insert(ego, Counter::new());

        for _ in 0..num_walks {
            let walk = self.perform_walk(ego)?;
            let walk_steps = walk.iter().cloned();

            self.personal_hits
                .entry(ego)
                .and_modify(|counter| counter.increment_unique_counts(walk_steps));

            self.update_negative_hits(&walk, &mut negs, false);
            self.add_walk(walk, 0);
        }

        Ok(())
    }

    /// Updates the negative hits based on a random walk and negative penalties.
    ///
    /// This method updates the negative hit counts for each node in the `walk` based on the penalties
    /// specified in the `negs` hashmap. The `subtract` flag determines whether the penalties should be added
    /// or subtracted from the hit counts.
    ///
    /// # Arguments
    ///
    /// * `walk` - The random walk for which to update the negative hits.
    /// * `negs` - A hashmap containing the negative penalties for each node.
    /// * `subtract` - A boolean flag indicating whether to subtract the penalties from the hit counts.
    pub fn update_negative_hits(
        &mut self,
        walk: &RandomWalk,
        negs: &mut HashMap<NodeId, Weight>,
        subtract: bool,
    ) {
        if walk.intersects_nodes(&negs.keys().cloned().collect::<Vec<NodeId>>()) {
            let ego_neg_hits = self
                .neg_hits
                .entry(walk.first_node().unwrap())
                .or_insert_with(HashMap::new);

            for (node, penalty) in walk.calculate_penalties(negs) {
                let adjusted_penalty = if subtract { -penalty } else { penalty };
                let entry = ego_neg_hits.entry(node).or_insert(0.0);
                *entry += adjusted_penalty;
            }
        }
    }

    /// Retrieves the MeritRank score for a target node from the perspective of the ego node.
    ///
    /// The score is calculated based on the accumulated hits and penalized by negative hits.
    ///
    /// # Arguments
    ///
    /// * `ego` - The ego node from whose perspective the score is calculated.
    /// * `target` - The target node for which the score is calculated.
    ///
    /// # Returns
    ///
    /// * `Weight` - The MeritRank score for the target node.
    ///
    /// # Panics
    ///
    /// A panic occurs if hits are greater than 0 but there is no path from the ego to the target node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meritrank::{MeritRank, MeritRankError, MyGraph, NodeId, Weight};
    ///
    /// let graph = MyGraph::new();
    /// let mut merit_rank = MeritRank::new(graph).unwrap();
    ///
    /// let ego = NodeId::UInt(1);
    /// let target = NodeId::UInt(2);
    ///
    /// let score = merit_rank.get_node_score(ego, target);
    ///
    /// println!("MeritRank score for node {:?} from node {:?}: {:?}", target, ego, score);
    /// ```
    pub fn get_node_score(&self, ego: NodeId, target: NodeId) -> Result<Weight, MeritRankError> {
        let counter = self
            .personal_hits
            .get(&ego)
            .ok_or(MeritRankError::NodeDoesNotCalculated)?;

        let hits = counter.get_count(&target).copied().unwrap_or(0.0);

        if ASSERT {
            let has_path = self.graph.is_connecting(ego, target);

            if hits > 0.0 && !has_path {
                return Err(MeritRankError::NoPathExists);
            }
        }

        let binding = HashMap::new();
        let neg_hits = self.neg_hits.get(&ego).unwrap_or(&binding);
        let hits_penalized = hits + neg_hits.get(&target).copied().unwrap_or(0.0);

        Ok(hits_penalized / counter.total_count())
    }

    /// Returns the ranks of peers for the given ego node.
    ///
    /// This method calculates the ranks of peers for the specified ego node based on their node scores.
    /// It retrieves the node scores from the personal hits counter and sorts them in descending order.
    /// The ranks are limited to the specified limit if provided.
    ///
    /// # Arguments
    ///
    /// * `ego` - The ego node for which to retrieve the ranks.
    /// * `limit` - The maximum number of ranks to return (optional).
    ///
    /// # Returns
    ///
    /// A dictionary of peer ranks, where keys are peer node IDs and values are their corresponding ranks.
    pub fn get_ranks(
        &self,
        ego: NodeId,
        limit: Option<usize>,
    ) -> Result<Vec<(NodeId, Weight)>, MeritRankError> {
        let counter = self
            .personal_hits
            .get(&ego)
            .ok_or(MeritRankError::NodeDoesNotExist)?;

        let mut peer_scores: Vec<(NodeId, Weight)> = counter
            .keys()
            .iter()
            .map(|&peer| Ok((peer, self.get_node_score(ego, peer)?)))
            .collect::<Result<Vec<(NodeId, Weight)>, MeritRankError>>()?;

        peer_scores.sort_unstable_by(|(_, score1), (_, score2)| {
            score2
                .partial_cmp(score1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let limit = limit.unwrap_or(peer_scores.len());
        let peer_scores: Vec<(NodeId, Weight)> = peer_scores.into_iter().take(limit).collect();

        Ok(peer_scores)
    }

    /// Performs a random walk starting from the specified node.
    ///
    /// This method generates a random walk starting from the `start_node` by iteratively selecting neighbors
    /// based on their weights until the stopping condition is met.
    ///
    /// # Arguments
    ///
    /// * `start_node` - The starting node for the random walk.
    ///
    /// # Returns
    ///
    /// A `Result` containing the random walk as a `RandomWalk` if successful, or a `MeritRankError` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use meritrank::{MyGraph, NodeId, MeritRankError, MeritRank};
    ///
    /// let graph = MyGraph::new();
    /// let merit_rank = MeritRank::new(graph).unwrap();
    ///
    /// let start_node = NodeId::UInt(1);
    ///
    /// match merit_rank.perform_walk(start_node) {
    ///     Ok(random_walk) => {
    ///         println!("Random walk: {:?}", random_walk);
    ///     }
    ///     Err(error) => {
    ///         println!("Error performing random walk: {}", error);
    ///     }
    /// }
    /// ```
    pub fn perform_walk(&self, start_node: NodeId) -> Result<RandomWalk, MeritRankError> {
        let mut walk = RandomWalk::new();
        walk.push(start_node);
        let new_segment = self.generate_walk_segment(start_node, false)?;
        walk.extend(&new_segment);
        Ok(walk)
    }

    /// Generates a walk segment for the specified start node.
    ///
    /// This method generates a walk segment by iteratively selecting neighbors based on their weights
    /// until the stopping condition is met.
    ///
    /// # Arguments
    ///
    /// * `start_node` - The starting node for the walk segment.
    /// * `skip_alpha_on_first_step` - A boolean flag indicating whether to skip the alpha probability check
    ///   on the first step of the walk segment.
    ///
    /// # Returns
    ///
    /// A `Result` containing the walk segment as a `Vec<NodeId>` if successful, or a `MeritRankError` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use meritrank::{MyGraph, NodeId, MeritRankError, MeritRank};
    ///
    /// let graph = MyGraph::new();
    /// let merit_rank = MeritRank::new(graph).unwrap();
    ///
    /// let start_node = NodeId::UInt(1);
    /// let skip_alpha_on_first_step = false;
    ///
    /// match merit_rank.generate_walk_segment(start_node, skip_alpha_on_first_step) {
    ///     Ok(walk_segment) => {
    ///         println!("Walk segment: {:?}", walk_segment);
    ///     }
    ///     Err(error) => {
    ///         println!("Error generating walk segment: {}", error);
    ///     }
    /// }
    /// ```
    pub fn generate_walk_segment(
        &self,
        start_node: NodeId,
        skip_alpha_on_first_step: bool,
    ) -> Result<Vec<NodeId>, MeritRankError> {
        let mut node = start_node;
        let mut segment = Vec::new();
        let mut rng = thread_rng();
        let mut skip_alpha_on_first_step = skip_alpha_on_first_step;

        while let Some(neighbors) = self.neighbors_weighted(node, true) {
            if skip_alpha_on_first_step || rng.gen::<f64>() <= self.alpha {
                skip_alpha_on_first_step = false;
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

    /// Randomly selects an item from a list of values based on their weights.
    ///
    /// This function performs a weighted random selection by assigning probabilities to each item based on their weights,
    /// and then selecting one item at random according to those probabilities.
    ///
    /// # Arguments
    ///
    /// * `values` - A slice containing the values to select from.
    /// * `weights` - A slice containing the weights corresponding to the values.
    /// * `rng` - A mutable reference to the random number generator.
    ///
    /// # Returns
    ///
    /// An `Option` containing the selected item if successful, or `None` if the selection fails.
    pub fn random_choice<T: Copy>(values: &[T], weights: &[f64], rng: &mut impl Rng) -> Option<T> {
        let dist = WeightedIndex::new(weights).ok()?;
        let index = dist.sample(rng);
        values.get(index).copied()
    }

    /// Retrieves the weight of an edge between two nodes.
    ///
    /// This method returns the weight of the edge between the source node (`src`) and the destination node (`dest`).
    /// If no edge exists between the nodes, `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `src` - The source node ID.
    /// * `dest` - The destination node ID.
    ///
    /// # Returns
    ///
    /// The weight of the edge between the source and destination nodes, or `None` if no edge exists.
    pub fn get_edge(&self, src: NodeId, dest: NodeId) -> Option<Weight> {
        self.graph.edge_weight(src, dest)
    }

    /// Updates penalties and negative hits for a specific edge.
    ///
    /// This method updates the penalties and negative hits for the edge between the source node (`src`) and the destination node (`dest`).
    /// It retrieves all walks that pass through the destination node and start with the source node.
    /// It then calculates the penalties for each affected walk based on the edge weight, and updates the negative hits accordingly.
    /// If `remove_penalties` is set to `true`, the penalties are subtracted instead of added to the negative hits.
    ///
    /// # Arguments
    ///
    /// * `src` - The source node ID.
    /// * `dest` - The destination node ID.
    /// * `remove_penalties` - A flag indicating whether to remove the penalties instead of adding them. Default is `false`.
    pub fn update_penalties_for_edge(&mut self, src: NodeId, dest: NodeId, remove_penalties: bool) {
        // Retrieve all walks that pass through the destination node and start with the source node
        let affected_walks = self
            .walks
            .get_walks_through_node(dest, |pos_walk| pos_walk.get_current_node() == src);

        // Check if the edge exists and retrieve the edge weight
        let weight = match self.get_edge(src, dest) {
            Some(weight) => weight,
            None => {
                // Edge does not exist, no action needed
                return;
            }
        };

        // Update penalties and negative hits for each affected walk
        let ego_neg_hits = self.neg_hits.entry(src).or_insert_with(HashMap::new);

        // Create a hashmap with the negative weight of the edge for the affected node
        let neg_weights: HashMap<NodeId, Weight> = [(dest, weight)].iter().cloned().collect();

        for walk in affected_walks {
            // Calculate penalties for the affected walk
            let penalties = walk.calculate_penalties(&neg_weights);

            // Update negative hits for each node in the penalties
            for (node, penalty) in penalties {
                let adjusted_penalty = if remove_penalties { -penalty } else { penalty };

                ego_neg_hits
                    .entry(node)
                    .and_modify(|entry| *entry += adjusted_penalty)
                    .or_insert(adjusted_penalty);
            }
        }
    }

    /// Clears an invalidated walk by subtracting the invalidated segment nodes from the hit counter.
    ///
    /// This method clears an invalidated walk by subtracting the nodes in the invalidated segment from the hit counter
    /// for the starting node of the invalidated walk. It ensures that the hit counter values remain non-negative.
    /// The invalidated segment may include nodes that are still present in the original walk, so special care is taken
    /// to avoid subtracting them from the counter by accident.
    ///
    /// # Arguments
    ///
    /// * `walk` - The invalidated walk.
    /// * `invalidated_segment` - The list of invalidated segment nodes.
    pub fn clear_invalidated_walk(&mut self, walk: &mut RandomWalk, invalidated_segment: &Vec<NodeId>) {
        // Get the starting node (ego) of the invalidated walk
        let ego = walk.first_node().unwrap();

        // Get or insert the hit counter for the starting node
        let counter: &mut Counter = self.personal_hits.entry(ego).or_insert_with(Counter::new);

        // Subtract the nodes in the invalidated segment from the hit counter
        let to_remove: HashSet<&NodeId> = invalidated_segment
            .into_iter()
            .filter(|node| !walk.contains(node))
            .collect();

        if to_remove.len() > 0 {
            for node_to_remove in to_remove {
                *counter.get_mut_count(node_to_remove) -= 1.0;
            }

            // Check if hit counter values are non-negative
            if ASSERT {
                for &c in counter.count_values() {
                    assert!(c >= 0.0);
                }
            }
        }
    }

    /// Recalculates an invalidated random walk by extending it with a new segment.
    ///
    /// This method extends an invalidated random walk (`walk`) by generating a new segment and appending it to the walk.
    /// The new segment is generated starting from the last node of the walk unless `force_first_step` is specified.
    /// If `force_first_step` is provided, it determines the first step of the new segment.
    /// The `skip_alpha_on_first_step` flag indicates whether to skip the alpha probability check on the first step of the new segment.
    ///
    /// # Arguments
    ///
    /// * `walk` - A mutable reference to the random walk that needs to be recalculated.
    /// * `force_first_step` - An optional `NodeId` representing the node to be used as the first step of the new segment.
    ///                        If `None`, the last node of the walk is used as the first step.
    /// * `skip_alpha_on_first_step` - A boolean flag indicating whether to skip the alpha probability check
    ///                                on the first step of the new segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use meritrank::{MeritRank, MyGraph, NodeId, RandomWalk};
    ///
    /// let graph = MyGraph::new();
    /// let mut merit_rank = MeritRank::new(graph).unwrap();
    /// let mut random_walk = RandomWalk::new();
    /// random_walk.extend(&*vec![NodeId::Int(1), NodeId::Int(2)]);
    /// // ... Initialize random_walk ...
    /// let force_first_step = Some(NodeId::Int(3));
    /// let skip_alpha_on_first_step = true;
    /// merit_rank.recalc_invalidated_walk(&mut random_walk, force_first_step, skip_alpha_on_first_step);
    /// ```
    pub fn recalc_invalidated_walk(
        &mut self,
        walk: &mut RandomWalk,
        force_first_step: Option<NodeId>,
        mut skip_alpha_on_first_step: bool,
    ) -> Result<(), MeritRankError> {
        // Get the ID of the first node in the walk
        let ego = walk.first_node().ok_or(MeritRankError::InvalidWalkLength)?;

        // Get the index where the new segment starts
        let new_segment_start = walk.len();

        // Determine the first step based on the `force_first_step` parameter
        let first_step = match force_first_step {
            Some(step) => step,
            None => walk.last_node().ok_or(MeritRankError::InvalidWalkLength)?,
        };

        // Check if the alpha probability should be skipped on the first step
        if force_first_step.is_some() {
            if skip_alpha_on_first_step {
                skip_alpha_on_first_step = false;
            } else {
                // Check if the random value exceeds the alpha probability
                if random::<f64>() >= self.alpha {
                    return Ok(()); // Exit the function early if the alpha check fails
                }
            }
        }

        // Generate the new segment
        let mut new_segment = self.generate_walk_segment(first_step, skip_alpha_on_first_step)?;

        // Insert the first step at the beginning of the new segment if necessary
        if let Some(force_first_step) = force_first_step {
            new_segment.insert(0, force_first_step);
        }

        // Update the personal hits counter for the new segment
        let counter: &mut Counter = self.personal_hits.entry(ego).or_insert_with(Counter::new);
        let node_set: HashSet<_> = new_segment.iter().cloned().collect();
        let diff: HashSet<_> = node_set
            .difference(&walk.get_nodes().iter().cloned().collect())
            .cloned()
            .collect();
        counter.increment_unique_counts(diff.iter().cloned());

        // Extend the walk with the new segment
        walk.extend(&new_segment);

        // Add the updated walk to the collection of walks
        self.add_walk(walk.clone(), new_segment_start);

        Ok(())
    }

    pub fn add_node(&mut self, node: NodeId) {
        self.graph.add_node(Node::new(node));
    }

    /// Adds an edge between two nodes with the specified weight.
    ///
    /// This method adds an edge between the source and destination nodes with the given weight.
    /// It handles various cases based on the old and new weights of the edge.
    ///
    /// # Arguments
    ///
    /// * `src` - The source node ID.
    /// * `dest` - The destination node ID.
    /// * `weight` - The weight of the edge (default is 1.0).
    ///
    /// # Panics
    ///
    /// This method panics if the source and destination nodes are the same.
    pub fn add_edge(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        if src == dest {
            panic!("Self reference not allowed");
        }

        let old_weight = self.graph.edge_weight(src, dest).unwrap_or(0.0);

        if old_weight == weight {
            return;
        }

        let old_sign = sign(old_weight);
        let new_sign = sign(weight);

        let row = old_sign as i32;
        let column = new_sign as i32;

        match (row, column) {
            (0, 0) => self.zz(src, dest, weight),
            (0, 1) => self.zp(src, dest, weight),
            (0, -1) => self.zn(src, dest, weight),
            (1, 0) => self.pz(src, dest, weight),
            (1, 1) => self.pp(src, dest, weight),
            (1, -1) => self.pn(src, dest, weight),
            (-1, 0) => self.nz(src, dest, weight),
            (-1, 1) => self.np(src, dest, weight),
            (-1, -1) => self.nn(src, dest, weight),
            _ => panic!("Invalid weight combination"),
        }
    }

    /// No-op function. Does nothing.
    fn zz(&mut self, _src: NodeId, _dest: NodeId, _weight: f64) {
        // No operation - do nothing
        // It should never happened that the old weight is zero and the new weight is zero
    }

    /// Handles the case where the old weight is zero and the new weight is positive.
    fn zp(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        // Clear the penalties resulting from the invalidated walks
        let step_recalc_probability =
            if OPTIMIZE_INVALIDATION && weight > 0.0 && self.graph.contains_node(src) {
                let g_edges = self
                    .neighbors_weighted(src, true)
                    .unwrap_or_else(HashMap::new);
                let sum_of_weights: f64 = g_edges.values().sum();
                weight / (sum_of_weights + weight)
            } else {
                0.0
            };

        let mut invalidated_walks =
            self.walks
                .invalidate_walks_through_node(src, Some(dest), step_recalc_probability);

        let mut negs_cache: HashMap<NodeId, HashMap<NodeId, f64>> = HashMap::new();
        for (walk, invalidated_segment) in &invalidated_walks {
            let mut negs = negs_cache
                .entry(walk.first_node().unwrap())
                .or_insert_with(|| {
                    self.neighbors_weighted(walk.first_node().unwrap(), false)
                        .unwrap_or_else(HashMap::new)
                });
            if negs.len() > 0 {
                let nodes = walk.iter().cloned()
                    .chain(invalidated_segment.iter().cloned())
                    .collect::<Vec<NodeId>>();

                self.update_negative_hits(&RandomWalk::from_nodes(nodes), &mut negs, true);
            }
        }

        if weight == 0.0 {
            if self.graph.contains_edge(src, dest) {
                self.graph.remove_edge(src, dest);
            }
        } else {
            self.graph.add_edge(src, dest, weight);
        }

        for (walk_mut, invalidated_segment) in &mut invalidated_walks {
            let first_node = walk_mut.first_node().unwrap();
            let invalidated_segment: &Vec<NodeId> =
                &invalidated_segment.iter().copied().collect::<Vec<NodeId>>();
            self.clear_invalidated_walk(walk_mut, invalidated_segment);
            // let mut walk_mut = walk.clone(); // Convert walk to a mutable reference
            let force_first_step = if step_recalc_probability > 0.0 {
                Some(dest)
            } else {
                None
            };
            let _ = self.recalc_invalidated_walk(
                walk_mut,
                force_first_step,
                OPTIMIZE_INVALIDATION && weight == 0.0,
            );

            if let Some(negs) = negs_cache.get_mut(&first_node) {
                if negs.len() > 0 {
                    self.update_negative_hits(walk_mut, negs, false);
                }
            } else {
                // Handle the case where negs is not found
                panic!("Negs not found");
            }

            // self.update_negative_hits(walk_mut, &mut negs_cache[&first_node], false);
        }

        // update walks
        self.walks.implement_changes(invalidated_walks);

        if ASSERT {
            for (ego, hits) in &self.personal_hits {
                for (peer, count) in hits {
                    let walks = self.walks.get_walks_through_node(*peer, |_| true);
                    if walks.len() != *count as usize {
                        assert!(false);
                    }
                    if *count > 0.0 && weight > 0.0 && !self.graph.is_connecting(*ego, *peer) {
                        assert!(false);
                    }
                }
            }
        }
    }

    /// Handles the case where the old weight is zero and the new weight is negative.
    fn zn(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        // Add an edge with the given weight
        self.graph.add_edge(src, dest, weight);
        // Update penalties for the edge
        self.update_penalties_for_edge(src, dest, false);
    }

    /// Handles the case where the old weight is positive and the new weight is zero.
    fn pz(&mut self, src: NodeId, dest: NodeId, _weight: f64) {
        // Call the zp method with weight set to 0.0
        self.zp(src, dest, 0.0);
    }

    /// Handles the case where the old weight is positive and the new weight is positive.
    fn pp(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        // Call the zp method with the given arguments
        self.zp(src, dest, weight);
    }

    /// Sets the weight of an edge to zero and updates the penalties.
    /// Then adds an edge with the given weight and updates the penalties.
    fn pn(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        // Call the pz and zn methods with the given arguments
        self.pz(src, dest, weight);
        self.zn(src, dest, weight);
    }

    /// Handles the case where the old weight is negative and the new weight is zero.
    fn nz(&mut self, src: NodeId, dest: NodeId, _weight: f64) {
        // Clear invalidated walks and update penalties
        self.update_penalties_for_edge(src, dest, true);
        // Remove the edge from the graph
        self.graph.remove_edge(src, dest);
    }

    /// Handles the case where the old weight is negative and the new weight is positive.
    fn np(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        // Call the nz and zp methods with the given arguments
        self.nz(src, dest, weight);
        self.zp(src, dest, weight);
    }

    /// Handles the case where the old weight is negative and the new weight is negative.
    fn nn(&mut self, src: NodeId, dest: NodeId, weight: f64) {
        // Call the nz and zn methods with the given arguments
        self.nz(src, dest, weight);
        self.zn(src, dest, weight);
    }

    // Experimental
    pub fn get_personal_hits(&self) -> &HashMap<NodeId, Counter> {
        &self.personal_hits
    }
}
