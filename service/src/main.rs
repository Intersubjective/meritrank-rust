pub mod astar;
pub mod log;
pub mod operations;
pub mod protocol;
pub mod service;

#[cfg(test)]
mod tests;

use ctrlc;

use crate::service::{main_async, THREADS};

fn main() -> Result<(), ()>
{
  let _ = ctrlc::set_handler(move || {
    println!("");
    std::process::exit(0)
  });

  main_async(*THREADS)
}
