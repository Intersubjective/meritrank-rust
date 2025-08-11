//  NOTE: Don't bother to clean this up.
//        This code will become obsolete after we stop using NNG.

use crate::data::*;
use crate::legacy_protocol::*;
use crate::settings::*;
use crate::state_manager::MultiGraphProcessor;
use crate::utils::log::*;

use nng::{Aio, AioResult, Context, Protocol, Socket};
use std::{
  collections::HashSet,
  string::ToString,
  sync::{atomic::Ordering, Arc, Mutex},
};

pub struct LegacyServer {
  pub _socket:  Socket,
  pub _workers: Vec<(Aio, Context)>,
}

fn kind_from_prefix(prefix: NodeName) -> Option<NodeKind> {
  if prefix.starts_with("U") {
    Some(NodeKind::User)
  } else if prefix.starts_with("B") {
    Some(NodeKind::Beacon)
  } else if prefix.starts_with("C") {
    Some(NodeKind::Comment)
  } else if prefix.starts_with("O") {
    Some(NodeKind::Opinion)
  } else if prefix.starts_with("V") {
    Some(NodeKind::PollVariant)
  } else if prefix.starts_with("P") {
    Some(NodeKind::Poll)
  } else {
    None
  }
}

fn request_from_command(
  command: &Command,
  stamp: &mut u64,
) -> Option<Request> {
  log_trace!();

  match command.id.as_str() {
    CMD_SYNC => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        *stamp += 1;

        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::Sync(*stamp),
        });
      }
    },
    CMD_RESET => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::WriteReset,
        });
      }
    },
    CMD_RECALCULATE_ZERO => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::WriteRecalculateZero,
        });
      }
    },
    CMD_SET_ZERO_OPINION => {
      if let Ok((node, score)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::WriteZeroOpinion(OpWriteZeroOpinion {
            node,
            score,
          }),
        });
      }
    },
    CMD_RECALCULATE_CLUSTERING => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::WriteRecalculateClustering,
        });
      }
    },
    CMD_DELETE_EDGE => {
      if let Ok((src, dst, index)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::WriteDeleteEdge(OpWriteDeleteEdge {
            src,
            dst,
            index,
          }),
        });
      }
    },
    CMD_DELETE_NODE => {
      if let Ok((node, index)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::WriteDeleteNode(OpWriteDeleteNode {
            node,
            index,
          }),
        });
      }
    },
    CMD_PUT_EDGE => {
      if let Ok((src, dst, amount, index)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        let magnitude: i64 = index;
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::WriteEdge(OpWriteEdge {
            src,
            dst,
            amount,
            magnitude: magnitude as u32,
          }),
        });
      }
    },
    CMD_CREATE_CONTEXT => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::WriteCreateContext,
        });
      }
    },
    CMD_WRITE_NEW_EDGES_FILTER => {
      if let Ok((src, filter)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        let v: Vec<u8> = filter;
        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::WriteNewEdgesFilter(OpWriteNewEdgesFilter {
            src,
            filter: v,
          }),
        });
      }
    },
    CMD_FETCH_NEW_EDGES => {
      if let Ok((src, prefix)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::WriteFetchNewEdges(OpWriteFetchNewEdges {
            src,
            prefix,
          }),
        });
      }
    },
    CMD_NODE_LIST => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::ReadNodeList,
        });
      }
    },
    CMD_NODE_SCORE => {
      if let Ok((ego, target)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::ReadNodeScore(OpReadNodeScore {
            ego,
            target,
          }),
        });
      }
    },
    CMD_SCORES => {
      if let Ok((ego, kind, hide_personal, lt, lte, gt, gte, index, count)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::ReadScores(OpReadScores {
            ego,
            score_options: FilterOptions {
              node_kind: kind_from_prefix(kind),
              hide_personal,
              score_lt: lt,
              score_lte: lte,
              score_gt: gt,
              score_gte: gte,
              index,
              count,
            },
          }),
        });
      }
    },
    CMD_GRAPH => {
      if let Ok((ego, focus, positive_only, index, count)) =
        rmp_serde::from_slice(command.payload.as_slice())
      {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::ReadGraph(OpReadGraph {
            ego,
            focus,
            positive_only,
            index,
            count,
          }),
        });
      }
    },
    CMD_CONNECTED => {
      if let Ok(node) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::ReadConnected(OpReadConnected {
            node,
          }),
        });
      }
    },
    CMD_EDGES => {
      if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::ReadEdges,
        });
      }
    },
    CMD_MUTUAL_SCORES => {
      if let Ok(ego) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::ReadMutualScores(OpReadMutualScores {
            ego,
          }),
        });
      }
    },
    CMD_READ_NEW_EDGES_FILTER => {
      if let Ok(src) = rmp_serde::from_slice(command.payload.as_slice()) {
        return Some(Request {
          subgraph: String::default(),
          data:     ReqData::ReadNewEdgesFilter(OpReadNewEdgesFilter {
            src,
          }),
        });
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
        return Some(Request {
          subgraph: command.context.clone(),
          data:     ReqData::ReadNeighbors(OpReadNeighbors {
            ego,
            focus,
            direction,
            kind: kind_from_prefix(kind),
            hide_personal,
            lt,
            lte,
            gt,
            gte,
            index,
            count,
          }),
        });
      }
    },
    _ => {
      log_error!("Unexpected command: {:?}", command);
      return None;
    },
  };

  log_error!("Invalid payload for command: {:?}", command);
  None
}

