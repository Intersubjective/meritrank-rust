use crate::aug_graph::*;
use crate::data::*;
use crate::node_registry::*;
use crate::settings::*;
use crate::utils::log::*;
use crate::vsids::Magnitude;

use dashmap::DashMap;
use left_right::{Absorb, ReadHandleFactory, WriteHandle};
use meritrank_core::Weight;
use tokio::{sync::mpsc, task::JoinSet};

use std::thread;

pub struct ConcurrentDataProcessor<T, Op> {
  #[allow(unused)]
  processing_thread:       thread::JoinHandle<()>,
  pub op_sender:           mpsc::Sender<Op>,
  pub data_reader_factory: ReadHandleFactory<T>,
}

pub struct MultiGraphProcessor {
  pub subgraphs_map: DashMap<SubgraphName, GraphProcessor>,
  settings:          Settings,
}

pub type GraphProcessor = ConcurrentDataProcessor<AugGraph, AugGraphOp>;

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
      // _phantom:            PhantomData,
    }
  }

  //  FIXME: Used in testing.
  #[allow(unused)]
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

impl MultiGraphProcessor {
  pub fn new(settings: Settings) -> Self {
    MultiGraphProcessor {
      subgraphs_map: DashMap::new(),
      settings,
    }
  }

  pub fn get_tx_channel(
    &self,
    subgraph_name: &SubgraphName,
  ) -> mpsc::Sender<AugGraphOp> {
    log_trace!();

    match self.subgraphs_map.get(subgraph_name) {
      Some(subgraph) => subgraph.op_sender.clone(),
      None => self
        .subgraphs_map
        .entry(subgraph_name.clone())
        .or_insert((|| {
          log_trace!("Create subgraph");
          GraphProcessor::new(
            AugGraph::new(self.settings.clone()),
            self.settings.sleep_duration_after_publish_ms,
            self.settings.subgraph_queue_capacity,
          )
        })())
        .op_sender
        .clone(),
    }
  }

  async fn send_op(
    &self,
    subgraph_name: &SubgraphName,
    op: AugGraphOp,
  ) -> Response {
    //  NOTE: Duplicated logic with `send_op_sync`.

    log_trace!();

    if self.get_tx_channel(subgraph_name).send(op).await.is_ok() {
      Response::Ok
    } else {
      Response::Fail
    }
  }

