use nng::{Aio, AioResult, Context, Protocol, Socket};
use std::{
  env::var,
  ops::DerefMut,
  string::ToString,
  sync::atomic::Ordering,
  sync::{Arc, Condvar, Mutex},
};

use crate::log_error;
use crate::log_info;
use crate::log_warning;
// use crate::log_verbose;
use crate::log::*;
use crate::log_trace;
use crate::operations::*;
use crate::protocol::*;
use std::time::SystemTime;

pub use meritrank_core::Weight;

lazy_static::lazy_static! {
  pub static ref THREADS : usize =
    var("MERITRANK_SERVICE_THREADS")
      .ok()
      .and_then(|s| s.parse::<usize>().ok())
      .unwrap_or(1);

  static ref SERVICE_URL : String =
    var("MERITRANK_SERVICE_URL")
      .unwrap_or("tcp://127.0.0.1:10234".to_string());
}

pub struct Data {
  pub graph_readable: Mutex<AugMultiGraph>,
  pub graph_writable: Mutex<AugMultiGraph>,
  pub queue_commands: Mutex<Vec<Command>>,
  pub write_sync:     Mutex<()>,
  pub cond_add:       Condvar,
  pub cond_done:      Condvar,
}

fn perform_command(
  data: &Data,
  command: Command,
) -> Result<Vec<u8>, ()> {
  log_trace!("perform_command");

  if command.id == CMD_RESET
    || command.id == CMD_RECALCULATE_ZERO
    || command.id == CMD_DELETE_EDGE
    || command.id == CMD_DELETE_NODE
    || command.id == CMD_PUT_EDGE
    || command.id == CMD_CREATE_CONTEXT
    || command.id == CMD_WRITE_NEW_EDGES_FILTER
    || command.id == CMD_FETCH_NEW_EDGES
  {
    let mut res = encode_response(&());

    //  Write commands

    let mut graph = match data.graph_writable.lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("(perform_command) {}", e);
        return Err(());
      },
    };

    let mut ok = false;

    match command.id.as_str() {
      CMD_RESET => {
        if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
          ok = true;
          graph.write_reset();
        }
      },
      CMD_RECALCULATE_ZERO => {
        if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
          ok = true;
          graph.write_recalculate_zero();
        }
      },
      CMD_DELETE_EDGE => {
        if let Ok((src, dst)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          graph.write_delete_edge(command.context.as_str(), src, dst);
        }
      },
      CMD_DELETE_NODE => {
        if let Ok(node) = rmp_serde::from_slice(command.payload.as_slice()) {
          ok = true;
          graph.write_delete_node(command.context.as_str(), node);
        }
      },
      CMD_PUT_EDGE => {
        if let Ok((src, dst, amount)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          graph.write_put_edge(command.context.as_str(), src, dst, amount);
        }
      },
      CMD_CREATE_CONTEXT => {
        if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
          ok = true;
          graph.write_create_context(command.context.as_str());
        }
      },
      CMD_WRITE_NEW_EDGES_FILTER => {
        if let Ok((src, filter)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          let v: Vec<u8> = filter;
          graph.write_new_edges_filter(src, &v);
        }
      },
      CMD_FETCH_NEW_EDGES => {
        if let Ok((src, prefix)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          res = encode_response(&graph.write_fetch_new_edges(src, prefix));
        }
      },
      _ => {
        log_error!("(perform_command) Unexpected command `{}`", command.id);
      },
    };
    match data.graph_readable.lock() {
      Ok(ref mut x) => {
        x.copy_from(graph.deref_mut());
      },
      Err(e) => {
        log_error!("(perform_command) {}", e);
        return Err(());
      },
    };

    if ok {
      return res;
    }
  } else if command.id == CMD_SYNC {
    if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
      let mut queue = data.queue_commands.lock().expect("Mutex lock failed");

      while !queue.is_empty() {
        log_trace!("wait for queue to be empty");
        queue = data.cond_done.wait(queue).expect("Condvar wait failed");
      }

      let _write = data.write_sync.lock().expect("Mutex lock failed");

      return encode_response(&());
    }
  } else if command.id == CMD_VERSION {
    if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
      return encode_response(&read_version());
    }
  } else if command.id == CMD_LOG_LEVEL {
    if let Ok(log_level) = rmp_serde::from_slice(command.payload.as_slice()) {
      return encode_response(&write_log_level(log_level));
    }
  } else {
    //  Read commands

    let mut graph = match data.graph_readable.lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("(perform_command) {}", e);
        return Err(());
      },
    };
    match command.id.as_str() {
      CMD_NODE_LIST => {
        if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
          return encode_response(&graph.read_node_list());
        }
      },
      CMD_NODE_SCORE => {
        if let Ok((ego, target)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          return encode_response(&graph.read_node_score(
            command.context.as_str(),
            ego,
            target,
          ));
        }
      },
      CMD_SCORES => {
        if let Ok((ego, kind, hide_personal, lt, lte, gt, gte, index, count)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          return encode_response(&graph.read_scores(
            command.context.as_str(),
            ego,
            kind,
            hide_personal,
            lt,
            lte,
            gt,
            gte,
            index,
            count,
          ));
        }
      },
      CMD_GRAPH => {
        if let Ok((ego, focus, positive_only, index, count)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          return encode_response(&graph.read_graph(
            command.context.as_str(),
            ego,
            focus,
            positive_only,
            index,
            count,
          ));
        }
      },
      CMD_CONNECTED => {
        if let Ok(node) = rmp_serde::from_slice(command.payload.as_slice()) {
          return encode_response(
            &graph.read_connected(command.context.as_str(), node),
          );
        }
      },
      CMD_EDGES => {
        if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
          return encode_response(&graph.read_edges(command.context.as_str()));
        }
      },
      CMD_MUTUAL_SCORES => {
        if let Ok(ego) = rmp_serde::from_slice(command.payload.as_slice()) {
          return encode_response(
            &graph.read_mutual_scores(command.context.as_str(), ego),
          );
        }
      },
      CMD_READ_NEW_EDGES_FILTER => {
        if let Ok(src) = rmp_serde::from_slice(command.payload.as_slice()) {
          return encode_response(&graph.read_new_edges_filter(src));
        }
      },
      _ => {
        log_error!("(perform_command) Unknown command: `{}`", command.id);
        return Err(());
      },
    }
  }

  log_error!(
    "(perform_command) Invalid payload for command `{}`: {:?}",
    command.id.as_str(),
    command.payload
  );
  Err(())
}

