use dashmap::DashMap;
use meritrank_service::aug_graph::{AugGraph, AugGraphOp, AugGraphOpcode};
use meritrank_service::new_server_ops::Response;
use meritrank_service::new_server_ops::{Request, ServiceRequestOpcode};

use meritrank_service::log::*;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpListener,
};

use bincode::{config::standard, decode_from_slice, encode_to_vec};
use meritrank_service::new_server_ops::SubgraphName;
use meritrank_service::nonblocking_loop::ConcurrentDataProcessor;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

type ConcurrentGraphProcessor = ConcurrentDataProcessor<AugGraph, AugGraphOp>;

const SUBGRAPH_QUEUE_CAPACITY: usize = 1024;
const SLEEP_DURATION_AFTER_PUBLISH_MS: u64 = 100;

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
    // Success
    Response {
      response: 2,
    }
  } else {
    // Fail
    Response {
      response: 0,
    }
  }
}

fn process_read<F>(
  subgraphs_map: &DashMap<SubgraphName, ConcurrentGraphProcessor>,
  subgraph_name: &SubgraphName,
  read_function: F,
) -> Response
where
  F: FnOnce(&AugGraph) -> Response,
{
  // Step 1: Get the ConcurrentGraphProcessor for the subgraph
  let subgraph = match subgraphs_map.get(subgraph_name) {
    Some(subgraph) => {
      log_verbose!("Found subgraph for name: {:?}", subgraph_name);
      subgraph
    },
    None => {
      log_warning!("Subgraph not found for name: {:?}", subgraph_name);
      return Response {
        response: 0,
      };
    },
  };

  // Step 2: Get the ReadHandle from the ConcurrentGraphProcessor
  let reader_handle = subgraph.data_reader_factory.handle();
  log_trace!("Obtained reader handle for subgraph: {:?}", subgraph_name);

  // Step 3: Enter the ReadHandle to get the ReadGuard
  let guard = match reader_handle.enter() {
    Some(guard) => {
      log_trace!(
        "Successfully entered reader handle for subgraph: {:?}",
        subgraph_name
      );
      guard
    },
    None => {
      log_warning!("Failed to enter reader handle for subgraph: {:?}. WriteHandle might have been dropped.", subgraph_name);
      return Response {
        response: 0,
      };
    },
  };

  // Step 4: Dereference the ReadGuard to get the AugGraph
  let aug_graph: &AugGraph = &*guard;
  log_trace!(
    "Successfully accessed AugGraph for subgraph: {:?}",
    subgraph_name
  );

  // Step 5: Call the read_function with the AugGraph
  let response = read_function(aug_graph);
  log_verbose!("Executed read function for subgraph: {:?}", subgraph_name);

  response
}

pub async fn process_request(
  subgraphs_map: &Arc<DashMap<SubgraphName, ConcurrentGraphProcessor>>,
  req: &Request,
) -> Response {
  match req.opcode {
    ServiceRequestOpcode::WriteEdge => {
      send_op(
        &*subgraphs_map,
        &req.subgraph_name,
        AugGraphOp::new(AugGraphOpcode::WriteEdge, req.ego.clone()),
      )
      .await
    },
    ServiceRequestOpcode::ReadRank => {
      process_read(&*subgraphs_map, &req.subgraph_name, |aug_graph| {
        aug_graph.get_rank(&req.ego)
      })
    },
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

      let response = process_request(&subgraphs_map, &req).await;

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
