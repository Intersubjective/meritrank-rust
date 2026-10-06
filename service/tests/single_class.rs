//! D14 part I: one node class and isolated contexts (JOURNAL.md D14).
//!
//! - every node is a plain node: any non-empty name, any node as an ego, no owners;
//! - the wire keeps `kind` and `hide_personal` (old clients' bytes still decode) and the service
//!   ignores them;
//! - contexts are isolated: a write reaches only the context it names; bulk loads the same.

use bincode::{config::standard, decode_from_slice};
use meritrank_service::data::{
  BulkEdge, EdgeResult, FilterOptions, NodeKind, OpReadMutualScores, OpReadNeighbors,
  OpReadScores, OpWriteBulkEdges, OpWriteEdge, ReqData, Request, ResEdges, ResScores, Response,
  ScoreResult, NEIGHBORS_INBOUND,
};
use meritrank_service::settings::Settings;
use meritrank_service::state_manager::MultiGraphProcessor;

fn settings() -> Settings {
  Settings {
    num_walks: 500,
    zero_opinion_factor: 0.0,
    seed: 7,
    ..Settings::default()
  }
}

async fn request(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  data: ReqData,
) -> Response {
  proc.process_request(&Request { subgraph: subgraph.into(), data }).await
}

async fn write(
  proc: &MultiGraphProcessor,
  subgraph: &str,
  src: &str,
  dst: &str,
  amount: f64,
) -> Response {
  request(
    proc,
    subgraph,
    ReqData::WriteEdge(OpWriteEdge { src: src.into(), dst: dst.into(), amount, magnitude: 0 }),
  )
  .await
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
      let mut v: Vec<_> =
        edges.into_iter().map(|e: EdgeResult| (e.src, e.dst, e.weight)).collect();
      v.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
      v
    },
    other => panic!("expected edges, got {:?}", other),
  }
}

fn scores(r: Response) -> Vec<ScoreResult> {
  match r {
    Response::Scores(ResScores { scores }) => scores,
    other => panic!("expected scores, got {:?}", other),
  }
}

fn key(rows: &[ScoreResult]) -> Vec<(String, u64, u64, usize, usize)> {
  rows
    .iter()
    .map(|r| {
      (r.target.clone(), r.score.to_bits(), r.reverse_score.to_bits(), r.cluster, r.reverse_cluster)
    })
    .collect()
}

// ---------------------------------------------------------------------------
// One node class
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn any_name_is_a_node() {
  let proc = MultiGraphProcessor::new(settings());
  assert!(matches!(write(&proc, "", "alice", "bob", 1.0).await, Response::Ok));
  assert!(matches!(write(&proc, "", "bob", "alice", 1.0).await, Response::Ok));
  assert!(matches!(write(&proc, "", "", "bob", 1.0).await, Response::Fail), "empty name");
  sync(&proc).await;
  let rows = scores(
    request(
      &proc,
      "",
      ReqData::ReadScores(OpReadScores { ego: "alice".into(), score_options: Default::default() }),
    )
    .await,
  );
  assert!(rows.iter().any(|r| r.target == "bob" && r.score > 0.0), "{:?}", rows);
}

/// Former objects (beacons, comments, opinions) are egos and peers like any node: they appear in
/// mutual scores, with a reverse score from their own frame.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn former_objects_are_plain_nodes() {
  let proc = MultiGraphProcessor::new(settings());
  write(&proc, "", "U1", "B1", 1.0).await;
  write(&proc, "", "B1", "U1", 1.0).await;
  write(&proc, "", "U1", "C1", 1.0).await;
  sync(&proc).await;
  let rows = scores(
    request(&proc, "", ReqData::ReadMutualScores(OpReadMutualScores { ego: "U1".into() })).await,
  );
  let b1 = rows.iter().find(|r| r.target == "B1").expect("B1 row");
  assert!(b1.reverse_score > 0.0, "{:?}", b1);
  let c1 = rows.iter().find(|r| r.target == "C1").expect("C1 row");
  assert_eq!(c1.reverse_score, 0.0, "C1 has no path back: {:?}", c1);

  let as_ego = scores(
    request(
      &proc,
      "",
      ReqData::ReadScores(OpReadScores { ego: "B1".into(), score_options: Default::default() }),
    )
    .await,
  );
  assert!(as_ego.iter().any(|r| r.target == "U1" && r.score > 0.0), "{:?}", as_ego);
}

