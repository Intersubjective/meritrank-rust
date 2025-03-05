pub mod astar;
pub mod log;
pub mod operations;
pub mod service;
pub mod vsids;

#[cfg(test)]
mod tests;

use ctrlc;
use std::env::var;

use crate::log::*;
use crate::service::*;

pub const VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
  Some(x) => x,
  None => "dev",
};

fn main() -> Result<(), ()> {
  log_trace!();

  let _ = ctrlc::set_handler(move || {
    println!("");
    std::process::exit(0)
  });

  let num_threads = match var("MERITRANK_SERVICE_THREADS") {
    Ok(s) => match s.parse::<usize>() {
      Ok(n) => {
        if n > 0 {
          n
        } else {
          log_error!("Invalid MERITRANK_SERVICE_THREADS: {:?}", s);
          return Err(());
        }
      },
      _ => {
        log_error!("Invalid MERITRANK_SERVICE_THREADS: {:?}", s);
        return Err(());
      },
    },
    _ => 1,
  };

  let url = match var("MERITRANK_SERVICE_URL") {
    Ok(s) => s,
    _ => "tcp://127.0.0.1:10234".to_string(),
  };

  log_info!(
    "Starting server {} at {}, {} threads",
    VERSION,
    url,
    num_threads
  );

  let settings = parse_settings()?;

  log_info!("Num walks: {}", settings.num_walks);

  let mut service = service_init(settings)?;

  service_run(&mut service, num_threads, &url)?;

  service_join(&mut service);
  Ok(())
}
