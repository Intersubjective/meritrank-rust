mod data;
mod request_handler;
mod state_manager;

mod utils;
mod vsids;

mod legacy_protocol;
mod legacy_request_handler;

#[cfg(test)]
mod legacy_tests;

use crate::request_handler::*;
use crate::state_manager::{MultiGraphProcessor, MultiGraphProcessorSettings};
use crate::utils::log::*;

use tokio::join;
use tokio_util::sync::CancellationToken;

use std::{error::Error, sync::Arc};

const DEFAULT_NNG_SERVER_URL: &str = "tcp://127.0.0.1:8040";
const DEFAULT_NNG_NUM_THREADS: usize = 4;
const DEFAULT_SERVER_URL: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let _ = ctrlc::set_handler(move || {
    println!();
    std::process::exit(0);
  });

  log_info!("MeritRank Service");

  let processor =
    Arc::new(MultiGraphProcessor::new(MultiGraphProcessorSettings {
      num_walks: 100,
      zero_opinion_num_walks: 50,
      ..MultiGraphProcessorSettings::default()
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
