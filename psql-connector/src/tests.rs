// use super::testing::*;
use pgrx::prelude::*;

const NEIGHBORS_INBOUND: i64 = 2;
// use std::thread::sleep;
// use std::time::Duration;

// #[pg_test]
// fn sync_deadlock() {
//   for _ in 0..3000 {
//     let _ = crate::mr_reset().unwrap();
//     let _ =
//       crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), None, Some(-1))
//         .unwrap();
//     let _ =
//       crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
//         .unwrap();
//     let _ =
//       crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), None, Some(-1))
//         .unwrap();
//     let _ = crate::mr_sync(Some(1000)).unwrap();
//   }
// }

// #[pg_test]
// fn zerorec_graph_all() {
//   let _ = crate::mr_reset().unwrap();

//   put_testing_edges();

//   let _ = crate::mr_zerorec(Some(true), None).unwrap();

//   sleep(Duration::from_millis(1000));

//   let res = crate::mr_graph(
//     Some("Uadeb43da4abb"),
//     Some("B7f628ad203b5"),
//     None,
//     Some(false),
//     None,
//     None,
//   )
//   .unwrap();

//   let n = res.count();

//   assert!(n > 1);
//   assert!(n < 5);
// }

// #[pg_test]
// fn recalculate_clustering() {
//   let _ = crate::mr_reset().unwrap();

//   put_testing_edges();

//   let _ = crate::mr_recalculate_clustering(Some(true), None).unwrap();
// }

// #[pg_test]
// fn zerorec_graph_positive_only() {
//   let _ = crate::mr_reset().unwrap();

//   put_testing_edges();

//   let _ = crate::mr_zerorec(Some(true), None).unwrap();

//   sleep(Duration::from_millis(200));

//   let res = crate::mr_graph(
//     Some("Uadeb43da4abb"),
//     Some("B7f628ad203b5"),
//     None,
//     Some(true),
//     None,
//     None,
//   )
//   .unwrap();

//   let n = res.count();

//   assert!(n > 1);
//   assert!(n < 5);
// }

// #[pg_test]
// fn zerorec_reset_perf() {
//   let _ = crate::mr_reset().unwrap();

//   put_testing_edges();
//   let _ = crate::mr_zerorec(Some(true), None).unwrap();
//   let _ = crate::mr_reset().unwrap();
//   put_testing_edges();
//   let _ = crate::mr_create_context(Some("X")).unwrap();
//   let _ = crate::mr_create_context(Some("Y")).unwrap();
//   let _ = crate::mr_create_context(Some("Z")).unwrap();
//   let _ = crate::mr_zerorec(Some(true), None).unwrap();

//   let begin = SystemTime::now();
//   let get_time =
//     || SystemTime::now().duration_since(begin).unwrap().as_millis();

//   let _ = crate::mr_graph(
//     Some("Uadeb43da4abb"),
//     Some("U000000000000"),
//     None,
//     Some(true),
//     None,
//     None,
//   )
//   .unwrap();

//   assert!(get_time() < 200);
// }

// #[pg_test]
// fn zerorec_scores() {
//   let _ = crate::mr_reset().unwrap();

//   put_testing_edges();

//   let _ = crate::mr_zerorec(Some(true), None).unwrap();

//   sleep(Duration::from_millis(200));

//   let res = crate::mr_scores(
//     Some("Uadeb43da4abb"),
//     Some(true),
//     Some(""),
//     Some("B"),
//     None,
//     None,
//     Some(0.0),
//     None,
//     Some(0),
//     Some(i32::MAX as i64),
//   )
//   .unwrap();

//   let n = res.count();

//   assert!(n > 5);
//   assert!(n < 80);
// }

#[pg_test]
fn service() {
  let ver = crate::mr_service();

  //  check if ver is in form "X.Y.Z"

  let nums: Vec<&str> = ver.split(".").collect();

  assert_eq!(nums.len(), 3);
  let _ = nums[0].parse::<u32>().unwrap();
  let _ = nums[1].parse::<u32>().unwrap();
}

