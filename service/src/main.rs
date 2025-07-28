mod data;
mod request_handler;
mod state_manager;

mod legacy_protocol;
mod legacy_request_handler;

mod utils;
mod vsids;

use crate::request_handler::*;
use crate::state_manager::{MultiGraphProcessor, MultiGraphProcessorSettings};
use crate::utils::log::*;
// use crate::legacy_request_handler;

use tokio::join;
use tokio_util::sync::CancellationToken;

use std::{error::Error, sync::Arc};

const DEFAULT_NNG_SERVER_URL: &str = "tcp://127.0.0.1:8040";
const DEFAULT_NNG_NUM_THREADS: usize = 4;
const DEFAULT_SERVER_URL: &str = "127.0.0.1:8080";
const DEFAULT_SLEEP_AFTER_PUBLISH: u64 = 10;
const DEFAULT_SUBGRAPH_QUEUE_CAPACITY: usize = 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let _ = ctrlc::set_handler(move || {
    println!();
    std::process::exit(0);
  });

  log_info!("MeritRank Service");

  let processor =
    Arc::new(MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: DEFAULT_SLEEP_AFTER_PUBLISH,
      subgraph_queue_capacity:         DEFAULT_SUBGRAPH_QUEUE_CAPACITY,
    }));

  let legacy_server_task = legacy_request_handler::run(
    legacy_request_handler::Settings {
      url:         DEFAULT_NNG_SERVER_URL.into(),
      num_threads: DEFAULT_NNG_NUM_THREADS,
    },
    Arc::clone(&processor),
    CancellationToken::new(),
  );

  let server_task = run_server(
    ServerSettings {
      url: DEFAULT_SERVER_URL.into(),
    },
    processor,
    CancellationToken::new(),
  );

  let _ = join!(legacy_server_task, server_task);

  Ok(())
}

//  Failing test until everything is done.
#[test]
fn work_in_progress() {
  assert!(false);
}
