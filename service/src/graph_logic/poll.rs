use crate::utils::quantiles::calculate_quantiles_bounds;
use indexmap::IndexMap;
use indexmap::IndexSet;
use meritrank_core::{NodeId, Weight};
use std::collections::HashMap;

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
  polls:            HashMap<PollId, IndexSet<PollVariantId>>,
  options:          HashMap<PollVariantId, PollId>,
  pub(crate) votes: HashMap<PollId, HashMap<UserId, Vote>>,
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
    self.polls.get_mut(&poll).unwrap().swap_remove(&option);

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

  pub(crate) fn calculate_poll_results(
    &self,
    poll_votes: &HashMap<UserId, Vote>,
    scores: &Vec<(UserId, Weight)>,
    num_quantiles: usize,
    normalize: bool,
  ) -> IndexMap<PollVariantId, Weight> {
    let scores_capped = self.cap_scores(scores, num_quantiles);
    let scores_map: HashMap<_, _> = scores_capped.into_iter().collect();

    let mut results = IndexMap::new();
    for (user_id, vote) in poll_votes {
      let user_score = scores_map.get(user_id).unwrap_or(&0.0);
      *results.entry(vote.option).or_insert(0.0) += vote.weight * user_score;
    }

    // Sort the results by value (weight) in descending order
    results.sort_by(|_, a, _, b| {
      b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)
    });

    if normalize {
      // Normalize the results
      let total_weight: Weight = results.values().sum();
      if total_weight > 0.0 {
        for weight in results.values_mut() {
          *weight /= total_weight;
        }
      }
    }

    results
  }

  fn cap_scores(
    &self,
    scores: &Vec<(UserId, Weight)>,
    num_quantiles: usize,
  ) -> Vec<(UserId, Weight)> {
    let quantiles = calculate_quantiles_bounds(
      scores.iter().map(|(_, weight)| *weight).collect(),
      num_quantiles,
    );
    let quantile_bound = *quantiles.last().unwrap_or(&Weight::MAX);

    scores
      .into_iter()
      .cloned()
      .map(|(user_id, weight)| {
        let capped_weight = weight.min(quantile_bound);
        (user_id, capped_weight)
      })
      .collect()
  }
}
// Add this at the end of the file

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_cap_scores() {
    let poll_store = PollStore::new();
    let scores = vec![(1, 10.0), (2, 20.0), (3, 30.0), (4, 40.0), (5, 50.0)];
    let num_quantiles = 4;

    let capped_scores = poll_store.cap_scores(&scores, num_quantiles);

    // The expected cap should be at the 75th percentile (3rd quantile)
    assert_eq!(capped_scores.len(), 5);
    assert_eq!(capped_scores[0], (1, 10.0));
    assert_eq!(capped_scores[1], (2, 20.0));
    assert_eq!(capped_scores[2], (3, 30.0));
    assert_eq!(capped_scores[3], (4, 35.0)); // Capped at 75th percentile
    assert_eq!(capped_scores[4], (5, 35.0)); // Capped at 75th percentile
  }

  #[test]
  fn test_calculate_poll_results() {
    let mut poll_store = PollStore::new();

    // Set up a poll with two options
    poll_store.add_poll_option(101, 1).unwrap(); // Option 1 for Poll 1
    poll_store.add_poll_option(102, 1).unwrap(); // Option 2 for Poll 1

    // Set up votes
    let poll_votes = HashMap::from([
      (
        1,
        Vote {
          option: 101,
          weight: 1.0,
        },
      ),
      (
        3,
        Vote {
          option: 101,
          weight: 1.0,
        },
      ),
      (
        4,
        Vote {
          option: 102,
          weight: 1.0,
        },
      ),
      (
        5,
        Vote {
          option: 101,
          weight: 1.0,
        },
      ),
    ]);

    // Set up user scores
    let scores = vec![(1, 10.0), (2, 20.0), (3, 30.0), (4, 40.0), (5, 50.0)];

    let results =
      poll_store.calculate_poll_results(&poll_votes, &scores, 4, true);

    // Check that we have results for both options
    assert_eq!(results.len(), 2);

    // Check that the results are normalized
    let total: f64 = results.values().sum();
    assert!((total - 1.0).abs() < 1e-6);

    // Option 101 should have more weight due to user 5's high score
    assert!(results[&101] > results[&102]);
  }

  #[test]
  fn test_calculate_poll_results_empty_votes() {
    let poll_store = PollStore::new();
    let poll_votes = HashMap::new();
    let scores = vec![(1, 10.0), (2, 20.0)];

    let results =
      poll_store.calculate_poll_results(&poll_votes, &scores, 4, true);

    // Results should be empty when there are no votes
    assert!(results.is_empty());
  }

  #[test]
  fn test_calculate_poll_results_zero_scores() {
    let mut poll_store = PollStore::new();

    poll_store.add_poll_option(101, 1).unwrap();
    poll_store.add_poll_option(102, 1).unwrap();

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

    let scores = vec![(1, 0.0), (2, 0.0)];

    let results =
      poll_store.calculate_poll_results(&poll_votes, &scores, 4, true);

    // Results should have zero weight for each option
    assert_eq!(results.len(), 2);
    assert_eq!(results.get(&101), Some(&0.0));
    assert_eq!(results.get(&102), Some(&0.0));
  }
}
