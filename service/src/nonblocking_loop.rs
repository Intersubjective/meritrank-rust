use left_right::{Absorb, ReadHandleFactory, WriteHandle};
use std::marker::PhantomData;
use std::thread;
use tokio::sync::mpsc;

pub struct ConcurrentDataProcessor<T, Op> {
  processing_thread:       thread::JoinHandle<()>,
  pub op_sender:           mpsc::Sender<Op>,
  pub data_reader_factory: ReadHandleFactory<T>,
  _phantom:                PhantomData<T>, // This is needed because T is not used directly in the struct
}

impl<T, Op> ConcurrentDataProcessor<T, Op>
where
  T: 'static + Send + Sync + Clone + Absorb<Op>,
  Op: 'static + Send,
{
  pub fn new(
    t: T,
    sleep: u64,
    queue_len: usize,
  ) -> Self {
    let (writer, reader) = left_right::new_from_empty::<T, Op>(t);
    let (tx, rx) = mpsc::channel::<Op>(queue_len);
    let loop_thread = thread::spawn(move || processing_loop(writer, rx, sleep));
    ConcurrentDataProcessor {
      processing_thread:   loop_thread,
      op_sender:           tx,
      data_reader_factory: reader.factory(),
      _phantom:            PhantomData,
    }
  }

  pub fn shutdown(self) -> thread::Result<()> {
    // Drop the sender, which will close the channel
    drop(self.op_sender);
    // Join the thread
    self.processing_thread.join()
  }
}

fn processing_loop<T, Op>(
  mut writer: WriteHandle<T, Op>,
  mut rx_ops_queue: mpsc::Receiver<Op>,
  sleep: u64,
) where
  T: 'static + Send + Sync + Absorb<Op>,
  Op: 'static + Send,
{
  while let Some(op) = rx_ops_queue.blocking_recv() {
    writer.append(op);
    //println!("Ops: {}", rx_ops_queue.len());
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

  let processor =
    ConcurrentDataProcessor::<MyDataType, TestOp>::new(MyDataType(0), 0, 10);
  processor.op_sender.blocking_send(TestOp(1)).unwrap();
  processor.op_sender.blocking_send(TestOp(1)).unwrap();
  processor.op_sender.blocking_send(TestOp(1)).unwrap();
  thread::sleep(std::time::Duration::from_millis(10));
  let handle = processor.data_reader_factory.handle();
  assert_eq!(handle.enter().unwrap().0, 3);
  processor
    .shutdown()
    .expect("Failed to shutdown processing loop");
}