fn command_queue_thread(data: &Data) {
  let mut queue = data.queue_commands.lock().expect("Mutex lock failed");
  loop {
    log_trace!("command_queue_thread (loop)");

    let write = data.write_sync.lock().expect("Mutex lock failed");

    let commands: Vec<_> = queue.clone();
    queue.clear();
    std::mem::drop(queue);

    for cmd in commands {
      let begin = SystemTime::now();
      let _ = perform_command(&data, cmd.clone());
      let duration = SystemTime::now().duration_since(begin).unwrap().as_secs();

      log_trace!("perform_command - done");
      if duration > 5 {
        log_warning!("Command was done in {} seconds", duration);
      }
    }

    std::mem::drop(write);

    queue = data.queue_commands.lock().expect("Mutex lock failed");
    if queue.is_empty() {
      log_trace!("notify done");
      data.cond_done.notify_all();

      queue = data.cond_add.wait(queue).expect("Condvar wait failed");
    }
  }
}

fn put_for_write(
  data: &Data,
  command: Command,
) {
  log_trace!("put_for_write");

  let mut queue = data.queue_commands.lock().expect("Mutex lock failed");
  queue.push(command);
  log_trace!("notify add");
  data.cond_add.notify_one();
}

fn decode_and_handle_request(
  data: &Data,
  request: &[u8],
) -> Result<Vec<u8>, ()> {
  log_trace!("decode_and_handle_request");

  let command = decode_request(request)?;

  if command.context.is_empty() {
    log_trace!(
      "decoded command `{}` in NULL with payload {:?}",
      command.id,
      command.payload
    );
  } else {
    log_trace!(
      "decoded command `{}` in `{}` with payload {:?}",
      command.id,
      command.context,
      command.payload
    );
  }

  if !command.context.is_empty()
    && (command.id == CMD_VERSION
      || command.id == CMD_LOG_LEVEL
      || command.id == CMD_RESET
      || command.id == CMD_RECALCULATE_ZERO
      || command.id == CMD_NODE_LIST
      || command.id == CMD_READ_NEW_EDGES_FILTER
      || command.id == CMD_WRITE_NEW_EDGES_FILTER
      || command.id == CMD_FETCH_NEW_EDGES)
  {
    log_error!("(decode_and_handle_request) Context should be empty");
    return Err(());
  }

  if !command.blocking {
    put_for_write(&data, command);
    encode_response(&())
  } else {
    let begin = SystemTime::now();
    let res = perform_command(&data, command);
    let duration = SystemTime::now().duration_since(begin).unwrap().as_secs();

    log_trace!("perform_command - done");
    if duration > 5 {
      log_warning!("Command was done in {} seconds", duration);
    }

    res
  }
}

