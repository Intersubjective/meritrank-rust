//  ============================================================
//
//    WARNING
//
//    Temporary solution for NNG compatibility.
//    There is a lot of duplicated logic here.
//
//  FIXME: Factor out duplicated logic, see `state_manager.rs`.
//
//  ============================================================

use crate::data::*;
use crate::node_registry::*;
use crate::state_manager::*;
use crate::utils::log::*;
use crate::vsids::Magnitude;

use meritrank_core::Weight;
use tokio::sync::mpsc;

use std::thread;
use std::time::Duration;

const SYNC_SLEEP_PERIOD: u64 = 10;

impl MultiGraphProcessor {
  fn process_user_to_user_edge_sync(
    &self,
    subgraph_name: &SubgraphName,
    src: &NodeName,
    dst: &NodeName,
    amount: Weight,
    magnitude: Magnitude,
  ) -> Response {
    log_trace!();

    self.insert_subgraph_if_does_not_exist(subgraph_name);

    for ref_multi in self.subgraphs_map.iter() {
      let subgraph = ref_multi.value();
      let op_sender = subgraph.op_sender.clone();
      let src = src.clone();
      let dst = dst.clone();

      let op = AugGraphOp::WriteEdge(OpWriteEdge {
        src,
        dst,
        amount,
        magnitude,
      });

      loop {
        let s = op_sender.try_send(op.clone());
        match s {
          Ok(_) => break,
          Err(e) => match e {
            mpsc::error::TrySendError::Full(_) => {},
            mpsc::error::TrySendError::Closed(_) => {
              log_error!("Send failed: channel is closed.");
              return Response::Fail;
            },
          },
        };

        thread::sleep(Duration::from_millis(SYNC_SLEEP_PERIOD));
      }
    }

    Response::Ok
  }

  fn send_op_sync(
    &self,
    subgraph_name: &SubgraphName,
    op: AugGraphOp,
  ) -> Response {
    log_trace!();

    let c = self.get_tx_channel(subgraph_name);

    loop {
      let s = c.try_send(op.clone());
      match s {
        Ok(_) => return Response::Ok,
        Err(e) => match e {
          mpsc::error::TrySendError::Full(_) => {},
          mpsc::error::TrySendError::Closed(_) => {
            log_error!("Send failed: channel is closed.");
            return Response::Fail;
          },
        },
      };

      thread::sleep(Duration::from_millis(SYNC_SLEEP_PERIOD));
    }
  }

  fn process_write_edge_sync(
    &self,
    subgraph_name: &SubgraphName,
    data: &OpWriteEdge,
  ) -> Response {
    log_trace!("{:?} {:?}", subgraph_name, data);

    if data.src == data.dst {
      log_error!("Self-reference is not allowed.");
      return Response::Fail;
    }

    let src_kind_opt = node_kind_from_prefix(&data.src);
    let dst_kind_opt = node_kind_from_prefix(&data.dst);

    match (src_kind_opt, dst_kind_opt) {
      (Some(NodeKind::User), Some(NodeKind::User)) => self
        .process_user_to_user_edge_sync(
          subgraph_name,
          &data.src,
          &data.dst,
          data.amount,
          data.magnitude,
        ),

      (Some(NodeKind::User), Some(NodeKind::PollVariant)) => self.send_op_sync(
        subgraph_name,
        AugGraphOp::SetUserVote(OpSetUserVote {
          user_id:    data.src.clone(),
          variant_id: data.dst.clone(),
          amount:     data.amount,
        }),
      ),
      (Some(NodeKind::PollVariant), Some(NodeKind::Poll)) => self.send_op_sync(
        subgraph_name,
        AugGraphOp::AddPollVariant(OpAddPollVariant {
          poll_id:    data.dst.clone(),
          variant_id: data.src.clone(),
        }),
      ),
      (Some(src_kind), Some(dst_kind))
        if src_kind == NodeKind::PollVariant
          || src_kind == NodeKind::Poll
          || dst_kind == NodeKind::PollVariant
          || dst_kind == NodeKind::Poll =>
      {
        log_error!("Unexpected edge type: {:?} -> {:?} in context {:?}. No action taken.", src_kind_opt, dst_kind_opt, subgraph_name);
        Response::Fail
      },
      _ => self.send_op_sync(
        subgraph_name,
        AugGraphOp::WriteEdge(OpWriteEdge {
          src:       data.src.clone(),
          dst:       data.dst.clone(),
          amount:    data.amount,
          magnitude: data.magnitude,
        }),
      ),
    }
  }

  pub fn process_request_sync(
    &self,
    req: &Request,
  ) -> Response {
    let data = req.data.clone();

    match data {
      ReqData::WriteEdge(data) => {
        self.process_write_edge_sync(&req.subgraph, &data)
      },
      ReqData::WriteCalculate(data) => self.send_op_sync(
        &req.subgraph,
        AugGraphOp::WriteCalculate(OpWriteCalculate {
          ego: data.ego.clone(),
        }),
      ),
      ReqData::WriteCreateContext => {
        self.insert_subgraph_if_does_not_exist(&req.subgraph);
        Response::Ok
      },
      ReqData::WriteDeleteEdge(data) => self.process_write_edge_sync(
        &req.subgraph,
        &OpWriteEdge {
          src:       data.src,
          dst:       data.dst,
          amount:    0.0,
          magnitude: data.index as u32,
        },
      ),
      ReqData::WriteDeleteNode(_) => {
        log_warning!("Delete node request ignored!");
        Response::Ok
      },
      ReqData::WriteZeroOpinion(data) => self.send_op_sync(
        &req.subgraph,
        AugGraphOp::WriteZeroOpinion(data.clone()),
      ),
      ReqData::WriteReset => {
        self.subgraphs_map.clear();
        Response::Ok
      },
      ReqData::WriteRecalculateZero => {
        self.send_op_sync(&req.subgraph, AugGraphOp::WriteRecalculateZero)
      },
      ReqData::WriteRecalculateClustering => {
        self.send_op_sync(&req.subgraph, AugGraphOp::WriteRecalculateClustering)
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
}
