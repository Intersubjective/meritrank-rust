pub mod log;
pub mod protocol;
pub mod astar;
pub mod libs;
pub mod operations;
pub mod service;

#[cfg(test)]
mod tests;

use crate::service::{main_async, THREADS};
use ctrlc;

fn main() -> Result<(), ()> {
  let _ = ctrlc::set_handler(move || {
    println!("");
    std::process::exit(0)
  });

  main_async(*THREADS)
} 