fn worker_callback(
  data: &Data,
  aio: Aio,
  ctx: &Context,
  res: AioResult,
) {
  log_trace!("worker_callback");

  match res {
    AioResult::Send(Ok(_)) => match ctx.recv(&aio) {
      Ok(_) => {},
      Err(error) => {
        log_error!("(worker_callback) RECV failed: {}", error);
      },
    },

    AioResult::Recv(Ok(req)) => {
      let msg: Vec<u8> = match decode_and_handle_request(data, req.as_slice()) {
        Ok(bytes) => bytes,
        Err(_) => {
          match encode_response(&"Internal error, see server logs".to_string())
          {
            Ok(bytes) => bytes,
            Err(error) => {
              log_error!(
                "(worker_callback) Unable to serialize error: {:?}",
                error
              );
              vec![]
            },
          }
        },
      };
      log_trace!("decode_and_handle_request - done");
      match ctx.send(&aio, msg.as_slice()) {
        Ok(_) => {},
        Err(error) => {
          log_error!("(worker_callback) SEND failed: {:?}", error);
        },
      };
    },

    AioResult::Sleep(_) => {},

    AioResult::Send(Err(error)) => {
      log_error!("(worker_callback) Async SEND failed: {:?}", error);
    },

    AioResult::Recv(Err(error)) => {
      log_error!("(worker_callback) Async RECV failed: {:?}", error);
    },
  };
}

pub fn main_async(threads: usize) -> Result<(), ()> {
  let threads = if threads < 1 { 1 } else { threads };

  log_info!(
    "Starting server {} at {}, {} threads",
    VERSION,
    *SERVICE_URL,
    threads
  );
  log_info!("NUM_WALK={}", *NUM_WALK);

  let data = Arc::<Data>::new(Data {
    graph_readable: Mutex::<AugMultiGraph>::new(AugMultiGraph::new()),
    graph_writable: Mutex::<AugMultiGraph>::new(AugMultiGraph::new()),
    queue_commands: Mutex::<Vec<Command>>::new(vec![]),
    write_sync:     Mutex::<()>::new(()),
    cond_add:       Condvar::new(),
    cond_done:      Condvar::new(),
  });

  let data_cloned = data.clone();

  std::thread::spawn(move || {
    command_queue_thread(&data_cloned);
  });

  let s = match Socket::new(Protocol::Rep0) {
    Ok(x) => x,
    Err(e) => {
      log_error!("(main_async) {}", e);
      return Err(());
    },
  };

  let workers: Vec<_> = match (0..threads)
    .map(|_| {
      let ctx = Context::new(&s)?;
      let ctx_cloned = ctx.clone();
      let data_cloned = data.clone();

      let aio = Aio::new(move |aio, res| {
        worker_callback(&data_cloned.clone(), aio, &ctx_cloned, res);
      })?;

      Ok((aio, ctx))
    })
    .collect::<Result<_, nng::Error>>()
  {
    Ok(x) => x,
    Err(e) => {
      log_error!("(main_async) {}", e);
      return Err(());
    },
  };

  match s.listen(&SERVICE_URL) {
    Err(e) => {
      log_error!("(main_async) {}", e);
      return Err(());
    },
    _ => {},
  };

  for (a, c) in &workers {
    match c.recv(a) {
      Err(e) => {
        log_error!("(main_async) {}", e);
        return Err(());
      },
      _ => {},
    };
  }

  std::thread::park();
  Ok(())
}
