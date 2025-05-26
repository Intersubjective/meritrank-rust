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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_data;

use crate::request_handler::main_async;

fn main() {
  let _ = ctrlc::set_handler(move || {
    println!();
    std::process::exit(0);
  });

  if let Err(e) = main_async() {
    eprintln!("Error in main_async: {:?}", e);
    std::process::exit(1);
  }
}
