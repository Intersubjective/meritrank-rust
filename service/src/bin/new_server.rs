use parking_lot::Mutex;
use scc::HashIndex;
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
  loop_task: tokio::task::JoinHandle<()>,
  tx_ops_queue: mpsc::Sender<GraphOperation>,
  reader_factory: ReadHandleFactory<i32>, // Changed from ReadHandle
  // other fields...

}


async fn _process_loop(
    writer: Arc<Mutex<WriteHandle<i32, CounterAddOp>>>,
    mut rx_ops_queue: mpsc::Receiver<GraphOperation>
) {
    while let Some(op) = rx_ops_queue.recv().await {
        let writer = writer.clone();
        // It is EXTREMELY important to move long-running tasks to a
        // background thread to avoid blocking the main thread.
        // Otherwise, the main thread pool could become clogged with
        // long writes, and reads will not be processed either.
        task::spawn_blocking(move || {
            let mut writer = writer.lock();
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
        })
        .await
        .expect("TODO: panic message");
    }
}

impl NonblockingSubgraph {
    fn new(name: SubgraphName) -> Self {
        let (writer, reader) = left_right::new::<i32, CounterAddOp>();
        let writer = Arc::new(Mutex::new(writer));
        let (tx, rx) = mpsc::channel::<GraphOperation>(SUBGRAPH_QUEUE_CAPACITY);
        let loop_task = tokio::spawn(_process_loop(writer.clone(), rx));

        NonblockingSubgraph {
            name,
            loop_task,
            tx_ops_queue: tx,
            reader_factory: reader.factory(),
            // Initialize other fields...
        }
    }
}




}

struct MultigraphOperation {
  subgraph_name: SubgraphName,
  op: GraphOperation,
}

/// A single subgraph's mailbox & processing loop
async fn subgraph_loop(mut rx: mpsc::Receiver<GraphOperation>) {
  let mut writer = shared_writer.lock();
  while let Some(op) = rx.recv().await {
    // It is EXTREMELY important to move long-running tasks to a
    // background thread to avoid blocking the main thread.
    // Otherwise, the main thread pool could become clogged with
    // long writes, and reads will not be processed either.
    task::spawn_blocking(move || {
      writer.append(op);
      println!("Ops: {}", rx.len());

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
    })
      .await
      .expect("TODO: panic message");
  }
}

const PRIMARY_QUEUE_CAPACITY: usize = 1024;
const SUBGRAPH_QUEUE_CAPACITY: usize = 1024;

#[derive(Debug, Encode, Decode)]
struct Request {
  number: u64,
}

#[derive(Debug, Encode, Decode)]
struct Response {
  response: u64,
}

fn create_dispatcher_task(mut primary_rx: mpsc::Receiver<MultigraphOperation>) {
  // ---- Dispatcher -------------------------------------------------------
  tokio::spawn(async move {
    // context-id → sender for that context
    let mut mailboxes: HashMap<SubgraphName, mpsc::Sender<GraphOperation>> =
      HashMap::new();

    while let Some(multigraph_op) = primary_rx.recv().await {
      let sender_for_subgraph = mailboxes
        .entry(multigraph_op.subgraph_name)
        .or_insert_with(|| {
          // New subgraph: create its private queue & loop
          let (tx, rx) = mpsc::channel::<GraphOperation>(SUBGRAPH_QUEUE_CAPACITY);
          tokio::spawn(subgraph_loop(rx));
          tx
        });

      // Forward the operation to its subgraph queue
      // Note: blocks if there is no capacity!
      let _ = sender_for_subgraph.send(multigraph_op.op).await;
    }
  });
  // -----------------------------------------------------------------------
}

#[tokio::main]
//#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
  let listener = TcpListener::bind("127.0.0.1:8080").await?;
  println!("Server running on 127.0.0.1:8080");
  let (write, read) = left_right::new::<i32, CounterAddOp>();
  let shared_writer = Arc::new(Mutex::new(write));

  let (tx, mut rx) = mpsc::channel::<CounterAddOp>(102400);

  let subgraphs_map =
    Arc::new(HashIndex::<SubgraphName, NonblockingSubgraph>::new());

  // Primary queue fed by your network / IO layer
  let (primary_tx, primary_rx) =
    mpsc::channel::<MultigraphOperation>(PRIMARY_QUEUE_CAPACITY);

  create_dispatcher_task(primary_rx);

  let tx = Arc::new(tx); // wrap in Arc to clone inside loop

  let counter = Arc::new(AtomicUsize::new(0));
  loop {
    let counter_clone = Arc::clone(&counter);
    let (mut socket, _) = listener.accept().await?;
    // We clone the reader handle outside to avoid locking it out
    // of the subsequent iterations.
    let read_handle_clone = read.clone();
    let tx = Arc::clone(&tx);

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
        let guard = Guard::new();
        if let Some(subgraph) = subgraphs_map.peek(&subgraph_name, &guard) {
          let reader = subgraph.reader_factory.handle();
        }
      } else {
        let subgraph = subgraphs_map
          .entry_async(subgraph_name.clone())
          .await
          .or_insert(NonblockingSubgraph::new(subgraph_name.clone()));
        MOVE
        QUEUE
        AND
        LOOP
        TO
        GRAPH
        OBJECT
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
