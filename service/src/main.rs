pub mod aug_graph;
mod constants;
pub mod log;
pub mod nonblocking_loop;
mod proc_graph;
mod proc_multigraph;
pub mod protocol;
pub mod utils;

use std::error::Error;
use std::sync::Arc;

use crate::log::*;
use crate::proc_multigraph::{
  MultiGraphProcessor, MultiGraphProcessorSettings,
};
use crate::protocol::{Request, Response};
use bincode::{config::standard, decode_from_slice, encode_to_vec};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpListener,
};

const SERVER_ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let listener = TcpListener::bind(SERVER_ADDRESS).await?;
  println!("Server running on {}", SERVER_ADDRESS);

  let multi_graph_processor = Arc::new(MultiGraphProcessor::new(
    MultiGraphProcessorSettings::default(),
  ));

  loop {
    let (mut socket, _) = listener.accept().await?;

    let processor = Arc::clone(&multi_graph_processor);
    tokio::spawn(async move {
      let mut len_buf = [0u8; 4];
      if socket.read_exact(&mut len_buf).await.is_err() {
        return;
      }

      let len = u32::from_be_bytes(len_buf) as usize;
      let mut buf = vec![0u8; len];
      if socket.read_exact(&mut buf).await.is_err() {
        return;
      }

      let config = standard();
      let (req, _): (Request, _) = match decode_from_slice(&buf, config) {
        Ok(r) => r,
        Err(_) => return,
      };

      let response = processor.process_request(&req).await;

      let out = match encode_to_vec(&response, config) {
        Ok(data) => data,
        Err(_) => return,
      };
      let len_bytes = (out.len() as u32).to_be_bytes();

      if socket.write_all(&len_bytes).await.is_err() {
        return;
      }
      let _ = socket.write_all(&out).await;
    });
  }
}

// Make sure we have a failing test until everything is ready.
#[test]
fn work_in_progress() {
  assert!(false);
}
