use meritrank_core::{NodeId, Weight};
use std::collections::{HashMap, HashSet};

pub type PollId = NodeId;
pub type PollOptionId = NodeId;
pub type UserId = NodeId;

#[derive(Debug, Clone)]
struct Vote {
  option: PollOptionId,
  weight: Weight,
}

#[derive(Debug, Default, Clone)]
pub struct PollStore {
  polls:   HashMap<PollId, HashSet<PollOptionId>>,
  options: HashMap<PollOptionId, PollId>,
  votes:   HashMap<PollId, HashMap<UserId, Vote>>,
}

impl PollStore {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn add_poll_option(
    &mut self,
    option: PollOptionId,
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
    option: PollOptionId,
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
    option: PollOptionId,
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
  ) -> Option<&HashSet<PollOptionId>> {
    self.polls.get(&poll)
  }

  pub fn get_option_poll(
    &self,
    option: PollOptionId,
  ) -> Option<&PollId> {
    self.options.get(&option)
  }

  // fn get_poll_votes(
  //   &self,
  //   poll: PollId,
  // ) -> Option<&HashMap<UserId, Vote>> {
  //   self.votes.get(&poll)
  // }

  // fn get_option_votes(
  //   &self,
  //   option: PollOptionId,
  // ) -> Option<Vec<(&UserId, &Vote)>> {
  //   self.options.get(&option).and_then(|poll| {
  //     self.votes.get(poll).map(|votes| {
  //       votes
  //         .iter()
  //         .filter(|(_, vote)| vote.option == option)
  //         .collect()
  //     })
  //   })
  // }

  pub fn get_poll_results(
    &self,
    _ego: UserId,
    poll: PollId,
  ) -> Option<Vec<(PollOptionId, Weight)>> {
    self.votes.get(&poll).map(|poll_votes| {
        let mut results: HashMap<PollOptionId, Weight> = HashMap::new();

        for (_user, vote) in poll_votes.iter() {
          *results.entry(vote.option).or_insert(0.0) += vote.weight;
        }

        let mut sorted_results: Vec<(PollOptionId, Weight)> = results.into_iter().collect();
        sorted_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted_results
    })
  }
}
