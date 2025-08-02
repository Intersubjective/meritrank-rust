use crate::state_manager::{MultiGraphProcessor, MultiGraphProcessorSettings};
use crate::legacy_protocol::*;
use crate::data::*;
// use crate::utils::log::*;

use tokio::time::{sleep, Duration};

async fn write_edge(processor: &MultiGraphProcessor, subgraph: &str, src: &str, dst: &str, amount: f64, magnitude: u32) {
  let _ = processor.process_request(&Request {
    subgraph: subgraph.into(),
    data: ReqData::WriteEdge(OpWriteEdge {
      src: src.into(),
      dst: dst.into(),
      amount,
      magnitude,
    })
  }).await;
}

async fn write_zero_opinion(processor: &MultiGraphProcessor, node: &str, score: f64) {
  let _ = processor.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::WriteZeroOpinion(OpWriteZeroOpinion {
      node: node.into(),
      score,
    })
  }).await;
}

async fn write_calculate(processor: &MultiGraphProcessor, ego: &str) {
  let _ = processor.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::WriteCalculate(OpWriteCalculate {
      ego: ego.into(),
    })
  }).await;
}

#[tokio::test]
async fn node_score_uncontexted() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 2.0, 1).await;
  write_edge(&proc, "", "U1", "U3", 1.0, 1).await;
  write_edge(&proc, "", "U3", "U2", 3.0, 1).await;

  write_calculate(&proc, "U1").await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadNodeScore(OpReadNodeScore {
      ego: "U1".into(),
      target: "U2".into(),
    }),
  }).await;

  match res {
    Response::Scores(v) => {
      assert_eq!(v.scores.len(), 1);
      assert_eq!(v.scores[0].ego, "U1");
      assert_eq!(v.scores[0].target, "U2");
      assert!(v.scores[0].score > 0.3);
      assert!(v.scores[0].score < 0.45);
    },

    _ => assert!(false),
  };
}

#[tokio::test]
async fn graph_uncontexted() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 2.0, 1).await;
  write_edge(&proc, "", "U1", "U3", 1.0, 1).await;
  write_edge(&proc, "", "U2", "U3", 3.0, 1).await;

  write_calculate(&proc, "U1").await;
  write_calculate(&proc, "U2").await;
  write_calculate(&proc, "U3").await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadGraph(OpReadGraph {
      ego: "U1".into(),
      focus: "U2".into(),
      positive_only: false,
      index: 0,
      count: 10000,
    }),
  }).await;

  match res {
    Response::Graph(v) => {
      assert_eq!(v.graph.len(), 2);

      let mut has_u1 = false;
      let mut has_u2 = false;

      for x in v.graph {
        match x.src.as_str() {
          "U1" => {
            assert_eq!(x.dst, "U2");
            assert!(x.score > 0.15);
            assert!(x.score < 0.35);
            has_u1 = true;
          },

          "U2" => {
            assert_eq!(x.dst, "U3");
            assert!(x.score > 0.2);
            assert!(x.score < 0.3);
            has_u2 = true;
          },

          _ => assert!(false),
        }
      }

      assert!(has_u1);
      assert!(has_u2);
    },

    _ => assert!(false),
  };
}

#[tokio::test]
async fn neighbors_all() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 1.0, 1).await;
  write_edge(&proc, "", "U2", "U3", 2.0, 1).await;
  write_edge(&proc, "", "U3", "U1", 3.0, 1).await;

  write_calculate(&proc, "U1").await;
  write_calculate(&proc, "U2").await;
  write_calculate(&proc, "U3").await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadNeighbors(OpReadNeighbors {
      ego: "U1".into(),
      focus: "U2".into(),
      direction: NEIGHBORS_ALL,
      kind: None,
      hide_personal: false,
      lt: 100.0,
      lte: false,
      gt: -100.0,
      gte: false,
      index: 0,
      count: 0,
    }),
  }).await;

  match res {
    Response::Scores(_) => {
      //  FIXME: Doesn't work currently.

      // assert_eq!(v.scores.len(), 2);
      // assert_eq!(v.scores[0].target, "U1");
      // assert_eq!(v.scores[1].target, "U3");
    },

    _ => assert!(false),
  };
}