/// The kind filter and `hide_personal` are accepted and ignored.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kind_and_hide_personal_are_ignored() {
  let proc = MultiGraphProcessor::new(settings());
  for (s, d) in [("U1", "B1"), ("U1", "U2"), ("B1", "U2"), ("U2", "C1"), ("C1", "U1")] {
    write(&proc, "", s, d, 1.0).await;
  }
  sync(&proc).await;
  let read = |kind: Option<NodeKind>, hide: bool| ReqData::ReadScores(OpReadScores {
    ego:           "U1".into(),
    score_options: FilterOptions { node_kind: kind, hide_personal: hide, ..Default::default() },
  });
  let base = key(&scores(request(&proc, "", read(None, false)).await));
  assert_eq!(base.len(), 4, "U1, U2, B1, C1: {:?}", base);
  for kind in [Some(NodeKind::User), Some(NodeKind::Beacon), Some(NodeKind::Opinion)] {
    for hide in [false, true] {
      assert_eq!(key(&scores(request(&proc, "", read(kind, hide)).await)), base, "{kind:?} {hide}");
    }
  }
  let neigh = |kind: Option<NodeKind>, hide: bool| ReqData::ReadNeighbors(OpReadNeighbors {
    ego: "U1".into(),
    focus: "U2".into(),
    direction: NEIGHBORS_INBOUND,
    kind,
    hide_personal: hide,
    lt: 100.0,
    lte: false,
    gt: -100.0,
    gte: false,
    index: 0,
    count: 100,
  });
  let base = key(&scores(request(&proc, "", neigh(None, false)).await));
  assert_eq!(base.len(), 2, "U1 and B1 point at U2: {:?}", base);
  assert_eq!(key(&scores(request(&proc, "", neigh(Some(NodeKind::Opinion), true)).await)), base);
}

// ---------------------------------------------------------------------------
// Wire compatibility: bytes of old clients still decode
// ---------------------------------------------------------------------------

