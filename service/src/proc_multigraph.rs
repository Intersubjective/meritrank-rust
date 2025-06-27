use crate::aug_graph::{AugGraph, AugGraphOpcode};
use crate::proc_graph::{AugGraphOp, GraphProcessor};
use crate::log::*;
use crate::protocol::{Request, Response, ServiceRequestOpcode, SubgraphName};
use crate::Ordering;
use crate::{log_trace, log_verbose, log_warning};
use dashmap::DashMap;
use tokio::sync::mpsc;
const SUBGRAPH_QUEUE_CAPACITY: usize = 1024;
const SLEEP_DURATION_AFTER_PUBLISH_MS: u64 = 100;

pub struct MultiGraphProcessor {
  subgraphs_map: DashMap<SubgraphName, GraphProcessor>,
}

impl MultiGraphProcessor {
  pub fn new() -> Self {
    MultiGraphProcessor {
      subgraphs_map: DashMap::new(),
    }
  }

  pub fn get_tx_channel(
    &self,
    subgraph_name: &SubgraphName,
  ) -> mpsc::Sender<AugGraphOp> {
    match self.subgraphs_map.get(subgraph_name) {
      Some(subgraph) => subgraph.op_sender.clone(),
      None => self
        .subgraphs_map
        .entry(subgraph_name.clone())
        .or_insert(GraphProcessor::new(
          AugGraph::new(),
          SLEEP_DURATION_AFTER_PUBLISH_MS,
          SUBGRAPH_QUEUE_CAPACITY,
        ))
        .op_sender
        .clone(),
    }
  }

  pub async fn send_op(
    &self,
    subgraph_name: &SubgraphName,
    op: AugGraphOp,
  ) -> Response {
    if self.get_tx_channel(subgraph_name).send(op).await.is_ok() {
      Response {
        response: 2,
      }
    } else {
      Response {
        response: 0,
      }
    }
  }

  fn process_read<F>(
    &self,
    subgraph_name: &SubgraphName,
    read_function: F,
  ) -> Response
  where
    F: FnOnce(&AugGraph) -> Response,
  {
    let subgraph = match self.subgraphs_map.get(subgraph_name) {
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

    let reader_handle = subgraph.data_reader_factory.handle();
    log_trace!("Obtained reader handle for subgraph: {:?}", subgraph_name);

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

    let aug_graph: &AugGraph = &*guard;
    log_trace!(
      "Successfully accessed AugGraph for subgraph: {:?}",
      subgraph_name
    );

    let response = read_function(aug_graph);
    log_verbose!("Executed read function for subgraph: {:?}", subgraph_name);

    response
  }

  pub async fn process_request(
    &self,
    req: &Request,
  ) -> Response {
    match req.opcode {
      ServiceRequestOpcode::WriteEdge => {
        self
          .send_op(
            &req.subgraph_name,
            AugGraphOp::new(AugGraphOpcode::WriteEdge, req.ego.clone()),
          )
          .await
      },
      ServiceRequestOpcode::ReadScores => {
        self.process_read(&req.subgraph_name, |aug_graph| {
          let scores = aug_graph.read_scores(&req.ego, &req.score_options);
          Response {
            response: 0,
          }
        })
      },
    }
  }
}