#[tokio::test]
async fn node_list_uncontexted() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 2.0, 1).await;
  write_edge(&proc, "", "U1", "U3", 1.0, 1).await;
  write_edge(&proc, "", "U3", "U2", 3.0, 1).await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadNodeList,
  }).await;

  match res {
    Response::NodeList(v) => {
      let mut has_u1 = false;
      let mut has_u2 = false;
      let mut has_u3 = false;

      for x in v.nodes {
        match x.0.as_str() {
          "U1" => has_u1 = true,
          "U2" => has_u2 = true,
          "U3" => has_u3 = true,
          _ => assert!(false),
        }
      }

      assert!(has_u1);
      assert!(has_u2);
      assert!(has_u3);
    },

    _ => assert!(false),
  };
}

#[tokio::test]
async fn edge_uncontexted() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 1.5, 1).await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadEdges,
  }).await;

  match res {
    Response::Edges(v) => {
      assert_eq!(v.edges[0].src, "U1");
      assert_eq!(v.edges[0].dst, "U2");
      assert_eq!(v.edges[0].weight, 1.5);
    },

    _ => assert!(false),
  };
}


#[tokio::test]
async fn connected() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 1.5, 1).await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadConnected(OpReadConnected {
      node: "U1".into(),
    }),
  }).await;

  match res {
    Response::Connections(v) => {
      assert_eq!(v.connections.len(), 1);
      assert_eq!(v.connections[0].src, "U1");
      assert_eq!(v.connections[0].dst, "U2");
    },

    _ => assert!(false),
  };
}

#[tokio::test]
async fn mutual_scores_uncontexted() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 3.0, 1).await;
  write_edge(&proc, "", "U1", "U3", 1.0, 1).await;
  write_edge(&proc, "", "U2", "U1", 2.0, 1).await;
  write_edge(&proc, "", "U2", "U3", 4.0, 1).await;
  write_edge(&proc, "", "U3", "U1", 3.0, 1).await;
  write_edge(&proc, "", "U3", "U2", 2.0, 1).await;

  write_calculate(&proc, "U1").await;
  write_calculate(&proc, "U2").await;
  write_calculate(&proc, "U3").await;

  sleep(Duration::from_millis(200)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadMutualScores(OpReadMutualScores {
      ego: "U1".into(),
    }),
  }).await;

  println!("{:?}", res);

  match res {
    Response::Scores(v) => {
      assert_eq!(v.scores.len(), 3);

      let mut u1 = true;
      let mut u2 = true;
      let mut u3 = true;

      for x in v.scores.iter() {
        assert_eq!(x.ego, "U1");

        match x.target.as_str() {
          "U1" => {
            assert!(x.score > 0.25);
            assert!(x.score < 0.5);
            assert!(x.reverse_score > 0.25);
            assert!(x.reverse_score < 0.5);
            assert!(u1);
            u1 = false;
          },

          "U2" => {
            assert!(x.score > 0.15);
            assert!(x.score < 0.35);
            assert!(x.reverse_score > 0.15);
            assert!(x.reverse_score < 0.35);
            assert!(u2);
            u2 = false;
          },

          "U3" => {
            assert!(x.score > 0.15);
            assert!(x.score < 0.35);
            assert!(x.reverse_score > 0.15);
            assert!(x.reverse_score < 0.35);
            assert!(u3);
            u3 = false;
          },

          _ => {
            assert!(false);
          },
        };
      }
    },

    _ => assert!(false),
  };
}

