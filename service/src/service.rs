//  ================================================================
//
//  Example of a redis_fdw interaction with the service
//
//  INSERT is used to request a write command.
//  SELECT is used to request a read command.
//
//          redis_fdw |  MeritRank Service
//                    |  emulating Redis Protocol
//
//                  INSERT
//
//  ------------------+---------------------------------------------
//    *2              |
//    $4              |
//    AUTH            |
//    $6              |
//    secret          |
//  ------------------+---------------------------------------------
//                    |   +OK
//  ------------------+---------------------------------------------
//    *2              |
//    $6              |
//    SELECT          |
//    $1              |
//    0               |
//  ------------------+---------------------------------------------
//                    |   +OK
//  ------------------+---------------------------------------------
//    *2              |
//    $6              |
//    EXISTS          |
//    $7              |
//    bar_key         |
//  ------------------+---------------------------------------------
//                    |   :0
//  ------------------+---------------------------------------------
//    *4              |
//    $4              |
//    HSET            |
//    $7              |
//    bar_key         |
//    $5              |
//    name1           |
//    $4              |
//    val1            |
//  ------------------+---------------------------------------------
//                    |   +OK
//  ------------------+---------------------------------------------
//    *4              |
//    $4              |
//    HSET            |
//    $7              |
//    bar_key         |
//    $5              |
//    name2           |
//    $4              |
//    val2            |
//  ------------------+---------------------------------------------
//                    |   +OK
//  ------------------+---------------------------------------------
//
//  ================================================================

use std::env::var;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::ErrorKind;
use std::net::Shutdown;
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crate::log::*;
use crate::operations::*;
use crate::VERSION;

//  FIXME: Make this into configurable settings.
pub const READ_TIMEOUT_SEC: u64 = 10;
pub const WRITE_TIMEOUT_SEC: u64 = 10;
pub const ACCEPT_DELAY_MSEC: u64 = 10;

pub struct Service {
  pub local_port:     u16,
  pub done:           Arc<AtomicBool>,
  pub graph_readable: Arc<Mutex<AugMultiGraph>>,
  pub graph_writable: Arc<Mutex<AugMultiGraph>>,
  pub threads:        Vec<thread::JoinHandle<()>>,
}

#[derive(PartialEq, Debug)]
pub enum Request {
  Unknown,
  ReadVersion,
  WritePutEdge(Option<String>, Option<String>, Option<Weight>),
}

pub const READ_VERSION: &str = "VERSION";
pub const READ_NODE_SCORE: &str = "NODE_SCORE";
pub const READ_SCORES: &str = "SCORES";
pub const READ_GRAPH: &str = "GRAPH";
pub const READ_CONNECTED: &str = "CONNECTED";
pub const READ_NODE_LIST: &str = "NODE_LIST";
pub const READ_EDGES: &str = "EDGES";
pub const READ_MUTUAL_SCORES: &str = "MUTUAL_SCORES";
pub const READ_NEW_EDGES_FILTER: &str = "NEW_EDGES_FILTER";
pub const WRITE_LOG_LEVEL: &str = "LOG_LEVEL";
pub const WRITE_CREATE_CONTEXT: &str = "CREATE_CONTEXT";
pub const WRITE_PUT_EDGE: &str = "PUT_EDGE";
pub const WRITE_DELETE_EDGE: &str = "DELETE_EDGE";
pub const WRITE_DELETE_NODE: &str = "DELETE_NODE";
pub const WRITE_RESET: &str = "RESET";
pub const WRITE_NEW_EDGES_FILTER: &str = "NEW_EDGES_FILTER";
pub const WRITE_FETCH_NEW_EDGES: &str = "FETCH_NEW_EDGES";
pub const WRITE_RECALCULATE_ZERO: &str = "RECALCULATE_ZERO";

#[derive(Debug)]
pub enum Response {
  Ok,
  Error(String),
  String(String),
}

pub enum RequestMode {
  Read,
  Write,
}

