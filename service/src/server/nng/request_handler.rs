use nng::{Aio, AioResult, Context, Protocol, Socket};
use std::{env::var, string::ToString, sync::atomic::Ordering};

use crate::utils::constants::*;
use crate::utils::errors::ServiceError;
use crate::utils::log::*;
use self::protocol::*;
use self::read_ops;
use crate::utils::settings::parse_settings;
use self::state_manager::*;
use self::write_ops;
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

fn nng_task(
  state: InternalState,
  url: &str,
  num_workers: usize,
) -> Result<(), ServiceError> {
  //  NOTE: Don't bother to clean this up.
  //        This code will become obsolete after we stop using NNG.

  log_info!("Starting {} NNG workers.", num_workers);

  // Request/Reply NNG protocol.
  let nng_socket = Socket::new(Protocol::Rep0)?;

  let workers: Vec<_> = (0..num_workers)
    .map(|_| {
      let ctx = Context::new(&nng_socket)?;
      let ctx_cloned = ctx.clone();
      let state_cloned = state.clone();
      let aio = Aio::new(move |aio, res| {
        worker_callback(&mut state_cloned.clone(), aio, &ctx_cloned, res);
      })?;
      Ok((aio, ctx))
    })
    .collect::<Result<_, nng::Error>>()?;

  nng_socket.listen(url)?;

  for (a, c) in &workers {
    c.recv(a)?;
  }

  //  This should never return.
  std::thread::park();
  Ok(())
}

pub async fn run() -> Result<(), ServiceError> {
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

  log_info!("Starting server {} at {}", VERSION, url);

  let settings = parse_settings()?;

  log_info!("Num walks: {}", settings.num_walks);

  let state = init();

  //  Wrap NNG inside a tokio task.

  let state_cloned = state.internal.clone();

  match tokio::spawn(async move {
    match nng_task(state_cloned, &url, threads) {
      _ => {},
    }
  })
  .await
  {
    _ => {},
  }

  Ok(())
}