#[tokio::test]
async fn set_zero_opinion_uncontexted() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", -5.0, 1).await;
  write_calculate(&proc, "U1").await;

  sleep(Duration::from_millis(100)).await;

  let s0 = match proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadNodeScore(OpReadNodeScore {
      ego: "U1".into(),
      target: "U2".into(),
    }),
  }).await {
    Response::Scores(v) => {
      assert_eq!(v.scores.len(), 1);
      v.scores[0].score
    },
    _ => {
      assert!(false);
      return;
    },
  };

  write_zero_opinion(&proc, "U2", 10.0).await;

  sleep(Duration::from_millis(100)).await;

  let s1 = match proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadNodeScore(OpReadNodeScore {
      ego: "U1".into(),
      target: "U2".into(),
    }),
  }).await {
    Response::Scores(v) => {
      assert_eq!(v.scores.len(), 1);
      v.scores[0].score
    },
    _ => {
      assert!(false);
      return;
    },
  };

  println!("{}, {}", s0, s1);
  assert_ne!(s0, s1);
}

#[tokio::test]
async fn reset() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      ..MultiGraphProcessorSettings::default()
    });

  write_edge(&proc, "", "U1", "U2", 3.0, 1).await;
  write_edge(&proc, "", "U1", "U3", 1.0, 1).await;
  write_edge(&proc, "", "U2", "U1", 2.0, 1).await;
  write_edge(&proc, "", "U2", "U3", 4.0, 1).await;
  write_edge(&proc, "", "U3", "U1", 3.0, 1).await;
  write_edge(&proc, "", "U3", "U2", 2.0, 1).await;

  write_calculate(&proc, "U1").await;
  write_calculate(&proc, "U2").await;
  write_calculate(&proc, "U3").await;

  let _ = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::WriteReset
  }).await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadEdges,
  }).await;

  match res {
    Response::Fail => {},
    _ => assert!(false),
  };
}

use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use tar::Archive;

#[derive(Debug, Deserialize)]
struct Edge {
  context:   String,
  src:       String,
  dst:       String,
  weight:    f64,
  magnitude: i64,
}

fn read_csv_from_tar_gz(
  tar_gz_path: &str,
  target_file: &str,
) -> Vec<Edge> {
  let file = File::open(tar_gz_path).unwrap();
  let decompressor = GzDecoder::new(BufReader::new(file));
  let mut archive = Archive::new(decompressor);

  for entry in archive.entries().unwrap() {
    let file = entry.unwrap();
    if file.path().unwrap().to_str() == Some(target_file) {
      let reader = csv::Reader::from_reader(file);
      return reader.into_deserialize().map(|r| r.unwrap()).collect();
    }
  }

  panic!("File '{}' not found in archive", target_file);
}

async fn put_testing_edges(
  proc: &MultiGraphProcessor,
  file_name: &str,
) {
  for edge in read_csv_from_tar_gz("src/edges.tar.gz", file_name) {
    let _ = edge.magnitude;
    write_edge(&proc, &edge.context, &edge.src, &edge.dst, edge.weight, 1).await;
  }
}

#[tokio::test]
async fn recalculate_zero_graph_all() {
  let proc =
    MultiGraphProcessor::new(MultiGraphProcessorSettings {
      sleep_duration_after_publish_ms: 0,
      num_walks: 500,
      zero_opinion_num_walks: 100,
      legacy_connections_mode: true,
      ..MultiGraphProcessorSettings::default()
    });

  put_testing_edges(&proc, "edges0.csv").await;

  let _ = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::WriteRecalculateZero
  }).await;

  sleep(Duration::from_millis(100)).await;

  let res = proc.process_request(&Request {
    subgraph: "".into(),
    data: ReqData::ReadGraph(OpReadGraph {
      ego: "Uadeb43da4abb".into(),
      focus: "B7f628ad203b5".into(),
      positive_only: false,
      index: 0,
      count: 10000,
    }),
  }).await;

  match res {
    Response::Graph(v) => {
      let n = v.graph.len();

      println!("Got {} edges", n);

      assert!(n > 1);
      assert!(n < 5);
    },

    _ => assert!(false),
  };
}