pub struct RequestRaw {
  pub mode:          RequestMode,
  pub id:            Option<String>,
  pub blocking:      Option<bool>,
  pub context:       Option<String>,
  pub node:          Option<String>,
  pub ego:           Option<String>,
  pub src:           Option<String>,
  pub dst:           Option<String>,
  pub focus:         Option<String>,
  pub amount:        Option<Weight>,
  pub magnitude:     Option<i64>,
  pub prefix:        Option<String>,
  pub hide_personal: Option<bool>,
  pub positive_only: Option<bool>,
  pub lt:            Option<Weight>,
  pub lte:           Option<Weight>,
  pub gt:            Option<Weight>,
  pub gte:           Option<Weight>,
  pub page_offset:   Option<i64>,
  pub page_size:     Option<i64>,
  pub bloom_filter:  Option<Vec<u8>>,
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
  log_trace!();

  let mut settings = AugMultiGraphSettings::default();

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

  Ok(settings)
}

fn response_error(message: &str) -> Response {
  log_error!("{}", message);
  Response::Error(format!("ERROR {}", message))
}

pub fn perform_request(req: Request) -> Response {
  log_trace!("{:?}", req);

  match req {
    Request::ReadVersion => Response::String(VERSION.to_string()),
    Request::WritePutEdge(Some(_src), Some(_dst), Some(_amount)) => {
      Response::Error("Not implemented".to_string())
    },
    Request::WritePutEdge(_, _, _) => response_error(&format!(
      "Invalid arguments for {:?} request",
      WRITE_PUT_EDGE
    )),
    _ => Response::Ok,
  }
}

pub fn encode_response(res: Response) -> String {
  log_trace!("{:?}", res);

  match res {
    Response::Ok => "+OK\r\n".to_string(),
    Response::Error(e) => format!("-{}\r\n", e),
    Response::String(s) => format!("+{}\r\n", s),
  }
}

pub fn parse_string(chunk: &[String]) -> Result<(String, usize), ()> {
  log_trace!("{:?}", chunk);

  if chunk.is_empty() {
    return Err(());
  }
  let c = match chunk[0].chars().nth(0) {
    Some(x) => x,
    _ => return Err(()),
  };
  if c == '+' {
    return Ok((chunk[0][1..chunk[0].len() - 2].to_string(), 1));
  }
  if c == '$' && chunk.len() >= 2 {
    let n = match chunk[0][1..chunk[0].len() - 2].parse::<i64>() {
      Ok(x) => x,
      _ => return Err(()),
    };
    if n < 0 {
      return Ok(("".to_string(), 1));
    }
    let s = chunk[1][0..chunk[1].len() - 2].to_string();
    if s.len() != n as usize {
      return Err(());
    }
    return Ok((s, 2));
  }
  return Err(());
}

pub fn parse_request_raw(chunk: &[String]) -> RequestRaw {
  log_trace!();

  let mut req = RequestRaw {
    mode:          RequestMode::Read,
    id:            None,
    blocking:      None,
    context:       None,
    node:          None,
    ego:           None,
    src:           None,
    dst:           None,
    focus:         None,
    amount:        None,
    magnitude:     None,
    prefix:        None,
    hide_personal: None,
    positive_only: None,
    lt:            None,
    lte:           None,
    gt:            None,
    gte:           None,
    page_offset:   None,
    page_size:     None,
    bloom_filter:  None,
  };

  if chunk.len() < 5 {
    return req;
  }
  let c = match chunk[0].chars().nth(0) {
    Some(x) => x,
    _ => return req,
  };
  if c != '*' {
    return req;
  }

  if chunk[1] == "$3\r\n" && chunk[2] == "GET\r\n" {
    match parse_string(&chunk[3..chunk.len()]) {
      Ok((s, _)) => req.id = Some(s),
      _ => {},
    }
  }

  if chunk[1] == "$3\r\n" && chunk[2] == "SET\r\n" {
    match parse_string(&chunk[3..chunk.len()]) {
      Ok((s, _)) => {
        req.mode = RequestMode::Write;
        req.id = Some(s);
      },
      _ => {},
    }
  }

  //  TODO: Parse request params.

  return req;
}