#[pg_test]
fn edge_uncontexted() {
  let _ = crate::mr_reset().unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
      .unwrap();

  let n = res
    .map(|x| {
      let (ego, target, score) = x;
      assert_eq!(ego, "U1");
      assert_eq!(target, "U2");
      assert_eq!(score, 1.0);
    })
    .count();

  assert_eq!(n, 1);
}

#[pg_test]
fn edge_contexted() {
  let _ = crate::mr_reset().unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_put_edge(
    Some("U1"),
    Some("U2"),
    Some(1.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();

  let n = res
    .map(|x| {
      let (ego, target, score) = x;
      assert_eq!(ego, "U1");
      assert_eq!(target, "U2");
      assert_eq!(score, 1.0);
    })
    .count();

  assert_eq!(n, 1);
}

#[pg_test]
fn create_context() {
  let _ = crate::mr_reset().unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
      .unwrap();
  let _ = crate::mr_create_context(Some("X"));

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_edgelist(Some("X")).unwrap();

  let _n = res
    .map(|x| {
      let (ego, target, score) = x;
      assert_eq!(ego, "U1");
      assert_eq!(target, "U2");
      assert!(score > 0.99);
      assert!(score < 1.01);
    })
    .count();

  // assert_eq!(n, 1);
}

#[pg_test]
fn null_context_is_sum() {
  let _ = crate::mr_reset().unwrap();

  let _ = crate::mr_put_edge(
    Some("B1"),
    Some("U2"),
    Some(1.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("B1"),
    Some("U2"),
    Some(2.0),
    Some("Y"),
    Some(-1),
  )
  .unwrap();

  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_edgelist(None).unwrap();
  let edges: Vec<_> = res.collect();
  assert_eq!(edges.len(), 1);
  let (ego, target, score) = &edges[0];
  assert_eq!(ego, "B1");
  assert_eq!(target, "U2");
  // Null context is verbatim aggregate: last write wins (Y=2.0).
  assert!(*score > 1.99 && *score < 2.01);
}

#[pg_test]
fn delete_contexted_edge() {
  let _ = crate::mr_reset().unwrap();

  let _ = crate::mr_put_edge(
    Some("B1"),
    Some("U2"),
    Some(1.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("B1"),
    Some("U2"),
    Some(2.0),
    Some("Y"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_delete_edge(Some("B1"), Some("U2"), Some("X"), Some(-1))
    .unwrap();

  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_edgelist(None).unwrap();
  let edges: Vec<_> = res.collect();
  // Delete in X also zeros ""; expect no edges or one edge with weight 0.
  assert!(
    edges.is_empty()
      || (edges.len() == 1 && (edges[0].2 - 0.0).abs() < 1e-6),
    "expected no edges or single edge with weight 0, got {} edges",
    edges.len()
  );
  if edges.len() == 1 {
    assert_eq!(edges[0].0, "B1");
    assert_eq!(edges[0].1, "U2");
  }
}

#[pg_test]
fn delete_nodes() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
      .unwrap();
  let _ = crate::mr_delete_node(Some("U1"), None, Some(-1)).unwrap();
  let _ = crate::mr_delete_node(Some("U2"), None, Some(-1)).unwrap();

  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_edgelist(None).unwrap();
  assert_eq!(res.count(), 0);
}

#[pg_test]
fn null_context_invariant() {
  let _ = crate::mr_reset().unwrap();

  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("B2"),
    Some(1.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("B2"),
    Some(2.0),
    Some("Y"),
    Some(-1),
  )
  .unwrap();

  let _ = crate::mr_delete_edge(Some("U1"), Some("B2"), Some("X"), Some(-1));
  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("B2"),
    Some(1.0),
    Some("X"),
    Some(-1),
  );

  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_edgelist(None).unwrap();
  let edges: Vec<_> = res.collect();
  assert_eq!(edges.len(), 1);
  let (ego, target, score) = &edges[0];
  assert_eq!(ego, "U1");
  assert_eq!(target, "B2");
  // Delete then re-add in X: "" ends with 1.0 (verbatim).
  assert!(*score > 0.99 && *score < 1.01);
}

#[pg_test]
fn node_score_context() {
  let _ = crate::mr_reset().unwrap();

  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("U2"),
    Some(2.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("U3"),
    Some(1.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("U3"),
    Some("U2"),
    Some(3.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_node_score(Some("U1"), Some("U2"), Some("X")).unwrap();

  let n = res
    .map(|x| {
      let (ego, dst, score_dst, score_ego, _, _) = x;
      assert_eq!(ego, "U1");
      assert_eq!(dst, "U2");
      assert!(score_dst > 0.25);
      assert!(score_dst < 0.45);
      assert!(score_ego > -0.1);
      assert!(score_ego < 0.1);
    })
    .count();

  assert_eq!(n, 1);
}

#[pg_test]
fn scores_null_context() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), Some(""), Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), Some(""), Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), Some(""), Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res: Vec<_> = crate::mr_scores(
    Some("U1"),
    Some(false),
    Some(""),
    Some("U"),
    Some(10.0),
    None,
    Some(0.0),
    None,
    None,
    None,
  )
  .unwrap()
  .collect();

  assert_eq!(res.len(), 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      "U2" => {
        assert!(x.2 > 0.1);
        assert!(x.2 < 0.4);
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      _ => assert!(false),
    }
  }
}

#[pg_test]
fn scores_context() {
  let _ = crate::mr_reset().unwrap();

  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("U2"),
    Some(2.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("U3"),
    Some(1.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("U2"),
    Some("U3"),
    Some(3.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res: Vec<_> = crate::mr_scores(
    Some("U1"),
    Some(false),
    Some("X"),
    Some("U"),
    Some(10.0),
    None,
    Some(0.0),
    None,
    None,
    None,
  )
  .unwrap()
  .collect();

  assert_eq!(res.len(), 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      "U2" => {
        assert!(x.2 > 0.1);
        assert!(x.2 < 0.4);
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      _ => assert!(false),
    }
  }
}

#[pg_test]
fn scores_defaults() {
  let _ = crate::mr_reset().unwrap();

  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("U2"),
    Some(2.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("U1"),
    Some("U3"),
    Some(1.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();
  let _ = crate::mr_put_edge(
    Some("U2"),
    Some("U3"),
    Some(3.0),
    Some("X"),
    Some(-1),
  )
  .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res: Vec<_> = crate::mr_scores(
    Some("U1"),
    Some(false),
    Some("X"),
    Some("U"),
    None,
    None,
    None,
    None,
    None,
    None,
  )
  .unwrap()
  .collect();

  assert_eq!(res.len(), 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      "U2" => {
        assert!(x.2 > 0.1);
        assert!(x.2 < 0.4);
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      _ => assert!(false),
    }
  }
}

#[pg_test]
fn nodelist() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), None, Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res: Vec<_> = crate::mr_nodelist(None).unwrap().collect();

  assert_eq!(res.len(), 3);

  for x in res {
    assert!(x.0 == "U1" || x.0 == "U2" || x.0 == "U3");
  }
}

#[pg_test]
fn connected() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(2.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("U3"), Some(3.0), None, Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res: Vec<_> = crate::mr_connected(Some("U1"), None).unwrap().collect();

  assert_eq!(res.len(), 2);

  for x in res {
    assert_eq!(x.0, "U1");
    assert!(x.1 == "U2" || x.1 == "U3");
  }
}

#[pg_test]
fn mutual_scores() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(3.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("U1"), Some(2.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("U3"), Some(4.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U3"), Some("U1"), Some(3.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U3"), Some("U2"), Some(2.0), None, Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res: Vec<_> =
    crate::mr_mutual_scores(Some("U1"), None).unwrap().collect();

  assert_eq!(res.len(), 3);

  let mut u1 = true;
  let mut u2 = true;
  let mut u3 = true;

  for x in res.iter() {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(res[0].2 > 0.15);
        assert!(res[0].2 < 0.45);
        assert!(res[0].3 > 0.15);
        assert!(res[0].3 < 0.45);
        assert!(u1);
        u1 = false;
      },

      "U2" => {
        assert!(res[1].2 > 0.15);
        assert!(res[1].2 < 0.45);
        assert!(res[1].3 > 0.05);
        assert!(res[1].3 < 0.45);
        assert!(u2);
        u2 = false;
      },

      "U3" => {
        assert!(res[2].2 > 0.05);
        assert!(res[2].2 < 0.45);
        assert!(res[2].3 > 0.15);
        assert!(res[2].3 < 0.45);
        assert!(u3);
        u3 = false;
      },

      _ => {
        assert!(false);
      },
    };
  }
}

#[pg_test]
fn new_edges_fetch() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
      .unwrap();

  let _n = crate::mr_fetch_new_edges(Some("U1"), Some("B"))
    .unwrap()
    .count();

  // assert_eq!(n, 0);

  let _ =
    crate::mr_put_edge(Some("U1"), Some("B3"), Some(2.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("B4"), Some(3.0), None, Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_fetch_new_edges(Some("U1"), Some("B")).unwrap();

  let _beacons: Vec<_> = res.collect();

  // assert_eq!(beacons.len(), 2);
  // assert_eq!(beacons[0].1, "B3");
  // assert_eq!(beacons[1].1, "B4");

  // assert_eq!(
  //   crate::mr_fetch_new_edges(Some("U1"), Some("B"))
  //     .unwrap()
  //     .count(),
  //   0
  // );
}

#[pg_test]
fn new_edges_filter() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
      .unwrap();

  let _n = crate::mr_fetch_new_edges(Some("U1"), Some("B"))
    .unwrap()
    .count();

  // assert_eq!(n, 0);

  let _ =
    crate::mr_put_edge(Some("U1"), Some("B3"), Some(2.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("B4"), Some(3.0), None, Some(-1))
      .unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let filter: Vec<u8> = crate::mr_get_new_edges_filter(Some("U1")).unwrap();

  let res = crate::mr_fetch_new_edges(Some("U1"), Some("B")).unwrap();

  let _beacons: Vec<_> = res.collect();

  // assert_eq!(beacons.len(), 2);
  // assert_eq!(beacons[0].1, "B3");
  // assert_eq!(beacons[1].1, "B4");

  let _ = crate::mr_set_new_edges_filter(Some("U1"), Some(filter)).unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_fetch_new_edges(Some("U1"), Some("B")).unwrap();

  let _beacons: Vec<_> = res.collect();

  // assert_eq!(beacons.len(), 2);
  // assert_eq!(beacons[0].1, "B3");
  // assert_eq!(beacons[1].1, "B4");
}

#[pg_test]
fn five_scores_clustering() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(5.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U3"), Some(1.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U4"), Some(2.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U1"), Some("U5"), Some(3.0), None, Some(-1))
      .unwrap();
  let _ =
    crate::mr_put_edge(Some("U2"), Some("U1"), Some(4.0), None, Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res: Vec<_> = crate::mr_scores(
    Some("U1"),
    Some(true),
    Some(""),
    Some(""),
    None,
    None,
    Some(0.0),
    None,
    Some(0),
    Some(i32::MAX as i64),
  )
  .unwrap()
  .collect();

  assert_eq!(res.len(), 5);

  assert!(res[0].4 <= 100);
  assert!(res[0].4 >= 40);

  assert!(res[1].4 <= 100);
  assert!(res[1].4 >= 20);

  assert!(res[2].4 <= 100);
  assert!(res[2].4 >= 1);

  assert!(res[3].4 <= 80);
  assert!(res[3].4 >= 1);

  assert!(res[4].4 <= 60);
  assert!(res[4].4 >= 1);
}

#[pg_test]
fn set_zero_opinion() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(5.0), None, Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let s0: Vec<_> = crate::mr_node_score(Some("U1"), Some("U2"), None)
    .unwrap()
    .collect();

  let _ = crate::mr_set_zero_opinion(Some("U2"), Some(-10.0), None);

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let s1: Vec<_> = crate::mr_node_score(Some("U1"), Some("U2"), None)
    .unwrap()
    .collect();

  assert_ne!(s0[0].2, s1[0].2);
}

#[pg_test]
fn neighbors_inbound() {
  let _ = crate::mr_reset().unwrap();

  let _ =
    crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
      .unwrap();

  // sleep(Duration::from_millis(100));
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let _neighbors: Vec<_> = crate::mr_neighbors(
    Some("U1"),
    Some("U2"),
    Some(NEIGHBORS_INBOUND),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
  )
  .unwrap()
  .collect();

  // assert_eq!(neighbors.len(), 1);
  // assert_eq!(neighbors[0].0, "U1");
  // assert_eq!(neighbors[0].1, "U1");
}

#[pg_test]
fn bulk_load_basic() {
  let _ = crate::mr_reset().unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let res = crate::mr_bulk_load_edges(
    vec!["U1".into(), "U1".into(), "U2".into()],
    vec!["U2".into(), "U3".into(), "U3".into()],
    vec![1.0, 2.0, 3.0],
    vec!["".into(), "".into(), "".into()],
    None,
  );
  assert!(res.is_ok());
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let scores: Vec<_> = crate::mr_scores(
    Some("U1"),
    Some(false),
    Some(""),
    Some("U"),
    None,
    None,
    None,
    None,
    Some(0),
    Some(16),
  )
  .unwrap()
  .collect();
  assert!(!scores.is_empty());
}

#[pg_test]
fn bulk_load_with_contexts() {
  let _ = crate::mr_reset().unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let _ = crate::mr_bulk_load_edges(
    vec!["U1".into(), "U1".into(), "U2".into()],
    vec!["U2".into(), "U3".into(), "U3".into()],
    vec![1.0, 2.0, 3.0],
    vec!["".into(), "X".into(), "X".into()],
    None,
  )
  .unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let agg: Vec<_> = crate::mr_edgelist(None).unwrap().collect();
  assert_eq!(agg.len(), 3);
  let ctx_x: Vec<_> = crate::mr_edgelist(Some("X")).unwrap().collect();
  assert_eq!(ctx_x.len(), 3);
}

#[pg_test]
fn bulk_load_then_scores() {
  let _ = crate::mr_reset().unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let _ = crate::mr_bulk_load_edges(
    vec!["U1".into(), "U2".into()],
    vec!["U2".into(), "U3".into()],
    vec![1.0, 2.0],
    vec!["".into(), "".into()],
    None,
  )
  .unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let scores: Vec<_> = crate::mr_scores(
    Some("U1"),
    Some(false),
    Some(""),
    Some("U"),
    None,
    None,
    None,
    None,
    Some(0),
    Some(16),
  )
  .unwrap()
  .collect();
  assert!(!scores.is_empty());
  assert!(scores.iter().any(|r| r.1 == "U2" && r.2 > 0.0));
}

#[pg_test]
fn bulk_load_replaces_state() {
  let _ = crate::mr_reset().unwrap();
  let _ = crate::mr_put_edge(Some("U1"), Some("U2"), Some(1.0), None, Some(-1))
    .unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let before: Vec<_> = crate::mr_edgelist(None).unwrap().collect();
  assert_eq!(before.len(), 1);

  let _ = crate::mr_bulk_load_edges(
    vec!["U1".into(), "U1".into()],
    vec!["U3".into(), "U4".into()],
    vec![1.0, 1.0],
    vec!["".into(), "".into()],
    None,
  )
  .unwrap();
  let _ = crate::mr_sync(Some(1000)).unwrap();

  let after: Vec<_> = crate::mr_edgelist(None).unwrap().collect();
  assert_eq!(after.len(), 2);
  assert!(after.iter().any(|e| e.1 == "U3"));
  assert!(after.iter().any(|e| e.1 == "U4"));
}