  pub fn process_read<F>(
    &self,
    subgraph_name: &SubgraphName,
    read_function: F,
  ) -> Response
  where
    F: FnOnce(&AugGraph) -> Response,
  {
    log_trace!();

    let subgraph = match self.subgraphs_map.get(subgraph_name) {
      Some(subgraph) => {
        log_verbose!("Found subgraph for name: {:?}", subgraph_name);
        subgraph
      },
      None => {
        log_warning!("Subgraph not found for name: {:?}", subgraph_name);
        return Response::Fail;
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
        return Response::Fail;
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
    //  NOTE: Duplicated logic with `process_request_sync`.
    //
    //  FIXME: No need to clone here, but borrow checker!!!

    log_trace!();

    let data = req.data.clone();

    match data {
      ReqData::WriteEdge(data) => {
        self.process_write_edge(&req.subgraph, &data).await
      },
      ReqData::WriteCalculate(data) => {
        self
          .send_op(
            &req.subgraph,
            AugGraphOp::WriteCalculate(OpWriteCalculate {
              ego: data.ego.clone(),
            }),
          )
          .await
      },
      ReqData::WriteCreateContext => {
        self.insert_subgraph_if_does_not_exist(&req.subgraph);
        Response::Ok
      },
      ReqData::WriteDeleteEdge(data) => {
        self
          .process_write_edge(
            &req.subgraph,
            &OpWriteEdge {
              src:       data.src,
              dst:       data.dst,
              amount:    0.0,
              magnitude: data.index as u32,
            },
          )
          .await
      },
      ReqData::WriteDeleteNode(_) => {
        log_warning!("Delete node request ignored!");
        Response::Ok
      },
      ReqData::WriteZeroOpinion(data) => {
        self
          .send_op(&req.subgraph, AugGraphOp::WriteZeroOpinion(data.clone()))
          .await
      },
      ReqData::WriteReset => {
        self.subgraphs_map.clear();
        Response::Ok
      },
      ReqData::WriteRecalculateZero => {
        self
          .send_op(&req.subgraph, AugGraphOp::WriteRecalculateZero)
          .await
      },
      ReqData::WriteRecalculateClustering => {
        self
          .send_op(&req.subgraph, AugGraphOp::WriteRecalculateClustering)
          .await
      },
      ReqData::WriteFetchNewEdges(_) => {
        self.process_read(&req.subgraph, |_| {
          log_warning!("Fetch new edges request ignored!");
          Response::NewEdges(ResNewEdges {
            new_edges: vec![],
          })
        })
      },
      ReqData::WriteNewEdgesFilter(_) => {
        self.process_read(&req.subgraph, |_| {
          log_warning!("New edges filter request ignored!");
          Response::Ok
        })
      },
      ReqData::ReadNewEdgesFilter(_) => {
        self.process_read(&req.subgraph, |_| {
          log_warning!("New edges filter request ignored!");
          Response::NewEdgesFilter(ResNewEdgesFilter {
            bytes: vec![],
          })
        })
      },
      ReqData::ReadScores(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_scores(data),
          })
        })
      },
      ReqData::ReadNodeScore(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_node_score(data),
          })
        })
      },
      ReqData::ReadGraph(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Graph(ResGraph {
            graph: aug_graph.read_graph(data),
          })
        })
      },
      ReqData::ReadNeighbors(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_neighbors(data),
          })
        })
      },
      ReqData::ReadNodeList => self.process_read(&req.subgraph, |aug_graph| {
        Response::NodeList(ResNodeList {
          nodes: aug_graph
            .nodes
            .id_to_info
            .iter()
            .map(|info| (info.name.clone(),))
            .collect(),
        })
      }),
      ReqData::ReadEdges => self.process_read(&req.subgraph, |aug_graph| {
        let mut edges = vec![];
        edges.reserve(aug_graph.nodes.id_to_info.len() * 2);

        for (src_id, info) in aug_graph.nodes.id_to_info.iter().enumerate() {
          if let Some(data) = aug_graph.mr.graph.get_node_data(src_id) {
            let src_name = &info.name;

            for (dst_id, weight) in data.get_outgoing_edges() {
              match aug_graph.nodes.get_by_id(dst_id) {
                Some(x) => edges.push(EdgeResult {
                  src: src_name.to_string(),
                  dst: x.name.clone(),
                  weight,
                }),
                None => log_error!("Node does not exist: {}", dst_id),
              }
            }
          };
        }

        Response::Edges(ResEdges {
          edges,
        })
      }),
      ReqData::ReadConnected(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          match aug_graph.nodes.get_by_name(&data.node) {
            Some(src) => Response::Connections(ResConnections {
              connections: aug_graph
                .mr
                .graph
                .get_node_data(src.id)
                .unwrap()
                .get_outgoing_edges()
                .map(|(dst_id, _)| ConnectionResult {
                  src: data.node.clone(),
                  dst: aug_graph.nodes.get_by_id(dst_id).unwrap().name.clone(),
                })
                .collect(),
            }),
            None => {
              log_error!("Node not found: {:?}", data.node);
              Response::Fail
            },
          }
        })
      },
      ReqData::ReadMutualScores(data) => {
        self.process_read(&req.subgraph, |aug_graph| {
          Response::Scores(ResScores {
            scores: aug_graph.read_mutual_scores(data),
          })
        })
      },
    }
  }

  async fn process_write_edge(
    &self,
    subgraph_name: &SubgraphName,
    data: &OpWriteEdge,
  ) -> Response {
    //  NOTE: Duplicated logic with `process_write_edge_sync`.

    log_trace!("{:?} {:?}", subgraph_name, data);

    if data.src == data.dst {
      log_error!("Self-reference is not allowed.");
      return Response::Fail;
    }

    let src_kind_opt = node_kind_from_prefix(&data.src);
    let dst_kind_opt = node_kind_from_prefix(&data.dst);

    match (src_kind_opt, dst_kind_opt) {
      (Some(NodeKind::User), Some(NodeKind::User)) => {
        self
          .process_user_to_user_edge(
            subgraph_name,
            &data.src,
            &data.dst,
            data.amount,
            data.magnitude,
          )
          .await
      },

      (Some(NodeKind::User), Some(NodeKind::PollVariant)) => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::SetUserVote(OpSetUserVote {
              user_id:    data.src.clone(),
              variant_id: data.dst.clone(),
              amount:     data.amount,
            }),
          )
          .await
      },
      (Some(NodeKind::PollVariant), Some(NodeKind::Poll)) => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::AddPollVariant(OpAddPollVariant {
              poll_id:    data.dst.clone(),
              variant_id: data.src.clone(),
            }),
          )
          .await
      },
      (Some(src_kind), Some(dst_kind))
        if src_kind == NodeKind::PollVariant
          || src_kind == NodeKind::Poll
          || dst_kind == NodeKind::PollVariant
          || dst_kind == NodeKind::Poll =>
      {
        log_error!("Unexpected edge type: {:?} -> {:?} in context {:?}. No action taken.", src_kind_opt, dst_kind_opt, subgraph_name);
        Response::Fail
      },
      _ => {
        self
          .send_op(
            subgraph_name,
            AugGraphOp::WriteEdge(OpWriteEdge {
              src:       data.src.clone(),
              dst:       data.dst.clone(),
              amount:    data.amount,
              magnitude: data.magnitude,
            }),
          )
          .await
      },
    }
  }

  pub fn insert_subgraph_if_does_not_exist(
    &self,
    subgraph_name: &SubgraphName,
  ) {
    log_trace!();

    //  FIXME: Cleanup! Code duplication here, but generic types are tricky.
    self
      .subgraphs_map
      .entry(subgraph_name.clone())
      .or_insert((|| {
        log_trace!("Create subgraph");
        GraphProcessor::new(
          AugGraph::new(self.settings.clone()),
          self.settings.sleep_duration_after_publish_ms,
          self.settings.subgraph_queue_capacity,
        )
      })());
  }

  async fn process_user_to_user_edge(
    &self,
    subgraph_name: &SubgraphName,
    src: &NodeName,
    dst: &NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) -> Response {
    //  NOTE: Duplicated logic with `process_user_to_user_edge_sync`.

    log_trace!();

    self.insert_subgraph_if_does_not_exist(subgraph_name);

    let mut join_set = JoinSet::new();

    for ref_multi in self.subgraphs_map.iter() {
      let subgraph = ref_multi.value();
      let op_sender = subgraph.op_sender.clone();
      let src = src.clone();
      let dst = dst.clone();

      join_set.spawn(async move {
        op_sender
          .send(AugGraphOp::WriteEdge(OpWriteEdge {
            src,
            dst,
            amount,
            magnitude,
          }))
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

    if all_successful {
      Response::Ok
    } else {
      Response::Fail
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nonblocking() {
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
}