pub fn parse_request(chunk: &[String]) -> Request {
  log_trace!("{:?}", chunk);

  let raw = parse_request_raw(chunk);

  match raw.id {
    Some(id) if id == READ_VERSION => Request::ReadVersion,
    Some(id) if id == WRITE_PUT_EDGE => {
      Request::WritePutEdge(raw.src, raw.dst, raw.amount)
    },
    _ => Request::Unknown,
  }
}

pub fn next_chunk(
  lines: &[String],
  index: usize,
) -> Result<usize, ()> {
  log_trace!();

  let c = match lines[index].chars().nth(0) {
    Some(x) => x,
    _ => {
      log_error!("Unexpected empty line");
      return Err(());
    },
  };
  if c == '+'
    || c == '-'
    || c == ':'
    || c == '_'
    || c == '#'
    || c == ','
    || c == '('
  {
    //  Simple values
    return Ok(index + 1);
  }
  if c == '*' {
    //  Array
    let n = match lines[index][1..lines[index].len() - 2].parse::<i64>() {
      Ok(x) => x,
      _ => {
        log_error!("Invalid syntax {:?}", lines[index]);
        return Err(());
      },
    };
    let mut next = index + 1;
    for _ in 0..n {
      next = next_chunk(lines, next)?;
    }
    return Ok(next);
  }
  if c == '$' {
    //  Bulk string
    //  FIXME: Check the bulk string length.
    return Ok(index + 2);
  }
  log_error!("Not implemented");
  return Err(());
}

pub fn process_requests(lines: &[String]) -> String {
  log_trace!();

  for line in lines {
    log_info!("RECV {:?}", line);
  }

  let mut responses = "".to_string();

  let mut index: usize = 0;
  loop {
    let next = match next_chunk(lines, index) {
      Ok(x) => x,
      _ => return "-Syntax error\r\n".to_string(),
    };

    let req = parse_request(&lines[index..next]);
    let res = perform_request(req);
    responses += &encode_response(res);

    if next == lines.len() {
      break;
    }

    index = next;
  }

  if responses.is_empty() {
    return "-Internal error\r\n".to_string();
  }

  log_info!("SEND {:?}", responses);

  return responses;
}

pub fn service_init(settings: AugMultiGraphSettings) -> Result<Service, ()> {
  log_trace!();

  Ok(Service {
    local_port:     0,
    done:           Arc::new(AtomicBool::new(false)),
    graph_readable: Arc::new(Mutex::new(AugMultiGraph::new(settings.clone()))),
    graph_writable: Arc::new(Mutex::new(AugMultiGraph::new(settings))),
    threads:        vec![],
  })
}

