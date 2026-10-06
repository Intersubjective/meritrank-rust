//! Service-level requirements of negative edges as walls (NEGATIVE_EDGES_FEATURE.md): encoding and
//! validation (R1–R3, R20), exact storage (R19), contexts (R7), mr_graph normalisation (R24),
//! settings (R23) and end-to-end scores.

use meritrank_service::data::{
  BulkEdge, EdgeResult, GraphResult, OpReadGraph, OpReadNodeScore, OpWriteBulkEdges,
  OpWriteEdge, ReqData, Request, ResEdges, ResGraph, ResScores, Response,
};
use meritrank_service::settings::{BlameRadius, Settings};
use meritrank_service::state_manager::MultiGraphProcessor;

fn settings() -> Settings {
  Settings {
    num_walks: 2_000,
    zero_opinion_factor: 0.0,
    ..Settings::default()
  }
}

async fn request(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  data: ReqData,
) -> Response {
  proc
    .process_request(&Request {
      subgraph: subgraph.into(),
      data,
    })
    .await
}

fn edge(
  src: &str,
  dst: &str,
  amount: f64,
  magnitude: u32,
) -> OpWriteEdge {
  OpWriteEdge {
    src: src.into(),
    dst: dst.into(),
    amount,
    magnitude,
  }
}

async fn write(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  src: &str,
  dst: &str,
  amount: f64,
) -> Response {
  request(proc, subgraph, ReqData::WriteEdge(edge(src, dst, amount, 0))).await
}

async fn sync(proc: &MultiGraphProcessor) {
  assert!(matches!(request(proc, "", ReqData::Sync(0)).await, Response::Ok));
}

async fn edges(
  proc: &MultiGraphProcessor,
  subgraph: &str,
) -> Vec<(String, String, f64)> {
  match request(proc, subgraph, ReqData::ReadEdges).await {
    Response::Edges(ResEdges { edges }) => {
      let mut v: Vec<_> = edges
        .into_iter()
        .map(|e: EdgeResult| (e.src, e.dst, e.weight))
        .collect();
      v.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
      v
    },
    other => panic!("expected edges, got {:?}", other),
  }
}

fn weight_of(
  edges: &[(String, String, f64)],
  src: &str,
  dst: &str,
) -> Option<f64> {
  edges
    .iter()
    .find(|(s, d, _)| s == src && d == dst)
    .map(|(_, _, w)| *w)
}

async fn node_score(
  proc: &MultiGraphProcessor,
  ego: &str,
  target: &str,
) -> f64 {
  match request(
    proc,
    "",
    ReqData::ReadNodeScore(OpReadNodeScore {
      ego:    ego.into(),
      target: target.into(),
    }),
  )
  .await
  {
    Response::Scores(ResScores { scores }) => scores.first().map_or(0.0, |s| s.score),
    other => panic!("expected scores, got {:?}", other),
  }
}

// ---------------------------------------------------------------------------
// R1–R3, R20: encoding and validation
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_walls_are_rejected() {
  let proc = MultiGraphProcessor::new(settings());
  assert!(matches!(write(&proc, "", "U1", "U2", -0.5).await, Response::Ok));
  // One node class and isolated contexts (D14): a wall between any two nodes, in any context.
  assert!(matches!(write(&proc, "X", "U1", "U3", -0.5).await, Response::Ok));
  assert!(matches!(write(&proc, "", "U1", "B1", -0.5).await, Response::Ok));
  assert!(matches!(write(&proc, "", "B1", "U1", -0.5).await, Response::Ok));
  // Self-edge (R3) and non-finite weights stay invalid.
  assert!(matches!(write(&proc, "", "U1", "U1", -0.5).await, Response::Fail));
  assert!(matches!(write(&proc, "", "U1", "U2", f64::NAN).await, Response::Fail));
  assert!(matches!(write(&proc, "", "U1", "U2", f64::INFINITY).await, Response::Fail));
  sync(&proc).await;
  let now = edges(&proc, "").await;
  assert_eq!(
    now,
    vec![
      ("B1".into(), "U1".into(), -0.5),
      ("U1".into(), "B1".into(), -0.5),
      ("U1".into(), "U2".into(), -0.5),
    ],
    "{:?}",
    now
  );
  assert_eq!(edges(&proc, "X").await, vec![("U1".into(), "U3".into(), -0.5)]);
}

