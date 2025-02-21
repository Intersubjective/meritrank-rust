pub mod astar;
pub mod log;
pub mod operations;
pub mod protocol;
pub mod service;
pub mod vsids;

#[cfg(test)]
mod tests;

use crate::service::main_async;
use ctrlc;

fn main() -> Result<(), ()> {
  let _ = ctrlc::set_handler(move || {
    println!("");
    std::process::exit(0)
  });

  main_async()
}
