pub mod astar;
pub mod aug_multi_graph;
pub mod bloom_filter;
pub mod constants;
pub mod errors;
pub mod log;
pub mod nodes;
pub mod protocol;
pub mod quantiles;
pub mod read_ops;
pub mod request_handler;
pub mod state_manager;
pub mod subgraph;
pub mod vsids;
pub mod write_ops;
pub mod zero_opinion;
pub mod settings;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_data;

use crate::log::*;
use crate::request_handler::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _ = ctrlc::set_handler(move || {
    println!();
    std::process::exit(0);
  });

  match run().await {
    Err(e) => {
      log_error!("Service handler failed: {}", e);
      Ok(())
    },

    _ => Ok(()),
  }
}
