use crate::pushsum::{calculate_consensus, PsNode, PushsumAdjMap};
use crate::subgraph::Subgraph;
use indexmap::IndexSet;
use meritrank_core::{NodeId, Weight};
use std::collections::{HashMap, HashSet};

pub type PollId = NodeId;
pub type PollVariantId = NodeId;
pub type UserId = NodeId;

#[derive(Debug, Clone)]
pub struct Vote {
  pub option: PollVariantId,
  pub weight: Weight,
}

#[derive(Debug, Default, Clone)]
pub struct PollStore {
  polls:   HashMap<PollId, IndexSet<PollVariantId>>,
  options: HashMap<PollVariantId, PollId>,
  votes:   HashMap<PollId, HashMap<UserId, Vote>>,
}

impl PollStore {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn add_poll_option(
    &mut self,
    option: PollVariantId,
    poll: PollId,
  ) -> Result<(), &'static str> {
    if self.options.contains_key(&option) {
      return Err("Option already exists in a poll");
    }

    self.polls.entry(poll).or_default().insert(option);
    self.options.insert(option, poll);
    Ok(())
  }

  pub fn add_user_vote(
    &mut self,
    user: UserId,
    option: PollVariantId,
    weight: Weight,
  ) -> Result<(), &'static str> {
    let poll = self.options.get(&option).ok_or("Option does not exist")?;
    let vote = Vote {
      option,
      weight,
    };
    self.votes.entry(*poll).or_default().insert(user, vote);
    Ok(())
  }

  pub fn remove_option_from_poll(
    &mut self,
    option: PollVariantId,
  ) -> Result<(), &'static str> {
    let poll = self
      .options
      .remove(&option)
      .ok_or("Option does not exist")?;
    self.polls.get_mut(&poll).unwrap().remove(&option);

    if let Some(poll_votes) = self.votes.get_mut(&poll) {
      poll_votes.retain(|_, vote| vote.option != option);
    }

    Ok(())
  }

  pub fn remove_user_vote(
    &mut self,
    user: UserId,
    poll: PollId,
  ) -> Result<(), &'static str> {
    if let Some(poll_votes) = self.votes.get_mut(&poll) {
      if poll_votes.remove(&user).is_none() {
        return Err("Vote not found");
      }
    } else {
      return Err("No votes for this poll");
    }

    Ok(())
  }

  pub fn remove_poll(
    &mut self,
    poll: PollId,
  ) -> Result<(), &'static str> {
    if let Some(options) = self.polls.remove(&poll) {
      for option in options {
        self.options.remove(&option);
      }
      self.votes.remove(&poll);
      Ok(())
    } else {
      Err("Poll does not exist")
    }
  }

  pub fn get_poll_options(
    &self,
    poll: PollId,
  ) -> Option<&IndexSet<PollVariantId>> {
    self.polls.get(&poll)
  }

  pub fn get_option_poll(
    &self,
    option: PollVariantId,
  ) -> Option<&PollId> {
    self.options.get(&option)
  }

  fn get_poll_votes(
    &self,
    poll: PollId,
  ) -> Option<&HashMap<UserId, Vote>> {
    self.votes.get(&poll)
  }

  fn get_option_votes(
    &self,
    option: PollVariantId,
  ) -> Option<Vec<(&UserId, &Vote)>> {
    self.options.get(&option).and_then(|poll| {
      self.votes.get(poll).map(|votes| {
        votes
          .iter()
          .filter(|(_, vote)| vote.option == option)
          .collect()
      })
    })
  }

  pub fn get_poll_results_simple(
    &self,
    ego: UserId,
    poll: PollId,
  ) -> Option<Vec<(PollVariantId, Weight)>> {
    self.votes.get(&poll).map(|poll_votes| {
      let mut results: HashMap<PollVariantId, Weight> = HashMap::new();

      for (_user, vote) in poll_votes.iter() {
        *results.entry(vote.option).or_insert(0.0) += vote.weight;
      }

      let mut sorted_results: Vec<(PollVariantId, Weight)> =
        results.into_iter().collect();
      sorted_results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
      });
      sorted_results
    })
  }

  pub fn get_poll_results(
    &self,
    poll_id: PollId,
    graph: &mut Subgraph,
    iteration_count: usize,
  ) -> Option<Vec<(PollVariantId, Weight)>> {
    let poll_variants_set = self.polls.get(&poll_id)?;
    let poll_votes = self.get_poll_votes(poll_id)?;

    let voters_set = self.get_voters_set(poll_votes);
    let trust_map = self.build_trust_map(poll_votes, graph, &voters_set);
    // Transpose the trust adjacency matrix to make it publish adj matrix
    // suitable for push-sum algorithm
    let push_map = self.transpose_trust_map(&trust_map);
    let normalized_push_map = self.normalize_push_map(push_map);
    let adj_map = self.build_adj_map(poll_votes, poll_variants_set, &normalized_push_map);
    self.calculate_and_sort_results(adj_map, poll_variants_set, iteration_count)
  }

  fn get_voters_set(
    &self,
    poll_votes: &HashMap<UserId, Vote>,
  ) -> HashSet<UserId> {
    poll_votes.keys().copied().collect()
  }

  pub fn build_trust_map(
    &self,
    poll_votes: &HashMap<UserId, Vote>,
    graph: &mut Subgraph,
    voters_set: &HashSet<UserId>,
  ) -> HashMap<UserId, HashMap<UserId, Weight>> {
    poll_votes
      .keys()
      .map(|&user_id| {
        let edges = graph
          .fetch_all_raw_scores(user_id, 0.0)
          .into_iter()
          .filter(|&(node_id, w)| voters_set.contains(&node_id) && w > 0.0)
          .collect();
        (user_id, edges)
      })
      .collect()
  }

  fn transpose_trust_map(
    &self,
    trust_map: &HashMap<UserId, HashMap<UserId, Weight>>,
  ) -> HashMap<UserId, HashMap<UserId, Weight>> {
    trust_map
      .iter()
      .flat_map(|(&from_user, edges)| {
        edges
          .iter()
          .map(move |(&to_user, &weight)| (to_user, (from_user, weight)))
      })
      .fold(HashMap::new(), |mut acc, (to_user, (from_user, weight))| {
        acc
          .entry(to_user)
          .or_insert_with(HashMap::new)
          .insert(from_user, weight);
        acc
      })
  }

  fn normalize_push_map(
    &self,
    transposed_trust_map: HashMap<UserId, HashMap<UserId, Weight>>,
  ) -> HashMap<UserId, HashMap<UserId, Weight>> {
    transposed_trust_map
      .into_iter()
      .map(|(user, edges)| {
        let sum: Weight = edges.values().sum();
        let normalized_edges = if sum > 0.0 {
          edges
            .into_iter()
            .map(|(to_user, weight)| (to_user, weight / sum))
            .collect()
        } else {
          edges
        };
        (user, normalized_edges)
      })
      .collect()
  }

  fn build_adj_map(
    &self,
    poll_votes: &HashMap<UserId, Vote>,
    poll_variants_set: &IndexSet<PollVariantId>,
    normalized_trust_map: &HashMap<UserId, HashMap<UserId, Weight>>,
  ) -> PushsumAdjMap {
    poll_votes
      .iter()
      .map(|(&user_id, vote)| {
        let mut initial_opinion_vec = vec![0.0; poll_variants_set.len()];
        initial_opinion_vec
          [poll_variants_set.get_index_of(&vote.option).unwrap()] = vote.weight;

        let edges = normalized_trust_map
          .get(&user_id)
          .cloned()
          .unwrap_or_default();

        (user_id, PsNode::new_hot(initial_opinion_vec, edges))
      })
      .collect()
  }

  fn calculate_and_sort_results(
    &self,
    adj_map: PushsumAdjMap,
    poll_variants_set: &IndexSet<PollVariantId>,
    iteration_count: usize,
  ) -> Option<Vec<(PollVariantId, Weight)>> {
    calculate_consensus(adj_map, iteration_count)
      .map(|consensus_vec| {
        println!("Consensus distribution: {:?}", consensus_vec);
        let mut results: Vec<(PollVariantId, Weight)> = consensus_vec
          .into_iter()
          .enumerate()
          .map(|(ind, weight)| {
            (*poll_variants_set.get_index(ind).unwrap(), weight)
          })
          .collect();

        results.sort_by(|a, b| {
          b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        results
      })
      .or_else(|| {
        println!("No consensus reached");
        Some(Vec::new())
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_transpose_trust_map() {
    let poll_store = PollStore::new();
    let trust_map = HashMap::from([
      (1, HashMap::from([(2, 0.5), (3, 0.3)])),
      (2, HashMap::from([(1, 0.6), (3, 0.4)])),
    ]);

    let transposed = poll_store.transpose_trust_map(&trust_map);

    assert_eq!(transposed[&1][&2], 0.6);
    assert_eq!(transposed[&2][&1], 0.5);
    assert_eq!(transposed[&3][&1], 0.3);
    assert_eq!(transposed[&3][&2], 0.4);
  }

  #[test]
  fn test_normalize_trust_map() {
    let poll_store = PollStore::new();
    let transposed_trust_map = HashMap::from([
      (1, HashMap::from([(2, 0.5), (3, 0.5)])),
      (2, HashMap::from([(1, 0.3), (3, 0.7)])),
    ]);

    let normalized = poll_store.normalize_push_map(transposed_trust_map);

    assert_eq!(normalized[&1][&2], 0.5);
    assert_eq!(normalized[&1][&3], 0.5);
    assert!((normalized[&2][&1] - 0.3).abs() < f64::EPSILON);
    assert!((normalized[&2][&3] - 0.7).abs() < f64::EPSILON);
  }

  #[test]
  fn test_build_adj_map() {
    let poll_store = PollStore::new();
    let poll_votes = HashMap::from([
      (
        1,
        Vote {
          option: 101,
          weight: 1.0,
        },
      ),
      (
        2,
        Vote {
          option: 102,
          weight: 1.0,
        },
      ),
    ]);
    let poll_variants_set: IndexSet<PollVariantId> =
      vec![101, 102].into_iter().collect();
    let normalized_trust_map = HashMap::from([
      (1, HashMap::from([(2, 0.5)])),
      (2, HashMap::from([(1, 0.5)])),
    ]);

    let adj_map = poll_store.build_adj_map(
      &poll_votes,
      &poll_variants_set,
      &normalized_trust_map,
    );

    assert_eq!(adj_map.len(), 2);
    assert_eq!(adj_map[&1].s, vec![1.0, 0.0]);
    assert_eq!(adj_map[&2].s, vec![0.0, 1.0]);
    assert_eq!(adj_map[&1].edges[&2], 0.5);
    assert_eq!(adj_map[&2].edges[&1], 0.5);
  }
}