/// Requests encoded by meritrank_service 0.11 (generated before D14): every kind variant and both
/// `hide_personal` values, in ReadScores and ReadNeighbors.
const OLD_CLIENT_BYTES: &[(&str, usize, bool, &str)] = &[
  ("SCORES", 0, false, "03637478000255310000000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 0, false, "000f025531025532000000000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 0, true, "03637478000255310001000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 0, true, "000f025531025532000001000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 1, false, "0363747800025531010000000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 1, false, "000f02553102553200010000000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 1, true, "0363747800025531010001000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 1, true, "000f02553102553200010001000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 2, false, "0363747800025531010100000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 2, false, "000f02553102553200010100000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 2, true, "0363747800025531010101000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 2, true, "000f02553102553200010101000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 3, false, "0363747800025531010200000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 3, false, "000f02553102553200010200000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 3, true, "0363747800025531010201000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 3, true, "000f02553102553200010201000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 4, false, "0363747800025531010300000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 4, false, "000f02553102553200010300000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 4, true, "0363747800025531010301000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 4, true, "000f02553102553200010301000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 5, false, "0363747800025531010400000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 5, false, "000f02553102553200010400000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 5, true, "0363747800025531010401000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 5, true, "000f02553102553200010401000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 6, false, "0363747800025531010500000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 6, false, "000f02553102553200010500000000000000f83f01000000000000e0bf000207"),
  ("SCORES", 6, true, "0363747800025531010501000000000000f83f01000000000000e0bf000207"),
  ("NEIGH", 6, true, "000f02553102553200010501000000000000f83f01000000000000e0bf000207"),
];

fn unhex(s: &str) -> Vec<u8> {
  (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn old_client_bytes_decode() {
  let kinds = [
    None,
    Some(NodeKind::User),
    Some(NodeKind::Beacon),
    Some(NodeKind::Comment),
    Some(NodeKind::Opinion),
    Some(NodeKind::PollVariant),
    Some(NodeKind::Poll),
  ];
  for (shape, k, hide, hex) in OLD_CLIENT_BYTES {
    let (req, used): (Request, usize) = decode_from_slice(&unhex(hex), standard()).unwrap();
    assert_eq!(used, hex.len() / 2);
    match (&req.data, *shape) {
      (ReqData::ReadScores(d), "SCORES") => {
        assert_eq!(req.subgraph, "ctx");
        assert_eq!(d.ego, "U1");
        assert_eq!(d.score_options.node_kind, kinds[*k]);
        assert_eq!(d.score_options.hide_personal, *hide);
        assert_eq!((d.score_options.index, d.score_options.count), (2, 7));
      },
      (ReqData::ReadNeighbors(d), "NEIGH") => {
        assert_eq!((d.ego.as_str(), d.focus.as_str()), ("U1", "U2"));
        assert_eq!(d.kind, kinds[*k]);
        assert_eq!(d.hide_personal, *hide);
        assert_eq!((d.index, d.count), (2, 7));
      },
      other => panic!("unexpected decode: {:?}", other),
    }
  }
}

// ---------------------------------------------------------------------------
// Isolated contexts
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writes_reach_only_their_context() {
  let proc = MultiGraphProcessor::new(settings());
  write(&proc, "", "U1", "U2", 1.0).await;
  write(&proc, "X", "U1", "U3", 2.0).await;
  write(&proc, "X", "U1", "U2", -0.5).await; // a wall, in a named context
  write(&proc, "Y", "B1", "U1", 1.0).await;
  request(&proc, "Z", ReqData::WriteCreateContext).await;
  sync(&proc).await;
  assert_eq!(edges(&proc, "").await, vec![("U1".into(), "U2".into(), 1.0)]);
  assert_eq!(
    edges(&proc, "X").await,
    vec![("U1".into(), "U2".into(), -0.5), ("U1".into(), "U3".into(), 2.0)]
  );
  assert_eq!(edges(&proc, "Y").await, vec![("B1".into(), "U1".into(), 1.0)]);
  assert!(edges(&proc, "Z").await.is_empty());
}

/// A bulk load gives every context exactly its own edges, the same as writing them one by one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bulk_load_equals_sequential_writes_per_context() {
  let input: Vec<(&str, &str, f64, &str)> = vec![
    ("U1", "U2", 1.0, ""),
    ("U2", "U3", 2.0, ""),
    ("U1", "U2", 3.0, "X"),
    ("B1", "U1", 1.0, "X"),
    ("U3", "U1", -0.4, "X"),
    ("U1", "U2", 0.5, ""),
    ("C1", "U2", 1.0, "Y"),
  ];
  let bulk = MultiGraphProcessor::new(settings());
  let r = request(
    &bulk,
    "",
    ReqData::WriteBulkEdges(OpWriteBulkEdges {
      edges: input
        .iter()
        .map(|(s, d, w, c)| BulkEdge {
          src:       (*s).into(),
          dst:       (*d).into(),
          amount:    *w,
          magnitude: 0,
          context:   (*c).into(),
        })
        .collect(),
    }),
  )
  .await;
  assert!(matches!(r, Response::Ok));
  let seq = MultiGraphProcessor::new(settings());
  for (s, d, w, c) in &input {
    write(&seq, c, s, d, *w).await;
  }
  sync(&bulk).await;
  sync(&seq).await;
  for ctx in ["", "X", "Y"] {
    assert_eq!(edges(&bulk, ctx).await, edges(&seq, ctx).await, "context {ctx:?}");
  }
  assert_eq!(edges(&bulk, "").await.len(), 2);
  assert_eq!(edges(&bulk, "X").await.len(), 3);
}
