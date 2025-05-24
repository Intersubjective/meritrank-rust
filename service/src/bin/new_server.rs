use parking_lot::Mutex;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpListener,
};

use bincode::{
  config::standard, decode_from_slice, encode_to_vec, Decode, Encode,
};
use meritrank_service::lrgraph::{CountReader, Counter, CounterAddOp};
use std::sync::Arc;
use std::{error::Error, thread, time::Duration};

#[derive(Debug, Encode, Decode)]
struct Request {
  number: u64,
}

#[derive(Debug, Encode, Decode)]
struct Response {
  response: u64,
}
#[tokio::main]
//#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
  let listener = TcpListener::bind("127.0.0.1:8080").await?;
  println!("Server running on 127.0.0.1:8080");
  let (write, read) = left_right::new::<i32, CounterAddOp>();
  let shared_writer = Arc::new(Mutex::new(write));
  loop {
    let (mut socket, _) = listener.accept().await?;
    // We clone the reader handle outside to avoid locking it out
    // of the subsequent iterations.
    let shared_writer_clone = Arc::clone(&shared_writer);
    let read_handle_clone = read.clone();
    tokio::spawn(async move {
      // These wrapper types are likely what you'll give out to your consumers.
      //let mut cw = Counter::new(write);
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

      let mut counter_state = 0;
      if req.number < 10 {
        tokio::spawn(async move {
          // It is EXTREMELY important to move long-running tasks to a 
          // background thread to avoid blocking the main thread.
          // Otherwise, the main thread pool could become clogged with 
          // long writes, and reads will not be processed either.
          let _ = tokio::task::spawn_blocking(move || {
            let mut write_guard = shared_writer_clone.lock();
            write_guard.append(CounterAddOp::new());
            thread::sleep(Duration::from_millis(100));
            write_guard.publish();
          })
          .await;
        });

        // Simulate write request
      } else {
        let cr = CountReader::new(read_handle_clone);
        counter_state = cr.get();
      }

      let resp = Response {
        response: counter_state as u64,
      };
      let out = match encode_to_vec(&resp, config) {
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
