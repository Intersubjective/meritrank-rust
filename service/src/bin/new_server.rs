use dashmap::DashMap;
use meritrank_service::aug_graph::{AugGraph, AugGraphOp, AugGraphOpcode};
use meritrank_service::new_sever_ops::Response;
use meritrank_service::new_sever_ops::{Request, ServiceRequestOpcode};
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
use meritrank_service::new_sever_ops::SubgraphName;
use meritrank_service::nonblocking_loop::ConcurrentDataProcessor;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

type ConcurrentGraphProcessor = ConcurrentDataProcessor<AugGraph, AugGraphOp>;

const SUBGRAPH_QUEUE_CAPACITY: usize = 1024;
const SLEEP_DURATION_AFTER_PUBLISH_MS: u64 = 100;

pub fn get_reader_handle(
  subgraphs_map: &DashMap<SubgraphName, ConcurrentGraphProcessor>,
  subgraph_name: &SubgraphName,
) -> Option<left_right::ReadHandle<AugGraph>> {
  subgraphs_map
    .get(subgraph_name)
    .map(|subgraph| subgraph.data_reader_factory.handle())
}

pub fn get_tx_channel(
  subgraphs_map: &DashMap<SubgraphName, ConcurrentGraphProcessor>,
  subgraph_name: &SubgraphName,
) -> mpsc::Sender<AugGraphOp> {
  // Note: cloning the ops_sender creates additional input paths into
  // the queue. However, when pushing multiple operation through them,
  // the operations sent from different threads might end up interleaved.
  // This is fine, since we always push just a single operation.
  // In general, we want to minimize the lifetime of the reference to
  // the subgraphs_map, since that uses Dashmap, and that may block the
  // readers on write, and vice-versae.
  match subgraphs_map.get(&subgraph_name.clone()) {
    Some(subgraph) => subgraph.op_sender.clone(),
    None => subgraphs_map
      .entry(subgraph_name.clone())
      .or_insert(ConcurrentGraphProcessor::new(
        AugGraph::new(),
        SLEEP_DURATION_AFTER_PUBLISH_MS,
        SUBGRAPH_QUEUE_CAPACITY,
      ))
      .op_sender
      .clone(),
  }
}

pub async fn send_op(
  subgraphs_map: &DashMap<SubgraphName, ConcurrentGraphProcessor>,
  subgraph_name: &SubgraphName,
  op: AugGraphOp,
) -> Response {
  if get_tx_channel(&*subgraphs_map, &subgraph_name)
    .send(op)
    .await
    .is_ok()
  {
    Response {
      response: 2,
    }
  } else {
    Response {
      response: 0,
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let listener = TcpListener::bind("127.0.0.1:8080").await?;
  println!("Server running on 127.0.0.1:8080");

  let subgraphs_map =
    Arc::new(DashMap::<SubgraphName, ConcurrentGraphProcessor>::new());
  loop {
    let (mut socket, _) = listener.accept().await?;

    let subgraphs_map = Arc::clone(&subgraphs_map);
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

      let response = match req.opcode {
        ServiceRequestOpcode::WriteEdge => {
          send_op(
            &*subgraphs_map,
            &req.subgraph_name,
            AugGraphOp::new(AugGraphOpcode::WriteEdge, req.ego.clone()),
          )
          .await
        },
        ServiceRequestOpcode::ReadRank => {
          if let Some(reader) =
            get_reader_handle(&*subgraphs_map, &req.subgraph_name)
          {
            Response {
              response: 1,
            }
          } else {
            Response {
              response: 0,
            }
          }
        },
      };

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