fn encode_response_dispatch(
  response: Response
) -> Result<Vec<u8>, ServiceError> {
  log_trace!("{:?}", response);

  match response {
    Response::NodeList(nodes) => encode_response(&nodes.nodes),
    Response::NewEdgesFilter(bytes) => encode_response(&bytes.bytes),
    Response::Scores(scores) => encode_response(&scores.scores),
    Response::Graph(graph) => encode_response(&graph.graph),
    Response::Connections(connections) => {
      encode_response(&connections.connections)
    },
    Response::Edges(edges) => encode_response(&edges.edges),
    Response::NewEdges(new_edges) => encode_response(&new_edges.new_edges),
    _ => encode_response(&()),
  }
}

struct InternalState {
  node_names: HashSet<String>,
  stamp:      u64,
}

fn decode_and_handle_request(
  state: Arc<MultiGraphProcessor>,
  request: &[u8],
  internal: &mut InternalState,
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
        //  FIXME: Get version from build env.
        return encode_response(&"0.3.0".to_string());
      }
      let err_msg = "Invalid payload.".to_string();
      log_error!("{}", err_msg);
      return Err(ServiceError::Internal(err_msg));
    },
    CMD_LOG_LEVEL => {
      if let Ok(log_level) = rmp_serde::from_slice(command.payload.as_slice()) {
        // NOTE: Log level ignored.
        let _: u32 = log_level; // For type annotation.
        return encode_response(&());
      }
      let err_msg = "Invalid payload.".to_string();
      log_error!("{}", err_msg);
      return Err(ServiceError::Internal(err_msg));
    },
    CMD_RESET => {
      internal.node_names.clear();
      // Don't return yet, we have to process reset on the state layer.
    },
    _ => {},
  }

  let request = request_from_command(&command, &mut internal.stamp);

  //  NOTE: command.blocking ignored.

  if let Some(request_data) = request {
    if let ReqData::WriteEdge(ref write_edge) = request_data.data {
      let subgraph = request_data.subgraph.clone();
      let src = write_edge.src.clone();
      let dst = write_edge.dst.clone();

      //  FIXME: Refactor code duplication.

      if !internal.node_names.contains(&src) {
        let _ = state.process_request_blocking(&Request {
          subgraph: subgraph.clone(),
          data:     ReqData::WriteCalculate(OpWriteCalculate {
            ego: src.clone(),
          }),
        });

        internal.node_names.insert(src);
      }

      if !internal.node_names.contains(&dst) {
        let _ = state.process_request_blocking(&Request {
          subgraph,
          data: ReqData::WriteCalculate(OpWriteCalculate {
            ego: dst.clone(),
          }),
        });

        internal.node_names.insert(dst);
      }
    }

    let response = state.process_request_blocking(&request_data);
    return encode_response_dispatch(response);
  } else {
    return Err(ServiceError::Internal("Process failed.".into()));
  }
}

