pub mod protocol;
pub mod read_ops;
pub mod request_handler;
pub mod state_manager;
pub mod write_ops;

use crate::utils::log::*;
use self::request_handler::run;

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
