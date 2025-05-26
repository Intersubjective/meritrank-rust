use nng::{Aio, AioResult, Context, Protocol, Socket};
use std::{env::var, string::ToString, sync::atomic::Ordering};

use crate::aug_multi_graph::{AugMultiGraphSettings, Weight};
use crate::constants::*;
use crate::log::*;
use crate::operations::*;
use crate::protocol::{
    decode_request, encode_response, Command, Response, CMD_VERSION, CMD_LOG_LEVEL, CMD_SYNC,
    CMD_RESET, CMD_RECALCULATE_ZERO, CMD_RECALCULATE_CLUSTERING, CMD_NODE_LIST,
    CMD_READ_NEW_EDGES_FILTER, CMD_WRITE_NEW_EDGES_FILTER, CMD_FETCH_NEW_EDGES,
};
// Ensure state_manager::Response is aliased if protocol::Response is also in scope.
// Assuming state_manager::Response is the one used by encode_response_dispatch.
use crate::state_manager::{InternalState, Request, Response as StateManagerResponse, queue, perform, sync, init, shutdown};
use std::time::SystemTime;
use crate::request_parser::request_from_command; // Added this line

pub use meritrank_core::Weight;

fn encode_response_dispatch(response: StateManagerResponse) -> Result<Vec<u8>, ()> {
    match response {
        StateManagerResponse::NodeList(nodes) => encode_response(&nodes),
        StateManagerResponse::NewEdgesFilter(bytes) => encode_response(&bytes),
        StateManagerResponse::NodeScores(scores) => encode_response(&scores),
        StateManagerResponse::Graph(graph) => encode_response(&graph),
        StateManagerResponse::Connections(connections) => encode_response(&connections),
        StateManagerResponse::Edges(edges) => encode_response(&edges),
        StateManagerResponse::NewEdges(new_edges) => encode_response(&new_edges),
        _ => encode_response(&()), 
    }
}

fn decode_and_handle_request(
    state: &mut InternalState,
    request_bytes: &[u8], 
) -> Result<Vec<u8>, ()> {
    log_trace!();

    let command = decode_request(request_bytes)?;

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
        log_error!("Context should be empty.");
        return Err(());
    }

    match command.id.as_str() {
        CMD_VERSION => {
            if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
                return encode_response(&read_version());
            }
            log_error!("Invalid payload.");
            return Err(());
        },
        CMD_LOG_LEVEL => {
            if let Ok(log_level) = rmp_serde::from_slice(command.payload.as_slice()) {
                return encode_response(&write_log_level(log_level));
            }
            log_error!("Invalid payload.");
            return Err(());
        },
        CMD_SYNC => {
            sync(state);
            return encode_response(&());
        },
        _ => {},
    }

    let request_obj = request_from_command(&command); 

    if !command.blocking {
        let _ = queue(state, request_obj);
        encode_response(&())
    } else {
        let begin = SystemTime::now();
        let response = perform(state, request_obj);
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
            Err(error) => log_error!("RECV failed: {}", error),
        },
        AioResult::Recv(Ok(req)) => {
            let msg: Vec<u8> = match decode_and_handle_request(state, req.as_slice()) {
                Ok(bytes) => bytes,
                Err(_) => match encode_response(&"Internal error, see server logs".to_string()) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        log_error!("Unable to serialize error: {:?}", error);
                        vec![]
                    },
                },
            };
            match ctx.send(&aio, msg.as_slice()) {
                Ok(_) => {},
                Err(error) => log_error!("SEND failed: {:?}", error),
            };
        },
        AioResult::Sleep(_) => {},
        AioResult::Send(Err(error)) => log_error!("Async SEND failed: {:?}", error),
        AioResult::Recv(Err(error)) => log_error!("Async RECV failed: {:?}", error),
    };
}

pub fn main_async() -> Result<(), ()> {
    let threads = match var("MERITRANK_SERVICE_THREADS") {
        Ok(s) => match s.parse::<usize>() {
            Ok(n) => {
                if n > 0 { n } else {
                    log_error!("Invalid MERITRANK_SERVICE_THREADS: {:?}", s);
                    return Err(());
                }
            },
            _ => {
                log_error!("Invalid MERITRANK_SERVICE_THREADS: {:?}", s);
                return Err(());
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
        VERSION, url, threads
    );

    let settings = crate::settings_parser::parse_settings()?;

    log_info!("Num walks: {}", settings.num_walks);

    let state = init();
    let s = match Socket::new(Protocol::Rep0) {
        Ok(x) => x,
        Err(e) => {
            log_error!("{}", e);
            return Err(());
        },
    };

    let workers: Vec<_> = match (0..threads)
        .map(|_| {
            let ctx = Context::new(&s)?;
            let ctx_cloned = ctx.clone();
            let state_cloned = state.internal.clone();
            let aio = Aio::new(move |aio, res| {
                worker_callback(&mut state_cloned.clone(), aio, &ctx_cloned, res);
            })?;
            Ok((aio, ctx))
        })
        .collect::<Result<_, nng::Error>>()
    {
        Ok(x) => x,
        Err(e) => {
            log_error!("{}", e);
            return Err(());
        },
    };

    match s.listen(&url) {
        Err(e) => {
            log_error!("{}", e);
            return Err(());
        },
        _ => {},
    };

    for (a, c) in &workers {
        match c.recv(a) {
            Err(e) => {
                log_error!("{}", e);
                return Err(());
            },
            _ => {},
        };
    }

    std::thread::park();
    shutdown(state);

    Ok(())
}
