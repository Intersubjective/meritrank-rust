use nng::{Aio, AioResult, Context, Protocol, Socket};
use std::{env::var, string::ToString, sync::atomic::Ordering};

use crate::aug_multi_graph::*;
use crate::constants::*;
use crate::errors::ServiceError;
use crate::log::*;
use crate::protocol::*;
use crate::read_ops;
use crate::state_manager::*;
use crate::write_ops;
use std::time::SystemTime;

pub use meritrank_core::Weight;

fn request_from_command(command: &Command) -> Request {
  match command.id.as_str() {
    CMD_RESET => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::WriteReset;
      }
    },
    CMD_RECALCULATE_ZERO => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::WriteRecalculateZero;
      }
    },
    CMD_SET_ZERO_OPINION => {
      if let Ok((node, score)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::WriteSetZeroOpinion(
          command.context.clone(),
          node,
          score,
        );
      }
    },
    CMD_RECALCULATE_CLUSTERING => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::WriteRecalculateClustering;
      }
    },
    CMD_DELETE_EDGE => {
      if let Ok((src, dst, index)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::WriteDeleteEdge(
          command.context.clone(),
          src,
          dst,
          index,
        );
      }
    },
    CMD_DELETE_NODE => {
      if let Ok((node, index)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::WriteDeleteNode(command.context.clone(), node, index);
      }
    },
    CMD_PUT_EDGE => {
      if let Ok((src, dst, amount, index)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::WritePutEdge(
          command.context.clone(),
          src,
          dst,
          amount,
          index,
        );
      }
    },
    CMD_CREATE_CONTEXT => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::WriteCreateContext(command.context.clone());
      }
    },
    CMD_WRITE_NEW_EDGES_FILTER => {
      if let Ok((src, filter)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        let v: Vec<u8> = filter;
        return Request::WriteNewEdgesFilter(src, v);
      }
    },
    CMD_FETCH_NEW_EDGES => {
      if let Ok((src, prefix)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::WriteFetchNewEdges(src, prefix);
      }
    },
    CMD_NODE_LIST => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::ReadNodeList;
      }
    },
    CMD_NODE_SCORE => {
      if let Ok((ego, target)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::ReadNodeScore(command.context.clone(), ego, target);
      }
    },
    CMD_SCORES => {
      if let Ok((ego, kind, hide_personal, lt, lte, gt, gte, index, count)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::ReadScores(
          command.context.clone(),
          ego,
          kind,
          hide_personal,
          lt,
          lte,
          gt,
          gte,
          index,
          count,
        );
      }
    },
    CMD_GRAPH => {
      if let Ok((ego, focus, positive_only, index, count)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::ReadGraph(
          command.context.clone(),
          ego,
          focus,
          positive_only,
          index,
          count,
        );
      }
    },
    CMD_CONNECTED => {
      if let Ok(node) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::ReadConnected(command.context.clone(), node);
      }
    },
    CMD_EDGES => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::ReadEdges(command.context.clone());
      }
    },
    CMD_MUTUAL_SCORES => {
      if let Ok(ego) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::ReadMutualScores(command.context.clone(), ego);
      }
    },
    CMD_READ_NEW_EDGES_FILTER => {
      if let Ok(src) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Request::ReadNewEdgesFilter(src);
      }
    },
    CMD_NEIGHBORS => {
      if let Ok((
        ego,
        focus,
        direction,
        kind,
        hide_personal,
        lt,
        lte,
        gt,
        gte,
        index,
        count,
      )) = rmp_serde::from_slice(command.payload.as_slice())
      {
        return Request::ReadNeighbors(
          command.context.clone(),
          ego,
          focus,
          direction,
          kind,
          hide_personal,
          lt,
          lte,
          gt,
          gte,
          index,
          count,
        );
      }
    },
    _ => {
      log_error!("Unexpected command: {:?}", command);
      return Request::None;
    },
  };

  log_error!("Invalid payload for command: {:?}", command);
  Request::None
}

