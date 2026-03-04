use std::error::Error;
use std::fmt::{Display, Formatter, Result};

/// Short diagnostic tags for InternalFatalError; used so the single log at the service shows the site.
pub mod internal_fatal {
  // rank.rs
  pub const RANK_CALCULATE_GET_WALK_MUT: &str =
    "rank::calculate get_walk_mut None";
  pub const RANK_SET_EDGE_GET_NODE_DATA_SRC: &str =
    "rank::set_edge_ get_node_data(src) None";
  pub const RANK_SET_EDGE_GET_WALK: &str = "rank::set_edge_ get_walk None";
  pub const RANK_SET_EDGE_FIRST_NODE: &str =
    "rank::set_edge_ walk first_node None";
  pub const RANK_SET_EDGE_GET_WALK_MUT: &str =
    "rank::set_edge_ get_walk_mut None";
  pub const RANK_ASSERT_GET_VISITS: &str =
    "rank::assert_counters get_visits_through_node None";
  pub const RANK_ASSERT_POS_HITS_COUNT: &str =
    "rank::assert_counters pos_hits count mismatch";
  pub const RANK_ASSERT_NEG_HITS_COUNT: &str =
    "rank::assert_counters neg_hits count mismatch";
  // graph.rs
  pub const GRAPH_NODEDATA_POS_WEIGHTED_INDEX: &str =
    "graph::NodeData random_neighbor pos WeightedIndex::new Err";
  pub const GRAPH_NODEDATA_POS_DISTR_CACHE: &str =
    "graph::NodeData pos_distr_cache None";
  pub const GRAPH_NODEDATA_POS_KEYS_NTH: &str =
    "graph::NodeData pos_edges.keys().nth None";
  pub const GRAPH_NODEDATA_ABS_WEIGHTED_INDEX: &str =
    "graph::NodeData random_neighbor abs WeightedIndex::new Err";
  pub const GRAPH_NODEDATA_ABS_DISTR_CACHE: &str =
    "graph::NodeData abs_distr_cache None";
  pub const GRAPH_GET_NODE_AT_INDEX_POS: &str =
    "graph::get_node_at_index pos_edges.keys().nth None";
  pub const GRAPH_GET_NODE_AT_INDEX_NEG: &str =
    "graph::get_node_at_index neg_edges.keys().nth None";
  pub const GRAPH_SET_EDGE_REMOVE_FAILED: &str =
    "graph::set_edge remove_edge failed";
  pub const GRAPH_GENERATE_WALK_GET_NODE_DATA: &str =
    "graph::generate_walk_segment get_node_data_mut None";
  pub const GRAPH_CONTINUE_WALK_LAST_NODE: &str =
    "graph::continue_walk last_node None";
  // walk_storage.rs
  pub const WALK_STORAGE_DROP_WALKS_GET_MUT: &str =
    "walk_storage::drop_walks_from_node get_mut None";
  pub const WALK_STORAGE_ASSERT_VISITS: &str =
    "walk_storage::assert_visits_consistency";
  pub const WALK_STORAGE_FIND_AFFECTED_GET_WALK: &str =
    "walk_storage::find_affected_walkids get_walk None";
  pub const WALK_STORAGE_FIND_AFFECTED_DST_NONE: &str =
    "walk_storage::find_affected_walkids dst_node None";
  pub const WALK_STORAGE_SPLIT_GET_MUT: &str =
    "walk_storage::split_and_remove_from_bookkeeping get_mut None";
  pub const WALK_DECIDE_SKIP_DELETION_POS: &str =
    "decide_skip_invalidation_on_edge_deletion pos >= walk.len";
  pub const WALK_DECIDE_SKIP_ADDITION_POS: &str =
    "decide_skip_invalidation_on_edge_addition pos >= walk.len";
  // random_walk.rs
  pub const RANDOM_WALK_PUSH_SELF_LOOP: &str = "random_walk::push self-loop";
  pub const RANDOM_WALK_PUSH_NEG_SEGMENT: &str =
    "random_walk::push negative_segment_start already Some";
  pub const RANDOM_WALK_EXTEND_TWO_NEG: &str =
    "random_walk::extend two negative segments";
}

#[derive(Debug, Clone)]
pub enum MeritRankError {
  InfWeightEncountered,
  NaNWeightEncountered,
  ZeroWeightEncountered,
  NodeDoesNotExist,
  SelfReferenceNotAllowed,
  RandomChoiceError,
  NoPathExists,
  NodeIdParseError,
  NodeIsNotCalculated,
  InvalidWalkLength,
  NodeNotFound,
  WalkNotFound,
  EdgeNotFound,
  InternalFatalError(Option<&'static str>),
}

impl Display for MeritRankError {
  fn fmt(
    &self,
    f: &mut Formatter<'_>,
  ) -> Result {
    match self {
      MeritRankError::InfWeightEncountered => {
        write!(f, "Edge with infinite weights are not allowed")
      },
      MeritRankError::NaNWeightEncountered => {
        write!(f, "Edge with NaN weights are not allowed")
      },
      MeritRankError::ZeroWeightEncountered => {
        write!(f, "Edge with zero weights are not allowed")
      },
      MeritRankError::NodeDoesNotExist => {
        write!(f, "Node does not exist")
      },
      MeritRankError::SelfReferenceNotAllowed => {
        write!(f, "Self-reference is not allowed")
      },
      MeritRankError::RandomChoiceError => {
        write!(f, "Random choice error")
      },
      MeritRankError::NoPathExists => write!(f, "No path exists"),
      MeritRankError::NodeIdParseError => {
        write!(f, "Node ID parse error")
      },
      MeritRankError::NodeIsNotCalculated => {
        write!(f, "Node is not calculated")
      },
      MeritRankError::InvalidWalkLength => {
        write!(f, "Invalid walk length")
      },
      MeritRankError::NodeNotFound => {
        write!(f, "Can't find the node with the given ID")
      },
      MeritRankError::WalkNotFound => {
        write!(f, "Can't find the walk with the given ID")
      },
      MeritRankError::EdgeNotFound => {
        write!(f, "Can't find the edge between given nodes")
      },
      MeritRankError::InternalFatalError(None) => {
        write!(f, "Internal fatal error")
      },
      MeritRankError::InternalFatalError(Some(site)) => {
        write!(f, "Internal fatal error: {}", site)
      },
    }
  }
}

impl Error for MeritRankError {}
