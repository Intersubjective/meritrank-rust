use meritrank_service::request_handler::run_server;
use meritrank_service::settings::load_from_env;
use meritrank_service::state_manager::MultiGraphProcessor;
use meritrank_service::utils::log::{init_log_cmd_from_env, *};

use tokio_util::sync::CancellationToken;

use std::{error::Error, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  init_log_cmd_from_env();
  let _ = ctrlc::set_handler(move || {
    println!();
    std::process::exit(0);
  });

  log_info!("MeritRank Service");

  let settings = load_from_env();

  let processor = Arc::new(MultiGraphProcessor::new(settings.clone()));

  let _ = run_server(settings, processor, CancellationToken::new()).await;

  Ok(())
}
