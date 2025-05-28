use parking_lot::Mutex;
use scc::HashIndex;
use dashmap::DashMap;

use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpListener,
  task,
};

use bincode::{
  config::standard, decode_from_slice, encode_to_vec, Decode, Encode,
};
use meritrank_service::lrgraph::{CountReader, Counter, CounterAddOp};
use scc::ebr::Guard;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::{error::Error, thread, time::Duration};
use tokio::sync::mpsc;

type SubgraphName = String;

struct GraphOperation {
  // fields …
}
use left_right::{ReadHandle, ReadHandleFactory, WriteHandle};

struct NonblockingSubgraph {
  name: SubgraphName,
  loop_thread: std::thread::JoinHandle<()>,
  tx_ops_queue: mpsc::Sender<GraphOperation>,
  reader_factory: ReadHandleFactory<i32>, // Changed from ReadHandle
  // other fields...

}


impl NonblockingSubgraph {
  fn new(name: SubgraphName) -> Self {
    let (writer, reader) = left_right::new::<i32, CounterAddOp>();
    let (tx, rx) = mpsc::channel::<GraphOperation>(SUBGRAPH_QUEUE_CAPACITY);
    let loop_thread = thread::spawn(move || _process_loop(writer, rx));
    NonblockingSubgraph {
      name,
      loop_thread,
      tx_ops_queue: tx,
      reader_factory: reader.factory(),
    }
  }
}
fn _process_loop(
  mut writer: WriteHandle<i32, CounterAddOp>,
  mut rx_ops_queue: mpsc::Receiver<GraphOperation>,
) {
  while let Some(op) = rx_ops_queue.blocking_recv() {
    writer.append(op);
    println!("Ops: {}", rx_ops_queue.len());
    // Note that left-right is not really eventually-consistent,
    // but instead strong-consistent. This means that in case of
    // high load on reading, publish() will block readers until all
    // the _reading_ operations are finished, and then all the operations
    // are applied in the correct order.
    // There are two ways to handle this:
    // 1. sleep a bit on the write execution thread to allow the readers to flush
    // 2. implement a truly eventually-consistent version of left-right that never blocks (arc-swap)
    thread::sleep(Duration::from_millis(100));
    writer.publish();
  }
}

struct MultigraphOperation {
  subgraph_name: SubgraphName,
  op: GraphOperation,
}

const SUBGRAPH_QUEUE_CAPACITY: usize = 1024;

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

  let subgraphs_map = Arc::new(DashMap::<SubgraphName, NonblockingSubgraph>::new());


  let counter = Arc::new(AtomicUsize::new(0));
  loop {
    let counter_clone = Arc::clone(&counter);
    let (mut socket, _) = listener.accept().await?;
    // We clone the reader handle outside to avoid locking it out
    // of the subsequent iterations.
    let subgraphs_map = Arc::clone(&subgraphs_map);
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

      let subgraph_name = SubgraphName::new();

      let mut response = Response {
        response: 0,
      }; // No subgraph found, return 0

      let request_is_read = true;
      if request_is_read {
        if let Some(reader) = subgraphs_map
          .get(&subgraph_name)
          .map(|subgraph| subgraph.reader_factory.handle()) {
        }
      } else {
        // Note: cloning the ops_sender creates additional input paths into
        // the queue. However, when pushing multiple operation through them,
        // the operations sent from different threads might end up interleaved.
        // This is fine, since we always push just a single operation.
        // In general, we want to minimize the lifetime of the reference to 
        // the subgraphs_map, since that uses Dashmap, and that may block the
        // readers on write, and vice-versae.
        let ops_sender = match subgraphs_map.get(&subgraph_name) {
          Some(subgraph) => subgraph.tx_ops_queue.clone(),
          None => subgraphs_map
            .entry(subgraph_name.clone())
            .or_insert(NonblockingSubgraph::new(subgraph_name.clone()))
            .tx_ops_queue
            .clone(),
        };
      }

      let mut counter_state = 0;
      if req.number < 1 {
        // Simulate write request
        if tx.send(CounterAddOp::new()).await.is_err() {
          return;
        }
      } else {
        let cr = CountReader::new(read_handle_clone);
        counter_state = cr.get();
        counter_clone.fetch_add(1, Ordering::Relaxed);
        println!(
          "CSTATE: {} {}",
          counter_clone.load(Ordering::Relaxed),
          counter_state
        );
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