fn encode_response_dispatch(
  response: Response
) -> Result<Vec<u8>, ServiceError> {
  match response {
    Response::NodeList(nodes) => encode_response(&nodes),
    Response::NewEdgesFilter(bytes) => encode_response(&bytes),
    Response::NodeScores(scores) => encode_response(&scores),
    Response::Graph(graph) => encode_response(&graph),
    Response::Connections(connections) => encode_response(&connections),
    Response::Edges(edges) => encode_response(&edges),
    Response::NewEdges(new_edges) => encode_response(&new_edges),
    _ => encode_response(&()),
  }
}

fn decode_and_handle_request(
  state: &mut InternalState,
  request: &[u8],
) -> Result<Vec<u8>, ServiceError> {
  log_trace!();

  let command = decode_request(request)?;

  log_verbose!("Decoded command: {:?}", command);

  if !command.context.is_empty()
    && (command.id == CMD_VERSION
      || command.id == CMD_LOG_LEVEL
      || command.id == CMD_RESET
      || command.id == CMD_RECALCULATE_ZERO
      || command.id == CMD_RECALCULATE_CLUSTERING
      || command.id == CMD_NODE_LIST
      || command.id == CMD_READ_NEW_EDGES_FILTER
      || command.id == CMD_WRITE_NEW_EDGES_FILTER
      || command.id == CMD_FETCH_NEW_EDGES)
  {
    let err_msg = "Context should be empty.".to_string();
    log_error!("{}", err_msg);
    return Err(ServiceError::Internal(err_msg));
  }

  //  Special commands
  match command.id.as_str() {
    CMD_VERSION => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return encode_response(&read_ops::read_version());
      }
      let err_msg = "Invalid payload.".to_string();
      log_error!("{}", err_msg);
      return Err(ServiceError::Internal(err_msg));
    },
    CMD_LOG_LEVEL => {
      if let Ok(log_level) = rmp_serde::from_slice(command.payload.as_slice()) {
        return encode_response(&write_ops::write_log_level(log_level));
      }
      let err_msg = "Invalid payload.".to_string();
      log_error!("{}", err_msg);
      return Err(ServiceError::Internal(err_msg));
    },
    CMD_SYNC => {
      sync(state);
      return encode_response(&());
    },
    _ => {},
  }

  let request = request_from_command(&command);

  if !command.blocking {
    let _ = queue(state, request); // Assuming queue handles its own errors or is infallible
    encode_response(&())
  } else {
    let begin = SystemTime::now();

    let response = perform(state, request); // Assuming perform handles its own errors or is infallible

    let duration = SystemTime::now().duration_since(begin).unwrap().as_secs();

    if duration > 5 {
      log_warning!("Command was done in {} seconds.", duration);
    }

    encode_response_dispatch(response)
  }
}

fn worker_callback(
  state: &mut InternalState,
  aio: Aio,
  ctx: &Context,
  res: AioResult,
) {
  log_trace!();

  match res {
    AioResult::Send(Ok(_)) => match ctx.recv(&aio) {
      Ok(_) => {},
      Err(error) => {
        log_error!("RECV failed: {}", error);
      },
    },

    AioResult::Recv(Ok(req)) => {
      let msg: Vec<u8> = decode_and_handle_request(state, req.as_slice())
        .unwrap_or_else(|_| {
          encode_response(&"Internal error, see server logs".to_string())
            .unwrap_or_else(|error| {
              log_error!("Unable to serialize error: {:?}", error);
              vec![]
            })
        });

      match ctx.send(&aio, msg.as_slice()) {
        Ok(_) => {},
        Err(error) => {
          log_error!("SEND failed: {:?}", error);
        },
      };
    },

    AioResult::Sleep(_) => {},

    AioResult::Send(Err(error)) => {
      log_error!("Async SEND failed: {:?}", error);
    },

    AioResult::Recv(Err(error)) => {
      log_error!("Async RECV failed: {:?}", error);
    },
  };
}

