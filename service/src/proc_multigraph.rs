use crate::aug_graph::nodes::{node_kind_from_prefix, NodeKind};
use crate::aug_graph::settings::AugGraphSettings;
use crate::aug_graph::{AugGraph, NodeName};
use crate::log::*;
use crate::proc_graph::{AugGraphOp, GraphProcessor};
use crate::protocol::{Request, Response, SubgraphName};
use crate::Ordering;
use crate::{log_trace, log_verbose, log_warning};
use dashmap::DashMap;
use meritrank_core::Weight;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use crate::aug_graph::vsids::Magnitude;

pub struct MultiGraphProcessorSettings {
  pub sleep_duration_after_publish_ms: u64,
  pub subgraph_queue_capacity:         usize,
}

impl Default for MultiGraphProcessorSettings {
  fn default() -> Self {
    MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 100,
      subgraph_queue_capacity:         1024,
    }
  }
}

pub struct MultiGraphProcessor {
  subgraphs_map: DashMap<SubgraphName, GraphProcessor>,
  settings:      MultiGraphProcessorSettings,
}

impl MultiGraphProcessor {
  pub fn new(settings: MultiGraphProcessorSettings) -> Self {
    MultiGraphProcessor {
      subgraphs_map: DashMap::new(),
      settings,
    }
  }

  fn get_tx_channel(
    &self,
    subgraph_name: &SubgraphName,
  ) -> mpsc::Sender<AugGraphOp> {
    match self.subgraphs_map.get(subgraph_name) {
      Some(subgraph) => subgraph.op_sender.clone(),
      None => self
        .subgraphs_map
        .entry(subgraph_name.clone())
        .or_insert(GraphProcessor::new(
          AugGraph::new(AugGraphSettings::from_env().unwrap_or_default()),
          self.settings.sleep_duration_after_publish_ms,
          self.settings.subgraph_queue_capacity,
        ))
        .op_sender
        .clone(),
    }
  }

  async fn send_op(
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
    match req {
      Request::WriteEdgeReq {
        subgraph_name,
        src,
        dst,
        amount,
        magnitude,
      } => {
        self
          .process_write_edge(subgraph_name, src, dst, *amount, *magnitude)
          .await
      },
      Request::ReadScoresReq {
        subgraph_name,
        ego,
        score_options,
      } => self.process_read(subgraph_name, |aug_graph| {
        let scores = aug_graph.read_scores(ego, score_options);
        Response {
          response: 0,
        }
      }),
    }
  }

  async fn process_write_edge(
    &self,
    subgraph_name: &SubgraphName,
    src: &NodeName,
    dst: &NodeName,
    amount: Weight,
    magnitude: Magnitude
  ) -> Response {
    log_trace!("{} {} {}", src, dst, amount);

    if src == dst {
      log_error!("Self-reference is not allowed.");
      return Response {
        response: 0,
      };
    }

    let src_kind_opt = node_kind_from_prefix(src);
    let dst_kind_opt = node_kind_from_prefix(dst);

    match (src_kind_opt, dst_kind_opt) {
      (Some(NodeKind::User), Some(NodeKind::User)) => {
        self.process_user_to_user_edge(src, dst, amount, magnitude).await
      },

      (Some(NodeKind::User), Some(NodeKind::PollVariant)) => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::SetUserVoteOp {
              user_id: src.clone(),
              variant_id: dst.clone(),
              amount,
            },
          )
          .await
      },
      (Some(NodeKind::PollVariant), Some(NodeKind::Poll)) => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::AddPollVariantOp {
              poll_id:    dst.clone(),
              variant_id: src.clone(),
            },
          )
          .await
      },
      (Some(src_kind), Some(dst_kind))
        if src_kind == NodeKind::PollVariant
          || src_kind == NodeKind::Poll
          || dst_kind == NodeKind::PollVariant
          || dst_kind == NodeKind::Poll =>
      {
        log_warning!("Unexpected edge type: {:?} -> {:?} in context {:?}. No action taken.", src_kind_opt, dst_kind_opt, subgraph_name);
        Response {
          response: 0,
        }
      },
      _ => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::WriteEdgeOp {
              src: src.clone(),
              dst: dst.clone(),
              amount,
              magnitude,
            },
          )
          .await
      },
    }
  }
  async fn process_user_to_user_edge(
    &self,
    src: &NodeName,
    dst: &NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) -> Response {
    let mut join_set = JoinSet::new();

    for ref_multi in self.subgraphs_map.iter() {
      let subgraph = ref_multi.value();
      let op_sender = subgraph.op_sender.clone();
      let src = src.clone();
      let dst = dst.clone();

      join_set.spawn(async move {
        op_sender
          .send(AugGraphOp::WriteEdgeOp {
            src,
            dst,
            amount,
            magnitude
          })
          .await
      });
    }

    let mut all_successful = true;
    while let Some(result) = join_set.join_next().await {
      match result {
        Ok(Ok(())) => {},
        _ => {
          log_error!("Failed to send WriteEdge operation to a subgraph");
          all_successful = false;
        },
      }
    }

    Response {
      response: if all_successful { 1 } else { 0 },
    }
  }
}