/// R20: a batch with one invalid wall changes nothing, and the service is not left loading.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bulk_with_an_invalid_wall_changes_nothing() {
  let proc = MultiGraphProcessor::new(settings());
  write(&proc, "", "U1", "U2", 1.0).await;
  sync(&proc).await;
  let bulk = |edges: Vec<(&str, &str, f64, &str)>| {
    ReqData::WriteBulkEdges(OpWriteBulkEdges {
      edges: edges
        .into_iter()
        .map(|(s, d, w, c)| BulkEdge {
          src:       s.into(),
          dst:       d.into(),
          amount:    w,
          magnitude: 0,
          context:   c.into(),
        })
        .collect(),
    })
  };
  let resp = request(
    &proc,
    "",
    bulk(vec![("U5", "U6", 1.0, ""), ("U5", "U5", -1.0, "X"), ("U6", "U7", 1.0, "")]),
  )
  .await;
  assert!(matches!(resp, Response::Fail));
  assert_eq!(edges(&proc, "").await, vec![("U1".into(), "U2".into(), 1.0)]);

  // A valid batch with walls loads them exactly.
  let resp = request(
    &proc,
    "",
    bulk(vec![("U5", "U6", 1.0, ""), ("U5", "U7", -0.25, ""), ("B1", "U6", 1.0, "X")]),
  )
  .await;
  assert!(matches!(resp, Response::Ok));
  assert_eq!(weight_of(&edges(&proc, "").await, "U5", "U7"), Some(-0.25));
  // Isolated contexts: X holds only its own edge.
  assert_eq!(edges(&proc, "X").await, vec![("B1".into(), "U6".into(), 1.0)]);
}

// ---------------------------------------------------------------------------
// R19: exact storage
// ---------------------------------------------------------------------------

/// A soft wall keeps its exact weight through VSIDS rescales of the same node's positive edges,
/// and a wall far below the pruning threshold is not pruned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn walls_are_exempt_from_vsids() {
  let proc = MultiGraphProcessor::new(settings());
  write(&proc, "", "U1", "U2", -0.37).await;
  write(&proc, "", "U1", "U3", -1e-4).await;
  for (i, magnitude) in [0u32, 50, 200, 400, 800, 1600].iter().enumerate() {
    request(
      &proc,
      "",
      ReqData::WriteEdge(edge("U1", &format!("U{}", 10 + i), 1.0, *magnitude)),
    )
    .await;
  }
  // A wall written after the node's magnitude scale has grown must not be scaled down.
  write(&proc, "", "U1", "U4", -0.5).await;
  request(&proc, "", ReqData::WriteEdge(edge("U1", "U20", 1.0, 3200))).await;
  sync(&proc).await;
  let now = edges(&proc, "").await;
  assert_eq!(weight_of(&now, "U1", "U2"), Some(-0.37), "{:?}", now);
  assert_eq!(weight_of(&now, "U1", "U3"), Some(-1e-4), "{:?}", now);
  assert_eq!(weight_of(&now, "U1", "U4"), Some(-0.5), "{:?}", now);
}

/// Sign transitions through the service: trust → wall → trust; the wall write leaves the node's
/// other positive edges alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sign_transitions() {
  let proc = MultiGraphProcessor::new(settings());
  write(&proc, "", "U1", "U2", 2.0).await;
  write(&proc, "", "U1", "U3", 1.0).await;
  sync(&proc).await;
  let other = weight_of(&edges(&proc, "").await, "U1", "U3");
  write(&proc, "", "U1", "U2", -0.8).await;
  sync(&proc).await;
  let now = edges(&proc, "").await;
  assert_eq!(weight_of(&now, "U1", "U2"), Some(-0.8));
  assert_eq!(weight_of(&now, "U1", "U3"), other, "the wall write changed a positive edge");
  write(&proc, "", "U1", "U2", 1.5).await;
  sync(&proc).await;
  assert!(weight_of(&edges(&proc, "").await, "U1", "U2").unwrap() > 0.0);
}

// ---------------------------------------------------------------------------
// R7: contexts
// ---------------------------------------------------------------------------