fn parse_env_var<T>(
  name: &str,
  min: T,
  max: T,
) -> Result<Option<T>, ServiceError>
where
  T: std::str::FromStr,
  T: std::cmp::Ord,
  T: std::fmt::Debug,
{
  match var(name) {
    Ok(s) => match s.parse::<T>() {
      Ok(n) => {
        if n >= min && n <= max {
          Ok(Some(n))
        } else {
          let err_msg = format!(
            "Invalid {}: {:?} not in range [{:?}, {:?}]",
            name, s, min, max
          );
          log_error!("{}", err_msg);
          Err(ServiceError::Internal(err_msg))
        }
      },
      _ => {
        let err_msg = format!("Invalid {}: {:?}", name, s);
        log_error!("{}", err_msg);
        Err(ServiceError::Internal(err_msg))
      },
    },
    _ => Ok(None),
  }
}

fn parse_and_set_value<T>(
  value: &mut T,
  name: &str,
  min: T,
  max: T,
) -> Result<(), ServiceError>
where
  T: std::str::FromStr,
  T: std::cmp::Ord,
  T: std::fmt::Debug,
{
  if let Some(n) = parse_env_var(name, min, max)? {
    *value = n
  }
  Ok(())
}

fn parse_and_set_bool(
  value: &mut bool,
  name: &str,
) -> Result<(), ServiceError> {
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
        let err_msg = format!(
          "Invalid {} (expected 0/1, true/false, yes/no): {:?}",
          name, s
        );
        log_error!("{}", err_msg);
        Err(ServiceError::Internal(err_msg))
      }
    },
    _ => Ok(()),
  }
}

pub fn parse_settings() -> Result<AugMultiGraphSettings, ServiceError> {
  let mut settings = AugMultiGraphSettings::default();

  //  TODO: Remove.
  if let Some(n) = parse_env_var("MERITRANK_NUM_WALK", 0, 1000000)? {
    log_warning!(
      "DEPRECATED: Use MERITRANK_NUM_WALKS instead of MERITRANK_NUM_WALK."
    );
    settings.num_walks = n;
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

  if let Some(n) = parse_env_var("MERITRANK_ZERO_OPINION_FACTOR", 0, 100)? {
    settings.zero_opinion_factor = (n as f64) * 0.01
  }

  static MIN_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(1).unwrap();
  static MAX_CACHE_SIZE: NonZeroUsize =
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

pub fn main_async() -> Result<(), ServiceError> {
  let threads = match var("MERITRANK_SERVICE_THREADS") {
    Ok(s) => match s.parse::<usize>() {
      Ok(n) => {
        if n > 0 {
          n
        } else {
          let err_msg = format!("Invalid MERITRANK_SERVICE_THREADS: {:?}", s);
          log_error!("{}", err_msg);
          return Err(ServiceError::Internal(err_msg));
        }
      },
      _ => {
        let err_msg = format!("Invalid MERITRANK_SERVICE_THREADS: {:?}", s);
        log_error!("{}", err_msg);
        return Err(ServiceError::Internal(err_msg));
      },
    },
    _ => 1,
  };

  let url = match var("MERITRANK_SERVICE_URL") {
    Ok(s) => s,
    _ => "tcp://127.0.0.1:10234".to_string(),
  };

  log_info!(
    "Starting server {} at {}, {} threads",
    VERSION,
    url,
    threads
  );

  let settings = parse_settings()?;

  log_info!("Num walks: {}", settings.num_walks);

  let state = init();

  let s = Socket::new(Protocol::Rep0)?;

  let workers: Vec<_> = (0..threads)
    .map(|_| {
      let ctx = Context::new(&s)?;
      let ctx_cloned = ctx.clone();
      let state_cloned = state.internal.clone();

      let aio = Aio::new(move |aio, res| {
        worker_callback(&mut state_cloned.clone(), aio, &ctx_cloned, res);
      })?;

      Ok((aio, ctx))
    })
    .collect::<Result<_, nng::Error>>()?;

  s.listen(&url)?;

  for (a, c) in &workers {
    c.recv(a)?;
  }

  std::thread::park();
  shutdown(state);

  Ok(())
}
