use crate::log::*;
use std::sync::atomic::Ordering;

//  No context
pub const CMD_VERSION: &str = "version";
pub const CMD_LOG_LEVEL: &str = "log_level";
pub const CMD_SYNC: &str = "sync";
pub const CMD_RESET: &str = "reset";
pub const CMD_RECALCULATE_ZERO: &str = "recalculate_zero";
pub const CMD_RECALCULATE_CLUSTERING: &str = "recalculate_clustering";
pub const CMD_NODE_LIST: &str = "node_list";
pub const CMD_READ_NEW_EDGES_FILTER: &str = "read_new_edges_filter";
pub const CMD_WRITE_NEW_EDGES_FILTER: &str = "write_new_edges_filter";
pub const CMD_FETCH_NEW_EDGES: &str = "fetch_new_edges";

//  With context
pub const CMD_NODE_SCORE: &str = "node_score";
pub const CMD_SCORES: &str = "scores";
pub const CMD_PUT_EDGE: &str = "put_edge";
pub const CMD_DELETE_EDGE: &str = "delete_edge";
pub const CMD_DELETE_NODE: &str = "delete_node";
pub const CMD_GRAPH: &str = "graph";
pub const CMD_CONNECTED: &str = "connected";
pub const CMD_EDGES: &str = "edges";
pub const CMD_MUTUAL_SCORES: &str = "mutual_scores";
pub const CMD_CREATE_CONTEXT: &str = "create_context";
pub const CMD_SET_ZERO_OPINION: &str = "set_zero_opinion";
pub const CMD_NEIGHBORS: &str = "neighbors";

pub const NEIGHBORS_ALL: i64 = 0;
pub const NEIGHBORS_OUTBOUND: i64 = 1;
pub const NEIGHBORS_INBOUND: i64 = 2;

#[derive(Clone, Debug)]
pub struct Command {
  pub id:       String,
  pub context:  String,
  pub blocking: bool,
  pub payload:  Vec<u8>,
}

pub fn encode_request(command: &Command) -> Result<Vec<u8>, String> {
  match rmp_serde::to_vec(&(
    command.id.clone(),
    command.context.clone(),
    command.blocking,
    command.payload.clone(),
  )) {
    Ok(x) => Ok(x),
    Err(s) => Err(s.to_string()),
  }
}

pub fn decode_request(request: &[u8]) -> Result<Command, ()> {
  match rmp_serde::from_slice(request) {
    Ok((command_value, context_value, blocking_value, payload_value)) => {
      Ok(Command {
        id:       command_value,
        context:  context_value,
        blocking: blocking_value,
        payload:  payload_value,
      })
    },
    Err(e) => {
      log_error!("(request_decode) {}", e);
      Err(())
    },
  }
}

pub fn encode_response<T>(response: &T) -> Result<Vec<u8>, ()>
where
  T: serde::ser::Serialize,
{
  match rmp_serde::to_vec(response) {
    Ok(x) => Ok(x),
    Err(e) => match rmp_serde::to_vec(&e.to_string()) {
      Ok(x) => Ok(x),
      Err(e) => {
        log_error!("(response_encode) {}", e);
        Err(())
      },
    },
  }
}

pub fn decode_response<'a, T>(response: &'a [u8]) -> Result<T, String>
where
  T: Clone + serde::Deserialize<'a>,
{
  match rmp_serde::from_slice::<T>(response) {
    Ok(x) => Ok(x.clone()),
    Err(e) => Err(e.to_string()),
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NeighborDirection {
    All,
    Outbound,
    Inbound,
}

pub fn neighbor_dir_from(direction: i64) -> Result<NeighborDirection, ()> {
    match direction {
        NEIGHBORS_ALL => Ok(NeighborDirection::All),
        NEIGHBORS_OUTBOUND => Ok(NeighborDirection::Outbound),
        NEIGHBORS_INBOUND => Ok(NeighborDirection::Inbound),
        _ => Err(()),
    }
}