fn worker_callback(
  state: Arc<MultiGraphProcessor>,
  internal: Arc<Mutex<InternalState>>,
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
      let mut internal_ref = match internal.lock() {
        Ok(x) => x,
        Err(e) => {
          log_error!("Mutex lock failed: {}", e);
          return;
        },
      };

      let msg: Vec<u8> =
        decode_and_handle_request(state, req.as_slice(), &mut internal_ref)
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

    AioResult::Send(Err(_error)) => {
      // log_error!("Async SEND failed: {:?}", error);
    },

    AioResult::Recv(Err(_error)) => {
      // log_error!("Async RECV failed: {:?}", error);
    },
  };
}

pub fn run(
  settings: Settings,
  state: Arc<MultiGraphProcessor>,
) -> Result<LegacyServer, ServiceError> {
  log_trace!();

  let internal = Arc::new(Mutex::new(InternalState {
    node_names: HashSet::<String>::new(),
    stamp:      0,
  }));

  let url = format!(
    "tcp://{}:{}",
    settings.server_address, settings.legacy_server_port
  );

  log_verbose!(
    "Starting {} NNG workers on {}",
    settings.legacy_server_num_threads,
    url
  );

  // Request/Reply NNG protocol.
  let nng_socket = Socket::new(Protocol::Rep0)?;

  let workers: Vec<_> = (0..settings.legacy_server_num_threads)
    .map(|_| {
      let ctx = Context::new(&nng_socket)?;
      let ctx_cloned = ctx.clone();
      let state_cloned = Arc::clone(&state);
      let internal_cloned = internal.clone();
      let aio = Aio::new(move |aio, res| {
        let ctx_cloned2 = ctx_cloned.clone();
        let state_cloned2 = Arc::clone(&state_cloned);
        let internal_cloned2 = internal_cloned.clone();
        worker_callback(
          state_cloned2,
          internal_cloned2,
          aio,
          &ctx_cloned2,
          res,
        );
      })?;
      Ok((aio, ctx))
    })
    .collect::<Result<_, nng::Error>>()?;

  nng_socket.listen(&url)?;

  for (a, c) in &workers {
    c.recv(a)?;
  }

  Ok(LegacyServer {
    _socket:  nng_socket,
    _workers: workers,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use nng::options::Options;
  use tokio::time::{sleep, Duration};

  #[tokio::test]
  async fn nng_request_response() {
    let settings = Settings {
      legacy_server_port: 8041,
      sleep_duration_after_publish_ms: 0,
      ..Settings::default()
    };

    let _server = run(
      settings.clone(),
      Arc::new(MultiGraphProcessor::new(settings)),
    )
    .unwrap();

    sleep(Duration::from_millis(20)).await;

    let do_request = |payload| {
      let client = nng::Socket::new(nng::Protocol::Req0).unwrap();
      client
        .set_opt::<nng::options::RecvTimeout>(Some(Duration::from_millis(1000)))
        .unwrap();
      client
        .set_opt::<nng::options::SendTimeout>(Some(Duration::from_millis(1000)))
        .unwrap();
      client.dial("tcp://127.0.0.1:8041").unwrap();
      client.send(nng::Message::from(payload)).unwrap();
      client.recv().unwrap()
    };

    let weight: f64 = 1.0;
    let magnitude: i64 = 1;

    let req = encode_request(&Command {
      id:       CMD_PUT_EDGE.into(),
      context:  "".into(),
      blocking: false,
      payload:  rmp_serde::to_vec(&("U1", "U2", weight, magnitude)).unwrap(),
    })
    .unwrap();

    let _ = do_request(req.as_slice());

    let req = encode_request(&Command {
      id:       CMD_SYNC.into(),
      context:  "".into(),
      blocking: true,
      payload:  rmp_serde::to_vec(&()).unwrap(),
    })
    .unwrap();

    let _ = do_request(req.as_slice());

    let response = do_request(
      encode_request(&Command {
        id:       CMD_SCORES.into(),
        context:  "".into(),
        blocking: true,
        payload:  rmp_serde::to_vec(&(
          "U1", "", false, 100.0, false, -100.0, false, 0i64, 100i64,
        ))
        .unwrap(),
      })
      .unwrap()
      .as_slice(),
    );

    let scores: Vec<(String, String, f64, f64, i32, i32)> =
      decode_response(response.as_slice()).unwrap();

    assert!(scores.len() == 2);
    assert!(scores[0].2 > 0.35);
    assert!(scores[0].2 < 0.50);
    assert!(scores[1].2 > 0.25);
    assert!(scores[1].2 < 0.45);
  }
}
