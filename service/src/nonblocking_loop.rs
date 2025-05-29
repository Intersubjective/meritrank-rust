use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use std::marker::PhantomData;
use std::thread;
use tokio::sync::mpsc;

struct NonblockingSubgraph<T, Op> {
  loop_thread: thread::JoinHandle<()>,
  tx_ops_queue: mpsc::Sender<Op>,
  reader_factory: ReadHandleFactory<T>,
  _phantom: PhantomData<T>, // This is needed because T is not used directly in the struct
}

impl<T, Op> NonblockingSubgraph<T, Op>
where
  T: 'static + Send + Sync + Clone + Absorb<Op>,
  Op: 'static + Send,
{
  fn new_from_empty(
    t: T,
    sleep: u64,
    queue_len: usize,
  ) -> Self {
    let (writer, reader) = left_right::new_from_empty::<T, Op>(t);
    let (tx, rx) = mpsc::channel::<Op>(queue_len);
    let loop_thread = thread::spawn(move || _process_loop(writer, rx, sleep));
    NonblockingSubgraph {
      loop_thread,
      tx_ops_queue: tx,
      reader_factory: reader.factory(),
      _phantom: PhantomData,
    }
  }

  pub fn shutdown(mut self) -> thread::Result<()> {
    // Drop the sender, which will close the channel
    drop(self.tx_ops_queue);
    // Join the thread
    self.loop_thread.join()
  }
}

fn _process_loop<T, Op>(
  mut writer: WriteHandle<T, Op>,
  mut rx_ops_queue: mpsc::Receiver<Op>,
  sleep: u64,
) where
  T: 'static + Send + Sync + Absorb<Op>,
  Op: 'static + Send,
{
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
    thread::sleep(std::time::Duration::from_millis(sleep));
    writer.publish();
  }
}

#[test]
fn test_nonblocking() {
  use left_right::Absorb;

  // Define a simple wrapper for i32 that implements Absorb
  #[derive(Clone)]
  pub struct MyDataType(i32);

  struct TestOp(i32);

  impl Absorb<TestOp> for MyDataType {
    fn absorb_first(
      &mut self,
      operation: &mut TestOp,
      _: &Self,
    ) {
      self.0 += operation.0;
    }
    fn absorb_second(
      &mut self,
      operation: TestOp,
      _: &Self,
    ) {
      self.0 += operation.0;
    }
    fn sync_with(
      &mut self,
      first: &Self,
    ) {
      *self = first.clone();
    }
  }

  // Create a new NonblockingSubgraph
  let subgraph = NonblockingSubgraph::<MyDataType, TestOp>::new_from_empty(
    MyDataType(0),
    0,
    10,
  );
  subgraph.tx_ops_queue.blocking_send(TestOp(1)).unwrap();
  subgraph.tx_ops_queue.blocking_send(TestOp(1)).unwrap();
  subgraph.tx_ops_queue.blocking_send(TestOp(1)).unwrap();
  thread::sleep(std::time::Duration::from_millis(10));
  let handle = subgraph.reader_factory.handle();
  assert_eq!(handle.enter().unwrap().0, 3);
  subgraph.shutdown().unwrap();
}
