use crate::aug_multi_graph::AugMultiGraphSettings;
use crate::log::{log_error, log_warning}; // Specific imports
use std::env::var;
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::cmp::Ord;

pub(crate) fn parse_env_var<T>(
  name: &str,
  min: T,
  max: T,
) -> Result<Option<T>, ()>
where
  T: std::str::FromStr,
  T: std::cmp::Ord,
{
  match var(name) {
    Ok(s) => match s.parse::<T>() {
      Ok(n) => {
        if n >= min && n <= max {
          Ok(Some(n))
        } else {
          log_error!("Invalid {}: {:?}", name, s);
          Err(())
        }
      },
      _ => {
        log_error!("Invalid {}: {:?}", name, s);
        Err(())
      },
    },
    _ => Ok(None),
  }
}

pub(crate) fn parse_and_set_value<T>(
  value: &mut T,
  name: &str,
  min: T,
  max: T,
) -> Result<(), ()>
where
  T: std::str::FromStr,
  T: std::cmp::Ord,
{
  match parse_env_var(name, min, max)? {
    Some(n) => *value = n,
    _ => {},
  }
  Ok(())
}

pub(crate) fn parse_and_set_bool(
  value: &mut bool,
  name: &str,
) -> Result<(), ()> {
  match var(name) {
    Ok(s) => {
      if s == "1" || s.to_lowercase() == "true" || s.to_lowercase() == "yes" {
        *value = true;
        Ok(())
      } else if s == "0"
        || s.to_lowercase() == "false"
        || s.to_lowercase() == "no"
      {
        *value = false;
        Ok(())
      } else {
        log_error!(
          "Invalid {} (expected 0/1, true/false, yes/no): {:?}",
          name,
          s
        );
        Err(())
      }
    },
    _ => Ok(()),
  }
}

pub fn parse_settings() -> Result<AugMultiGraphSettings, ()> {
  let mut settings = AugMultiGraphSettings::default();

  //  TODO: Remove.
  match parse_env_var("MERITRANK_NUM_WALK", 0, 1000000)? {
    Some(n) => {
      log_warning!(
        "DEPRECATED: Use MERITRANK_NUM_WALKS instead of MERITRANK_NUM_WALK."
      );
      settings.num_walks = n;
    },
    _ => {},
  }

  parse_and_set_value(
    &mut settings.num_walks,
    "MERITRANK_NUM_WALKS",
    0,
    1000000,
  )?;
  parse_and_set_value(
    &mut settings.top_nodes_limit,
    "MERITRANK_TOP_NODES_LIMIT",
    0,
    1000000,
  )?;
  parse_and_set_value(
    &mut settings.zero_opinion_num_walks,
    "MERITRANK_ZERO_OPINION_NUM_WALKS",
    0,
    1000000,
  )?;

  match parse_env_var("MERITRANK_ZERO_OPINION_FACTOR", 0, 100)? {
    Some(n) => settings.zero_opinion_factor = (n as f64) * 0.01,
    _ => {},
  }

  const MIN_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(1).unwrap();
  const MAX_CACHE_SIZE: NonZeroUsize =
    NonZeroUsize::new(1024 * 1024 * 100).unwrap();

  parse_and_set_value(
    &mut settings.score_clusters_timeout,
    "MERITRANK_SCORE_CLUSTERS_TIMEOUT",
    0,
    60 * 60 * 24 * 365,
  )?;
  parse_and_set_value(
    &mut settings.scores_cache_size,
    "MERITRANK_SCORES_CACHE_SIZE",
    MIN_CACHE_SIZE,
    MAX_CACHE_SIZE,
  )?;
  parse_and_set_value(
    &mut settings.walks_cache_size,
    "MERITRANK_WALKS_CACHE_SIZE",
    MIN_CACHE_SIZE,
    MAX_CACHE_SIZE,
  )?;
  parse_and_set_value(
    &mut settings.filter_num_hashes,
    "MERITRANK_FILTER_NUM_HASHES",
    1,
    1024,
  )?;
  parse_and_set_value(
    &mut settings.filter_max_size,
    "MERITRANK_FILTER_MAX_SIZE",
    1,
    1024 * 1024 * 10,
  )?;
  parse_and_set_value(
    &mut settings.filter_min_size,
    "MERITRANK_FILTER_MIN_SIZE",
    1,
    1024 * 1024 * 10,
  )?;
  parse_and_set_bool(
    &mut settings.omit_neg_edges_scores,
    "MERITRANK_OMIT_NEG_EDGES_SCORES",
  )?;
  parse_and_set_bool(
    &mut settings.force_read_graph_conn,
    "MERITRANK_FORCE_READ_GRAPH_CONN",
  )?;

  Ok(settings)
}