/// Walls stay in the context they are written to (D14: isolated contexts), with their exact
/// weight; incremental and bulk loading agree.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn walls_follow_contexts() {
  let proc = MultiGraphProcessor::new(settings());
  request(&proc, "A", ReqData::WriteCreateContext).await;
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "", "U1", "U3", -0.6).await;
  write(&proc, "A", "U1", "U3", -0.2).await;
  request(&proc, "B", ReqData::WriteCreateContext).await;
  write(&proc, "C", "B1", "U1", 1.0).await; // implicit creation
  sync(&proc).await;
  assert_eq!(weight_of(&edges(&proc, "").await, "U1", "U3"), Some(-0.6));
  assert_eq!(weight_of(&edges(&proc, "A").await, "U1", "U3"), Some(-0.2));
  for ctx in ["B", "C"] {
    assert_eq!(weight_of(&edges(&proc, ctx).await, "U1", "U3"), None, "context {ctx:?}");
  }

  let bulk = MultiGraphProcessor::new(settings());
  let resp = request(
    &bulk,
    "",
    ReqData::WriteBulkEdges(OpWriteBulkEdges {
      edges: vec![
        BulkEdge {
          src:       "U1".into(),
          dst:       "U2".into(),
          amount:    1.0,
          magnitude: 0,
          context:   "".into(),
        },
        BulkEdge {
          src:       "U1".into(),
          dst:       "U3".into(),
          amount:    -0.6,
          magnitude: 0,
          context:   "".into(),
        },
        BulkEdge {
          src:       "U1".into(),
          dst:       "U3".into(),
          amount:    -0.2,
          magnitude: 0,
          context:   "A".into(),
        },
        BulkEdge {
          src:       "B1".into(),
          dst:       "U1".into(),
          amount:    1.0,
          magnitude: 0,
          context:   "C".into(),
        },
      ],
    }),
  )
  .await;
  assert!(matches!(resp, Response::Ok));
  sync(&bulk).await;
  for ctx in ["", "A", "C"] {
    assert_eq!(edges(&bulk, ctx).await, edges(&proc, ctx).await, "bulk vs incremental {ctx:?}");
  }
}

// ---------------------------------------------------------------------------
// R24: mr_graph normalisation
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graph_weights_ignore_walls() {
  let proc = MultiGraphProcessor::new(settings());
  write(&proc, "", "U1", "U2", 3.0).await;
  write(&proc, "", "U1", "U3", 1.0).await;
  write(&proc, "", "U1", "U4", -1.0).await;
  sync(&proc).await;
  let graph = match request(
    &proc,
    "",
    ReqData::ReadGraph(OpReadGraph {
      ego:           "U1".into(),
      focus:         "U1".into(),
      positive_only: true,
      index:         0,
      count:         100,
    }),
  )
  .await
  {
    Response::Graph(ResGraph { graph }) => graph,
    other => panic!("expected graph, got {:?}", other),
  };
  let w = |dst: &str| {
    graph
      .iter()
      .find(|r: &&GraphResult| r.src == "U1" && r.dst == dst)
      .map(|r| r.weight)
  };
  assert!((w("U2").unwrap() - 0.75).abs() < 1e-9, "{:?}", graph);
  assert!((w("U3").unwrap() - 0.25).abs() < 1e-9, "{:?}", graph);
}

// ---------------------------------------------------------------------------
// R23: settings
// ---------------------------------------------------------------------------

#[test]
fn settings_are_validated() {
  assert!(Settings::default().validate().is_ok());
  for bad in [
    Settings { alpha: 1.0, ..Settings::default() },
    Settings { alpha: 0.0, ..Settings::default() },
    Settings { discredit_lambda: -0.1, ..Settings::default() },
    Settings { discredit_lambda: f64::NAN, ..Settings::default() },
    Settings { blame_decay: 1.5, ..Settings::default() },
    Settings { num_walks: 0, ..Settings::default() },
  ] {
    assert!(bad.validate().is_err());
  }
  assert_eq!(Settings::default().discredit_lambda, 0.0);
  assert_eq!(Settings::default().blame_radius, BlameRadius::Prefix);
}

// ---------------------------------------------------------------------------
// End to end
// ---------------------------------------------------------------------------

/// U1 → U2 → U3 with a hard wall U1 ⊣ U3: after a sync, U3 scores 0 in U1's frame and U2 keeps
/// only the walks that stop at it; removing the wall restores U3.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wall_end_to_end() {
  let proc = MultiGraphProcessor::new(settings());
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "", "U2", "U3", 1.0).await;
  sync(&proc).await;
  let before = node_score(&proc, "U1", "U3").await;
  assert!((before - 0.85 * 0.85).abs() < 0.05, "{before}");

  write(&proc, "", "U1", "U3", -1.0).await;
  sync(&proc).await;
  assert_eq!(node_score(&proc, "U1", "U3").await, 0.0);
  let u2 = node_score(&proc, "U1", "U2").await;
  assert!((u2 - 0.85 * 0.15).abs() < 0.04, "{u2}");

  write(&proc, "", "U1", "U3", 0.0).await;
  sync(&proc).await;
  let after = node_score(&proc, "U1", "U3").await;
  assert!((after - 0.85 * 0.85).abs() < 0.05, "{after}");
}

/// With discredit the voucher and the wall go negative end to end.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discredit_end_to_end() {
  let proc = MultiGraphProcessor::new(Settings {
    discredit_lambda: 1.0,
    ..settings()
  });
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "", "U2", "U3", 1.0).await;
  write(&proc, "", "U1", "U3", -1.0).await;
  sync(&proc).await;
  assert!(node_score(&proc, "U1", "U3").await < 0.0);
  assert!(node_score(&proc, "U1", "U2").await < 0.0);
}