pub fn service_run(
  service: &mut Service,
  mut num_threads: usize,
  url: &str,
) -> Result<(), ()> {
  log_trace!();

  if num_threads == 0 {
    num_threads = 1;
  }

  service.threads.reserve_exact(num_threads);

  let listener_root = match TcpListener::bind(url) {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return Err(());
    },
  };

  service.local_port = match listener_root.local_addr() {
    Ok(addr) => addr.port(),
    Err(e) => {
      log_error!("{}", e);
      return Err(());
    },
  };

  for _ in 0..num_threads {
    let done = service.done.clone();

    let listener = match listener_root.try_clone() {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
        return Err(());
      },
    };

    let thread = thread::spawn(move || {
      while !done.load(Ordering::SeqCst) {
        match listener.set_nonblocking(true) {
          Ok(_) => {},
          Err(e) => {
            log_error!("{}", e);
            return;
          },
        }

        match listener.accept() {
          Ok((mut stream, _)) => {
            match stream.set_nonblocking(true) {
              Ok(_) => {},
              Err(e) => {
                log_error!("{}", e);
                continue;
              },
            }

            match stream
              .set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SEC)))
            {
              Ok(_) => {},
              Err(e) => {
                log_error!("{}", e);
                continue;
              },
            }

            match stream
              .set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT_SEC)))
            {
              Ok(_) => {},
              Err(e) => {
                log_error!("{}", e);
                continue;
              },
            }

            loop {
              if done.load(Ordering::SeqCst) {
                match stream.shutdown(Shutdown::Both) {
                  Ok(_) => {},
                  Err(e) => log_error!("{}", e),
                }
                break;
              }

              let buf_stream = match stream.try_clone() {
                Ok(x) => x,
                Err(e) => {
                  log_error!("{}", e);
                  break;
                },
              };

              let mut lines: Vec<String> = vec![];

              let mut buf = BufReader::new(buf_stream);
              loop {
                let mut line = "".to_string();
                match buf.read_line(&mut line) {
                  Err(e) if e.kind() != ErrorKind::WouldBlock => {
                    log_error!("{:?}", e.kind())
                  },
                  _ => {},
                }
                if !line.is_empty() {
                  lines.push(line);
                } else if !lines.is_empty() || done.load(Ordering::SeqCst) {
                  break;
                } else {
                  thread::sleep(Duration::from_millis(ACCEPT_DELAY_MSEC));
                }
              }

              if lines.is_empty() {
                match stream.shutdown(Shutdown::Both) {
                  Ok(_) => {},
                  Err(e) => log_error!("{}", e),
                }
                break;
              }

              let response = process_requests(&lines);

              match stream.set_nonblocking(false) {
                Ok(_) => {},
                Err(e) => {
                  log_error!("{}", e);
                  break;
                },
              }

              match stream.write(response.as_bytes()) {
                Ok(_) => {},
                Err(e) => {
                  log_error!("{}", e);
                  break;
                },
              }

              match stream.set_nonblocking(true) {
                Ok(_) => {},
                Err(e) => {
                  log_error!("{}", e);
                  break;
                },
              }
            }
          },
          Err(e) if e.kind() == ErrorKind::WouldBlock => {
            thread::sleep(Duration::from_millis(ACCEPT_DELAY_MSEC));
          },
          Err(e) => log_error!("{}", e),
        }
      }
    });

    service.threads.push(thread);
  }

  Ok(())
}

pub fn service_join(service: &mut Service) {
  log_trace!();

  while let Some(x) = service.threads.pop() {
    match x.join() {
      Err(_) => log_error!("Failed to join thread"),
      _ => {},
    }
  }
}

pub fn service_stop(service: &mut Service) {
  log_trace!();

  service.done.store(true, Ordering::SeqCst);

  service_join(service);
}

#[cfg(test)]
mod tests {
  use super::*;
  use redis::*;

  #[test]
  fn cmd_version() {
    let mut service = service_init(AugMultiGraphSettings::default()).unwrap();
    service_run(&mut service, 4, "127.0.0.1:0").unwrap();

    let redis =
      Client::open(format!("redis://127.0.0.1:{}", service.local_port))
        .unwrap();
    let mut conn = redis.get_connection().unwrap();
    let ver: String = conn.get(READ_VERSION).unwrap();

    assert_eq!(ver, VERSION);

    service_stop(&mut service);
  }

  #[test]
  fn cmd_put_edge() {
    let mut service = service_init(AugMultiGraphSettings::default()).unwrap();
    service_run(&mut service, 4, "127.0.0.1:0").unwrap();

    let redis =
      Client::open(format!("redis://127.0.0.1:{}", service.local_port))
        .unwrap();
    let mut conn = redis.get_connection().unwrap();

    let ok: String = cmd("SET")
      .arg(WRITE_PUT_EDGE)
      .arg("src")
      .arg("U1")
      .arg("dst")
      .arg("U2")
      .arg("amount")
      .arg(42.0)
      .query(&mut conn)
      .unwrap();

    assert_eq!(ok, "OK");

    service_stop(&mut service);
  }
}
