use nng::{Aio, AioResult, Context, Protocol, Socket};
use std::{
  env::var,
  ops::DerefMut,
  string::ToString,
  sync::atomic::Ordering,
  sync::{Arc, Condvar, Mutex},
};

use crate::aug_multi_graph::*;
use crate::constants::*;
use crate::log::*;
use crate::operations::*;
use crate::protocol::*;
use std::time::SystemTime;

pub use meritrank_core::Weight;

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
  log_trace!();

  if command.id == CMD_RESET
    || command.id == CMD_RECALCULATE_ZERO
    || command.id == CMD_RECALCULATE_CLUSTERING
    || command.id == CMD_DELETE_EDGE
    || command.id == CMD_DELETE_NODE
    || command.id == CMD_PUT_EDGE
    || command.id == CMD_CREATE_CONTEXT
    || command.id == CMD_WRITE_NEW_EDGES_FILTER
    || command.id == CMD_FETCH_NEW_EDGES
    || command.id == CMD_SET_ZERO_OPINION
  {
    let mut res = encode_response(&());

    //  Write commands

    let mut graph = match data.graph_writable.lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
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
      CMD_SET_ZERO_OPINION => {
        if let Ok((node, score)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          graph.write_set_zero_opinion(command.context.as_str(), node, score);
        }
      },
      CMD_RECALCULATE_CLUSTERING => {
        if let Ok(()) = rmp_serde::from_slice(command.payload.as_slice()) {
          ok = true;
          graph.write_recalculate_clustering();
        }
      },
      CMD_DELETE_EDGE => {
        if let Ok((src, dst, index)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          graph.write_delete_edge(command.context.as_str(), src, dst, index);
        }
      },
      CMD_DELETE_NODE => {
        if let Ok((node, index)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          graph.write_delete_node(command.context.as_str(), node, index);
        }
      },
      CMD_PUT_EDGE => {
        if let Ok((src, dst, amount, index)) =
          rmp_serde::from_slice(command.payload.as_slice())
        {
          ok = true;
          graph.write_put_edge(
            command.context.as_str(),
            src,
            dst,
            amount,
            index,
          );
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
        log_error!("Unexpected command `{}`", command.id);
      },
    };
    match data.graph_readable.lock() {
      Ok(ref mut x) => {
        x.copy_from(graph.deref_mut());
      },
      Err(e) => {
        log_error!("{}", e);
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
        log_error!("{}", e);
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
          return encode_response(&graph.read_neighbors(
            command.context.as_str(),
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
          ));
        }
      },
      _ => {
        log_error!("Unknown command: {:?}", command.id);
        return Err(());
      },
    }
  }

  log_error!(
    "Invalid payload for command {:?}: {:?}",
    command.id,
    command.payload
  );
  Err(())
}

fn command_queue_thread(data: &Data) {
  log_trace!();

  let mut queue = data.queue_commands.lock().expect("Mutex lock failed");
  loop {
    log_trace!("Loop");

    let write = data.write_sync.lock().expect("Mutex lock failed");

    let commands: Vec<_> = queue.clone();
    queue.clear();
    std::mem::drop(queue);

    for cmd in commands {
      let begin = SystemTime::now();
      let _ = perform_command(&data, cmd.clone());
      let duration = SystemTime::now().duration_since(begin).unwrap().as_secs();

      if duration > 5 {
        log_warning!("Command was done in {} seconds", duration);
      }
    }

    std::mem::drop(write);

    queue = data.queue_commands.lock().expect("Mutex lock failed");
    if queue.is_empty() {
      data.cond_done.notify_all();

      queue = data.cond_add.wait(queue).expect("Condvar wait failed");
    }
  }
}

fn put_for_write(
  data: &Data,
  command: Command,
) {
  log_trace!();

  let mut queue = data.queue_commands.lock().expect("Mutex lock failed");
  queue.push(command);

  data.cond_add.notify_one();
}

fn decode_and_handle_request(
  data: &Data,
  request: &[u8],
) -> Result<Vec<u8>, ()> {
  log_trace!();

  let command = decode_request(request)?;

  log_verbose!(
    "Decoded command `{}` in {:?}, blocking {:?}, with payload {:?}",
    command.id,
    command.context,
    command.blocking,
    command.payload
  );

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
    log_error!("Context should be empty");
    return Err(());
  }

  if !command.blocking {
    put_for_write(&data, command);
    encode_response(&())
  } else {
    let begin = SystemTime::now();
    let res = perform_command(&data, command);
    let duration = SystemTime::now().duration_since(begin).unwrap().as_secs();

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
  log_trace!();

  match res {
    AioResult::Send(Ok(_)) => match ctx.recv(&aio) {
      Ok(_) => {},
      Err(error) => {
        log_error!("RECV failed: {}", error);
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
              log_error!("Unable to serialize error: {:?}", error);
              vec![]
            },
          }
        },
      };

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

fn parse_and_set_value<T>(
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

  Ok(settings)
}

pub fn main_async() -> Result<(), ()> {
  let threads = match var("MERITRANK_SERVICE_THREADS") {
    Ok(s) => match s.parse::<usize>() {
      Ok(n) => {
        if n > 0 {
          n
        } else {
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
    VERSION,
    url,
    threads
  );

  let settings = parse_settings()?;

  log_info!("Num walks: {}", settings.num_walks);

  let data = Arc::<Data>::new(Data {
    graph_readable: Mutex::<AugMultiGraph>::new(AugMultiGraph::new(
      settings.clone(),
    )),
    graph_writable: Mutex::<AugMultiGraph>::new(AugMultiGraph::new(settings)),
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
      log_error!("{}", e);
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
  Ok(())
}
