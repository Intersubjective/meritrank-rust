mod aug_graph;
mod data;
mod helpers;
mod node_registry;
mod request_handler;
mod settings;
mod state_manager;

mod utils;
mod vsids;

mod legacy_protocol;
mod legacy_request_handler;
mod legacy_sync_state_manager;

#[cfg(test)]
mod legacy_tests;

use crate::request_handler::*;
use crate::settings::*;
use crate::state_manager::MultiGraphProcessor;
use crate::utils::log::*;

use tokio_util::sync::CancellationToken;

use std::{error::Error, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let _ = ctrlc::set_handler(move || {
    println!();
    std::process::exit(0);
  });

  log_info!("MeritRank Service");

  let settings = load_from_env();

  let processor = Arc::new(MultiGraphProcessor::new(settings.clone()));

  let _legacy_server = legacy_request_handler::run(
    settings.clone(),
    Arc::clone(&processor)
  );

  let _ = run_server(settings, processor, CancellationToken::new()).await;

  Ok(())
}
