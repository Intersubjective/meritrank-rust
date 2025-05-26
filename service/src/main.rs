pub mod astar;
pub mod aug_multi_graph;
pub mod bloom_filter;
pub mod constants;
pub mod log;
pub mod nodes;
pub mod operations;
pub mod protocol;
pub mod quantiles;
pub mod request_handler;
pub mod state_manager;
pub mod subgraph;
pub mod vsids;
pub mod zero_opinion;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_data;

use crate::request_handler::main_async;
use ctrlc;

fn main() -> Result<(), ()> {
  let _ = ctrlc::set_handler(move || {
    println!("");
    std::process::exit(0)
  });

  main_async()
}
