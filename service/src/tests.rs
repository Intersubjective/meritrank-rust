use crate::aug_multi_graph::*;
use crate::nodes::*;
use crate::protocol::*;
use crate::test_data::*;
use std::time::SystemTime;

fn default_graph() -> AugMultiGraph {
  AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 50,
    zero_opinion_num_walks: 100,
    zero_opinion_factor: 0.0,
    ..AugMultiGraphSettings::default()
  })
}

fn default_graph_zero() -> AugMultiGraph {
  AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 50,
    zero_opinion_num_walks: 50,
    ..AugMultiGraphSettings::default()
  })
}

#[test]
fn encoding_serde() {
  let in_command: String = "foo".into();
  let in_context: &str = "bar";
  let in_arg1: &str = "baz";
  let in_arg2: &str = "bus";

  let payload = rmp_serde::to_vec(&(
    in_command.clone(),
    in_context,
    rmp_serde::to_vec(&(in_arg1, in_arg2)).unwrap(),
  ))
  .unwrap();

  let out_command: &str;
  let out_context: String;
  let _out_args: Vec<u8>;

  (out_command, out_context, _out_args) =
    rmp_serde::from_slice(payload.as_slice()).unwrap();

  assert_eq!(out_command, in_command);
  assert_eq!(out_context, in_context);
}

#[test]
fn encoding_response() {
  let foo = ("foo".to_string(), 1, 2, 3);
  let payload = encode_response(&foo).unwrap();

  let bar: (String, i32, i32, i32) = decode_response(&payload).unwrap();

  assert_eq!(foo.0, bar.0);
  assert_eq!(foo.1, bar.1);
  assert_eq!(foo.2, bar.2);
  assert_eq!(foo.3, bar.3);
}

#[test]
fn no_assert() {
  assert_eq!(meritrank_core::constants::ASSERT, false);
}

#[test]
fn recalculate_zero_graph_all() {
  let mut graph = default_graph();

  put_testing_edges_0(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "B7f628ad203b5", false, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);

  assert!(n > 1);
  assert!(n < 5);
}

#[test]
fn graph_sort_order() {
  let mut graph = default_graph();

  put_testing_edges_0(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "Bfae1726e4e87", false, 0, 10000);

  assert!(res.len() > 1);

  for n in 1..res.len() {
    assert!(res[n - 1].2.abs() >= res[n].2.abs());
  }
}

#[test]
fn recalculate_zero_graph_duplicates() {
  let mut graph = default_graph();

  put_testing_edges_0(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Bb5f87c1621d5", "Ub01f4ad1b03f", false, 0, 10000);

  assert!(res.len() > 1);

  for (i, x) in res.iter().enumerate() {
    for (j, y) in res.iter().take(i).enumerate() {
      if x.0 == y.0 && x.1 == y.1 {
        println!("Duplicate: [{}, {}] {} -> {}", i, j, x.0, x.1);
      }
      assert!(x.0 != y.0 || x.1 != y.1);
    }
  }
}

#[test]
fn recalculate_zero_graph_positive_only() {
  let mut graph = default_graph();

  put_testing_edges_0(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "B7f628ad203b5", true, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);
  assert!(n > 1);
  assert!(n < 5);
}

#[test]
fn recalculate_zero_graph_focus_beacon() {
  let mut graph = default_graph();

  put_testing_edges_0(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "U95f3426b8e5d", "B79efabc4d8bf", true, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);

  for edge in res {
    println!("{} -> {}", edge.0, edge.1);
  }

  assert!(n >= 2);
  assert!(n < 80);
}

#[test]
fn recalculate_zero_reset_perf() {
  let mut graph = default_graph();

  put_testing_edges_0(&mut graph);
  graph.write_recalculate_zero();
  graph.reset();
  put_testing_edges_0(&mut graph);
  graph.write_create_context("X");
  graph.write_create_context("Y");
  graph.write_create_context("Z");
  graph.write_recalculate_zero();

  let begin = SystemTime::now();
  let get_time =
    || SystemTime::now().duration_since(begin).unwrap().as_millis();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "B0e230e9108dd", true, 0, 10000);

  assert!(res.len() > 1);

  assert!(get_time() < 500);
}

#[test]
fn recalculate_zero_scores() {
  let mut graph = default_graph_zero();

  put_testing_edges_0(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> = graph.read_scores(
    "",
    "Uadeb43da4abb",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  println!("Got {} edges", n);
  assert!(n > 5);
  assert!(n < 80);
}

#[test]
fn scores_sort_order() {
  let mut graph = default_graph();

  put_testing_edges_0(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> = graph.read_scores(
    "",
    "Uadeb43da4abb",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  assert!(res.len() > 1);

  for n in 1..res.len() {
    assert!(res[n - 1].2.abs() >= res[n].2.abs());
  }
}

#[test]
fn scores_without_recalculate() {
  let mut graph = default_graph();

  put_testing_edges_1(&mut graph);

  graph.write_put_edge("", "U1", "U0", 1.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert_eq!(n, 2);
}

#[test]
fn scores_with_recalculate() {
  let mut graph = default_graph_zero();

  put_testing_edges_1(&mut graph);

  graph.write_recalculate_zero();

  graph.write_put_edge("", "U1", "U0", 1.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert!(n > 2);
}

#[test]
fn new_user_without_recalculate() {
  let mut graph = default_graph();

  put_testing_edges_1(&mut graph);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert_eq!(n, 1); // Only self-score
}

#[test]
fn new_user_with_recalculate() {
  let mut graph = default_graph();

  put_testing_edges_1(&mut graph);

  graph.write_recalculate_zero();
  graph.write_put_edge("", "U1", "U7a8d8324441d", -1000.0, -1);

  //  read_scores should return zero opinion data even if the node doesn't exist

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert!(n > 2);
}

#[test]
fn user_with_recalculate_negative_score() {
  let mut graph = default_graph_zero();

  graph.write_put_edge("", "U2", "U3", 1.0, -1);

  graph.write_recalculate_zero();
  graph.write_put_edge("", "U1", "U3", -1000.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  //  The negative opinion of the user should have affected the score of U3
  assert!(n == 3);
}

#[test]
fn new_friend_smol() {
  let mut graph = default_graph();

  graph.write_put_edge("", "Ue925856b9cd9", "Ucc76e1b73be0", 1.0, -1);

  graph.write_recalculate_zero();

  let (_, _, s0, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  graph.write_put_edge("", "Ue925856b9cd9", "U6d2f25cc4264", 1.0, -1);

  let (_, _, s1, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  assert_ne!(s0, s1);
}

#[test]
fn new_friend_big() {
  let mut graph = default_graph();

  put_testing_edges_1(&mut graph);

  graph.write_recalculate_zero();

  let (_, _, s0, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  graph.write_put_edge("", "Ue925856b9cd9", "U6d2f25cc4264", 1.0, -1);

  let (_, _, s1, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  assert_ne!(s0, s1);
}

#[test]
fn edge_uncontexted() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 1.5, -1);

  let edges: Vec<_> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("U1".to_string(), "U2".to_string(), 1.5)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn edge_contexted() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 1.5, -1);

  let edges: Vec<_> = graph.read_edges("X");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("U1".to_string(), "U2".to_string(), 1.5)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_is_sum() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "B1", "U2", 1.0, -1);
  graph.write_put_edge("Y", "B1", "U2", 2.0, -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("B1".to_string(), "U2".to_string(), 3.0)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_contains_all_users() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 1.0, -1);
  graph.write_put_edge("Y", "U1", "U3", 2.0, -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.0),
    ("U1".to_string(), "U3".to_string(), 2.0),
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn user_edges_dup() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 1.0, -1);
  graph.write_put_edge("X", "U1", "U3", 2.0, -1);
  graph.write_create_context("Y");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("Y");

  let edges_expected: Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.0),
    ("U1".to_string(), "U3".to_string(), 2.0),
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn non_user_edges_no_dup() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "C2", 1.0, -1);
  graph.write_put_edge("X", "U1", "C3", 2.0, -1);
  graph.write_create_context("Y");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("Y");

  assert_eq!(edges.len(), 0);
}

#[test]
fn delete_nodes() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_delete_node("", "U1", -1);
  graph.write_delete_node("", "U2", -1);

  assert_eq!(graph.read_edges("").len(), 0);
}

#[test]
fn delete_contexted_edge() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "B1", "U2", 1.0, -1);
  graph.write_put_edge("Y", "B1", "U2", 2.0, -1);
  graph.write_delete_edge("X", "B1", "U2", -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("B1".to_string(), "U2".to_string(), 2.0)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_invariant() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "B1", "B2", 1.0, -1);
  graph.write_put_edge("Y", "B1", "B2", 2.0, -1);
  graph.write_delete_edge("X", "B1", "B2", -1);
  graph.write_put_edge("X", "B1", "B2", 1.0, -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("B1".to_string(), "B2".to_string(), 3.0)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn scores_uncontexted() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert_eq!(res.len(), 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      "U2" => {
        assert!(x.2 > 0.18);
        assert!(x.2 < 0.5);
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      _ => assert!(false),
    }
  }
}

#[test]
fn scores_reversed() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);
  graph.write_put_edge("", "U3", "U1", -5.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert!(res.len() >= 2);
  assert!(res.len() <= 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.0);
        assert!(x.2 < 0.4);
        assert!(x.3 > 0.0);
        assert!(x.3 < 0.4);
      },

      "U2" => {
        assert!(x.2 > -0.1);
        assert!(x.2 < 0.3);
        assert!(x.3 > -0.3);
        assert!(x.3 < 0.1);
      },

      "U3" => {
        assert!(x.2 > -0.1);
        assert!(x.2 < 0.3);
        assert!(x.3 > -0.6);
        assert!(x.3 < 0.0);
      },

      _ => assert!(false),
    }
  }
}

#[test]
fn scores_contexted() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_scores(
    "X",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

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

#[test]
fn scores_unknown_context() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "B1", "B2", 2.0, -1);
  graph.write_put_edge("X", "B1", "B3", 1.0, -1);
  graph.write_put_edge("X", "B2", "B3", 3.0, -1);

  let res: Vec<_> = graph.read_scores(
    "Y",
    "B1",
    "B",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert_eq!(res.len(), 0);
}

#[test]
fn scores_reset_smoke() {
  let mut graph_read = default_graph();
  let mut graph_write = default_graph();

  graph_write.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph_write.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph_write.write_put_edge("X", "U2", "U3", 3.0, -1);

  graph_read.copy_from(&graph_write);
  let res: Vec<_> = graph_read.read_scores(
    "X", "U1", "U", false, 10.0, false, 0.0, false, 0, 2147483647,
  );

  assert_eq!(res.len(), 3);

  graph_write.reset();

  graph_write.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph_write.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph_write.write_put_edge("X", "U2", "U3", 3.0, -1);

  graph_read.copy_from(&graph_write);
  let res: Vec<_> = graph_read.read_scores(
    "X",
    "U1",
    "U",
    false,
    2147483647.0,
    false,
    -2147483648.0,
    false,
    0,
    2147483647,
  );

  assert_eq!(res.len(), 3);
}

#[test]
fn scores_self() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "B1", "B2", 2.0, -1);
  graph.write_put_edge("X", "B1", "B3", 1.0, -1);
  graph.write_put_edge("X", "B2", "U1", 3.0, -1);
  graph.write_create_context("Y");

  let res: Vec<_> = graph.read_scores(
    "Y",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U1");
  assert!(res[0].2 > 0.999);
  assert!(res[0].2 < 1.001);
}

#[test]
fn node_list_uncontexted() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U3", "U2", 3.0, -1);

  let res: Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _ => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_list_contexted() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U3", "U2", 3.0, -1);

  let res: Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _ => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_list_mixed() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("Y", "U3", "U2", 3.0, -1);

  let res: Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _ => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_score_uncontexted() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U3", "U2", 3.0, -1);

  let res: Vec<_> = graph.read_node_score("", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.3);
  assert!(res[0].2 < 0.45);
}

#[test]
fn node_score_reversed() {
  let mut graph = default_graph_zero();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U3", "U2", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);

  let res: Vec<_> = graph.read_node_score("", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.2);
  assert!(res[0].2 < 0.4);
  assert!(res[0].3 > 0.2);
  assert!(res[0].3 < 0.4);
}

#[test]
fn node_score_contexted() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U3", "U2", 3.0, -1);

  let res: Vec<_> = graph.read_node_score("X", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.3);
  assert!(res[0].2 < 0.45);
}

#[test]
fn mutual_scores_uncontexted() {
  let mut graph = default_graph_zero();

  graph.write_put_edge("", "U1", "U2", 3.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U1", 2.0, -1);
  graph.write_put_edge("", "U2", "U3", 4.0, -1);
  graph.write_put_edge("", "U3", "U1", 3.0, -1);
  graph.write_put_edge("", "U3", "U2", 2.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("", "U1");

  println!("{:?}", res);

  assert_eq!(res.len(), 3);

  let mut u1 = true;
  let mut u2 = true;
  let mut u3 = true;

  for x in res.iter() {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.25);
        assert!(x.2 < 0.5);
        assert!(x.3 > 0.25);
        assert!(x.3 < 0.5);
        assert!(u1);
        u1 = false;
      },

      "U2" => {
        assert!(x.2 > 0.15);
        assert!(x.2 < 0.35);
        assert!(x.3 > 0.15);
        assert!(x.3 < 0.35);
        assert!(u2);
        u2 = false;
      },

      "U3" => {
        assert!(x.2 > 0.15);
        assert!(x.2 < 0.35);
        assert!(x.3 > 0.15);
        assert!(x.3 < 0.35);
        assert!(u3);
        u3 = false;
      },

      _ => {
        assert!(false);
      },
    };
  }
}

#[test]
fn mutual_scores_self() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 3.0, -1);
  graph.write_delete_edge("", "U1", "U2", -1);

  let res: Vec<_> = graph.read_mutual_scores("", "U1");

  println!("{:?}", res);

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U1");
  assert!(res[0].2 > 0.99);
  assert!(res[0].2 < 1.01);
  assert!(res[0].3 > 0.99);
  assert!(res[0].3 < 1.01);
}

#[test]
fn mutual_scores_contexted() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 3.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U2", "U1", 2.0, -1);
  graph.write_put_edge("X", "U2", "U3", 4.0, -1);
  graph.write_put_edge("X", "U3", "U1", 3.0, -1);
  graph.write_put_edge("X", "U3", "U2", 2.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("X", "U1");

  assert_eq!(res.len(), 3);

  let mut u1 = true;
  let mut u2 = true;
  let mut u3 = true;

  for x in res.iter() {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.3);
        assert!(x.2 < 0.5);
        assert!(x.3 > 0.3);
        assert!(x.3 < 0.5);
        assert!(u1);
        u1 = false;
      },

      "U2" => {
        assert!(x.2 > 0.25);
        assert!(x.2 < 0.4);
        assert!(x.3 > 0.2);
        assert!(x.3 < 0.35);
        assert!(u2);
        u2 = false;
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.35);
        assert!(x.3 > 0.2);
        assert!(x.3 < 0.35);
        assert!(u3);
        u3 = false;
      },

      _ => {
        assert!(false);
      },
    };
  }
}

#[test]
fn graph_uncontexted() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 2);

  let mut has_u1 = false;
  let mut has_u2 = false;

  for x in res {
    match x.0.as_str() {
      "U1" => {
        assert_eq!(x.1, "U2");
        assert!(x.2 > 0.65);
        assert!(x.2 < 0.67);
        has_u1 = true;
      },

      "U2" => {
        assert_eq!(x.1, "U3");
        assert!(x.2 > 0.99);
        assert!(x.2 < 1.01);
        has_u2 = true;
      },

      _ => panic!(),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
}

#[test]
fn graph_reversed() {
  let mut graph = default_graph_zero();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 3);

  for x in res {
    match x.0.as_str() {
      "U1" => {
        assert_eq!(x.1, "U2");
        assert!(x.2 > 0.6);
        assert!(x.2 < 0.7);
        assert!(x.3 > 0.05);
        assert!(x.3 < 0.4);
      },

      "U2" => {
        if x.1 == "U1" {
          assert!(x.2 > 0.5);
          assert!(x.2 < 0.6);
          assert!(x.3 > 0.2);
          assert!(x.3 < 0.5);
        }

        if x.1 == "U3" {
          assert!(x.2 > 0.39);
          assert!(x.2 < 0.49);
          assert!(x.3 > 0.16);
          assert!(x.3 < 0.4);
        }
      },

      _ => panic!(),
    }
  }
}

#[test]
fn graph_contexted() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_graph("X", "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 2);

  let mut has_u1 = false;
  let mut has_u2 = false;

  for x in res {
    match x.0.as_str() {
      "U1" => {
        assert_eq!(x.1, "U2");
        assert!(x.2 > 0.65);
        assert!(x.2 < 0.67);
        has_u1 = true;
      },

      "U2" => {
        assert_eq!(x.1, "U3");
        assert!(x.2 > 0.99);
        assert!(x.2 < 1.01);
        has_u2 = true;
      },

      _ => panic!(),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
}

#[test]
fn graph_empty() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);

  graph.write_delete_edge("", "U1", "U2", -1);
  graph.write_delete_edge("", "U1", "U3", -1);
  graph.write_delete_edge("", "U2", "U3", -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  for x in res.iter() {
    println!("{} -> {}: {}", x.0, x.1, x.2);
  }

  assert_eq!(res.len(), 0);
}

#[test]
fn graph_no_direct_connectivity() {
  // Test that graph will show the focus and its neighborhood
  // even if there is no true path from ego to focus.
  // (E.g. when focusing directly by id or through zero opinion)
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "B1", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "B2", 1.0, -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  for x in res.iter() {
    println!("{} -> {}: {}", x.0, x.1, x.2);
  }

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U2");
  assert_eq!(res[0].1, "U3");
}

#[test]
fn graph_force_connectivity() {
  // Test workaround option to force add edge to unconnected focus
  let mut graph = AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 100,
    zero_opinion_num_walks: 100,
    force_read_graph_conn: true,
    ..AugMultiGraphSettings::default()
  });

  graph.write_put_edge("", "U1", "B1", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "B2", 1.0, -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  for x in res.iter() {
    println!("{} -> {}: {}", x.0, x.1, x.2);
  }

  assert_eq!(res.len(), 2);
  // Make sure
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert_eq!(res[1].0, "U2");
  assert_eq!(res[1].1, "U3");
}

#[test]
fn new_edges_fetch() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);

  graph.write_put_edge("", "U1", "B3", 2.0, -1);
  graph.write_put_edge("", "U2", "B4", 3.0, -1);

  let beacons = graph.write_fetch_new_edges("U1", "B");

  assert_eq!(beacons.len(), 2);
  assert_eq!(beacons[0].0, "B3");
  assert_eq!(beacons[1].0, "B4");

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);
}

#[test]
fn new_edges_filter() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);

  graph.write_put_edge("", "U1", "B3", 2.0, -1);
  graph.write_put_edge("", "U2", "B4", 3.0, -1);

  let filter = graph.read_new_edges_filter("U1");
  assert_eq!(filter.len(), 32);

  let beacons = graph.write_fetch_new_edges("U1", "B");

  assert_eq!(beacons.len(), 2);
  assert_eq!(beacons[0].0, "B3");
  assert_eq!(beacons[1].0, "B4");

  graph.write_new_edges_filter("U1", &filter);
  let beacons = graph.write_fetch_new_edges("U1", "B");

  assert_eq!(beacons.len(), 2);
  assert_eq!(beacons[0].0, "B3");
  assert_eq!(beacons[1].0, "B4");
}

#[test]
fn copy_user_edges_into_context() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 1.0, -1);
  graph.write_put_edge("X", "U1", "C2", 2.0, -1);
  graph.write_create_context("Y");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("Y");

  assert_eq!(edges.len(), 1);
  assert_eq!(edges[0].0, "U1");
  assert_eq!(edges[0].1, "U2");
  assert!(edges[0].2 > 0.999);
  assert!(edges[0].2 < 1.001);
}

#[test]
fn context_already_exist() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "C2", 1.0, -1);
  graph.write_create_context("X");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("X");

  assert_eq!(edges.len(), 1);
  assert_eq!(edges[0].0, "U1");
  assert_eq!(edges[0].1, "C2");
  assert!(edges[0].2 > 0.999);
  assert!(edges[0].2 < 1.001);
}

#[test]
fn mutual_scores_cluster_single_score_uncontexted() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 10.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("", "U1");

  println!("{:?}", res);

  assert_eq!(res.len(), 2);
  assert!(res[0].4 == 100);
  assert!(res[1].4 == 1);
}

#[test]
fn mutual_scores_cluster_single_score_contexted() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", 10.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("X", "U1");

  println!("{:?}", res);

  assert_eq!(res.len(), 2);
  assert!(res[0].4 == 100);
  assert!(res[1].4 == 1);
}

#[test]
fn mutual_scores_clustering() {
  let mut graph = default_graph();

  graph.write_put_edge("X", "U1", "U2", -5.0, -1);
  graph.write_put_edge("X", "U1", "U3", -5.0, -1);
  graph.write_put_edge("X", "U1", "U4", 1.0, -1);
  graph.write_put_edge("X", "U1", "U5", 1.0, -1);
  graph.write_put_edge("X", "U1", "U6", 3.0, -1);
  graph.write_put_edge("X", "U1", "U7", 3.0, -1);
  graph.write_put_edge("X", "U1", "U8", 5.0, -1);
  graph.write_put_edge("X", "U1", "U9", 5.0, -1);
  graph.write_put_edge("X", "U1", "U10", 6.0, -1);
  graph.write_put_edge("X", "U1", "U11", 6.0, -1);

  graph.write_put_edge("X", "U2", "U1", 1.0, -1);
  graph.write_put_edge("X", "U3", "U1", 2.0, -1);
  graph.write_put_edge("X", "U4", "U1", 3.0, -1);
  graph.write_put_edge("X", "U5", "U1", 1.0, -1);
  graph.write_put_edge("X", "U6", "U1", 2.0, -1);
  graph.write_put_edge("X", "U7", "U1", 3.0, -1);
  graph.write_put_edge("X", "U8", "U1", 1.0, -1);
  graph.write_put_edge("X", "U9", "U1", 2.0, -1);
  graph.write_put_edge("X", "U10", "U1", 3.0, -1);
  graph.write_put_edge("X", "U11", "U1", 1.0, -1);

  graph.write_put_edge("X", "U2", "U3", 4.0, -1);
  graph.write_put_edge("X", "U3", "U4", 5.0, -1);
  graph.write_put_edge("X", "U4", "U5", 6.0, -1);
  graph.write_put_edge("X", "U5", "U6", 1.0, -1);
  graph.write_put_edge("X", "U6", "U7", 2.0, -1);
  graph.write_put_edge("X", "U7", "U8", 3.0, -1);
  graph.write_put_edge("X", "U8", "U9", 4.0, -1);
  graph.write_put_edge("X", "U9", "U10", 5.0, -1);
  graph.write_put_edge("X", "U10", "U11", 6.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("X", "U1");

  for (
    _src,
    _dst,
    _score_of_dst,
    _score_of_src,
    cluster_of_dst,
    cluster_of_src,
  ) in res.iter()
  {
    assert!(*cluster_of_dst >= 1);
    assert!(*cluster_of_dst <= 100);
    assert!(*cluster_of_src >= 1);
    assert!(*cluster_of_src <= 100);
  }
}

#[test]
fn five_user_scores_clustering() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 5.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U1", "U4", 2.0, -1);
  graph.write_put_edge("", "U1", "U5", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);

  //  We will get 5 score values including self-score.

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

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

#[test]
fn five_beacon_scores_clustering() {
  let mut graph = AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 500,
    ..AugMultiGraphSettings::default()
  });

  graph.write_put_edge("", "U1", "B2", 5.0, -1);
  graph.write_put_edge("", "U1", "B3", 1.0, -1);
  graph.write_put_edge("", "U1", "B4", 2.0, -1);
  graph.write_put_edge("", "U1", "B5", 3.0, -1);
  graph.write_put_edge("", "U1", "B6", 4.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

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

#[test]
fn three_scores_chain_clustering() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);
  graph.write_put_edge("", "U3", "U1", 4.0, -1);

  //  We will get 3 score values including self-score.

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 3);

  assert!(res[0].4 <= 100);
  assert!(res[0].4 >= 40);

  assert!(res[1].4 <= 80);
  assert!(res[1].4 >= 20);

  assert!(res[2].4 <= 60);
  assert!(res[2].4 >= 1);
}

#[test]
fn separate_clusters_without_users() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "B1", 3.0, -1);
  graph.write_put_edge("", "U1", "C1", 4.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 3);

  assert_eq!(res[0].4, 100);
  assert_eq!(res[1].4, 100);
  assert_eq!(res[2].4, 100);
}

#[test]
fn separate_clusters_self_score() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "B1", 3.0, -1);
  graph.write_put_edge("", "U1", "C1", 4.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 2);

  assert_eq!(res[0].4, 100);
  assert_eq!(res[1].4, 1);
}

#[test]
fn set_zero_opinion_uncontexted() {
  let mut graph = default_graph_zero();
  graph.write_put_edge("", "U1", "U2", -5.0, -1);
  let s0 = graph.read_node_score("", "U1", "U2")[0].2;
  graph.write_set_zero_opinion("", "U2", 10.0);
  let s1 = graph.read_node_score("", "U1", "U2")[0].2;

  println!("{}, {}", s0, s1);
  assert_ne!(s0, s1);
}

#[test]
fn set_zero_opinion_contexted() {
  let mut graph = default_graph_zero();
  graph.write_put_edge("X", "U1", "U2", -5.0, -1);
  let s0 = graph.read_node_score("X", "U1", "U2")[0].2;
  graph.write_set_zero_opinion("X", "U2", 10.0);
  let s1 = graph.read_node_score("X", "U1", "U2")[0].2;

  println!("{}, {}", s0, s1);
  assert_ne!(s0, s1);
}

#[test]
fn neighbors_all() {
  let mut graph = default_graph();
  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 2.0, -1);
  graph.write_put_edge("", "U3", "U1", 3.0, -1);

  let neighbors = graph.read_neighbors(
    "",
    "U1",
    "U2",
    NEIGHBORS_ALL,
    "",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    100,
  );

  assert_eq!(neighbors.len(), 2);
  assert_eq!(neighbors[0].1, "U1");
  assert_eq!(neighbors[1].1, "U3");
}

#[test]
fn neighbors_inbound() {
  let mut graph = default_graph();
  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 2.0, -1);
  graph.write_put_edge("", "U3", "U1", 3.0, -1);

  let neighbors = graph.read_neighbors(
    "",
    "U1",
    "U2",
    NEIGHBORS_INBOUND,
    "",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    100,
  );

  assert_eq!(neighbors.len(), 1);
  assert_eq!(neighbors[0].1, "U1");
}

#[test]
fn neighbors_outbound() {
  let mut graph = default_graph();
  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 2.0, -1);
  graph.write_put_edge("", "U3", "U1", 3.0, -1);

  let neighbors = graph.read_neighbors(
    "",
    "U1",
    "U2",
    NEIGHBORS_OUTBOUND,
    "",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    100,
  );

  assert_eq!(neighbors.len(), 1);
  assert_eq!(neighbors[0].1, "U3");
}

#[test]
fn neighbors_non_ego_score() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 1.0, -1);
  graph.write_put_edge("", "U1", "U4", 1.0, -1);
  graph.write_put_edge("", "U4", "U3", 1.0, -1);
  graph.write_put_edge("", "U3", "U4", 1.0, -1);

  // U2 should have a score from ego (U1), despite the focus being U3

  let neighbors = graph.read_neighbors(
    "",
    "U1",
    "U3",
    NEIGHBORS_INBOUND,
    "",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    100,
  );

  assert_eq!(neighbors.len(), 2);
  assert_eq!(neighbors[0].0, "U1");
  assert_eq!(neighbors[0].1, "U4");
  assert!(neighbors[1].2> 0.0, "Rating of U2 is calculated from the standpoint of ego, so it must be greater than 0");
  assert_eq!(neighbors.len(), 2);
  assert_eq!(neighbors[1].0, "U1");
  assert_eq!(neighbors[1].1, "U2");
  assert!(neighbors[1].2 > 0.0);
}

#[test]
fn neighbors_prioritize_ego_owned_objects() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 100.0, -1);
  graph.write_put_edge("", "U2", "U3", 1.0, -1);

  graph.write_put_edge("", "U1", "O1", 1.0, -1);
  graph.write_put_edge("", "O1", "U1", 1.0, -1);
  graph.write_put_edge("", "O1", "U3", 1.0, -1);

  // U2 should have a score from ego (U1), despite the focus being U3

  let neighbors = graph.read_neighbors(
    "",
    "U1",
    "U3",
    NEIGHBORS_INBOUND,
    "",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    100,
  );

  assert_eq!(neighbors[0].0, "U1");
  assert_eq!(neighbors[0].1, "O1");
  assert_eq!(neighbors[1].0, "U1");
  assert_eq!(neighbors[1].1, "U2");
}

#[test]
fn neighbors_omit_opinions_from_self_to_focus() {
  let mut graph = default_graph();

  //         ┌─────────┐
  //        ┌▼─┐      ┌┴─┐
  //   ┌────►U2├──────►O2│
  // ┌─┴┐   └──┘      └┬─┘           │
  // │U1│              │
  // └─▲┘   ┌──┐      ┌▼─┐
  //   └────┤O3◄──────┤U3│
  //        └─┬┘      └▲─┘
  //          └────────┘

  graph.write_put_edge("", "U1", "U2", 1.0, -1);

  graph.write_put_edge("", "U2", "O2", 1.0, -1);
  graph.write_put_edge("", "O2", "U2", 1.0, -1);
  graph.write_put_edge("", "O2", "U3", 1.0, -1);

  graph.write_put_edge("", "U3", "O3", 1.0, -1);
  graph.write_put_edge("", "O3", "U3", 1.0, -1);
  graph.write_put_edge("", "O3", "U1", 1.0, -1);

  // U3 as focus should show only O2 leading to it, and not O3, because
  // O3 is the child of U3, and not and opinion of someone else about U3

  let neighbors = graph.read_neighbors(
    "",
    "U1",
    "U3",
    NEIGHBORS_INBOUND,
    "O",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    100,
  );

  assert_eq!(neighbors[0].0, "U1");
  assert_eq!(neighbors[0].1, "O2");
  assert_eq!(neighbors.len(), 1);
}

#[test]
fn regression_subgraph_from_context_perf() {
  let mut graph = AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 100,
    zero_opinion_num_walks: 50,
    ..AugMultiGraphSettings::default()
  });

  put_testing_edges_4(&mut graph);

  graph.write_new_edges_filter(
    "Ub01f4ad1b03f",
    &[
      105, 105, 105, 105, 105, 105, 105, 105, 10, 10, 10, 10, 10, 10, 10, 10,
      10, 10, 10, 10, 10, 10, 10, 10, 101, 101, 101, 101, 101, 101, 101, 101,
      2, 2, 2, 2, 2, 2, 2, 2, 52, 52, 52, 52, 52, 52, 52, 52, 137, 137, 137,
      137, 137, 137, 137, 137, 41, 41, 41, 41, 41, 41, 41, 41, 8, 8, 8, 8, 8,
      8, 8, 8, 33, 33, 33, 33, 33, 33, 33, 33, 176, 176, 176, 176, 176, 176,
      176, 176, 17, 17, 17, 17, 17, 17, 17, 17, 114, 114, 114, 114, 114, 114,
      114, 114, 83, 83, 83, 83, 83, 83, 83, 83, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2,
      2, 2, 2, 2, 2, 2,
    ],
  );

  graph.write_recalculate_zero();

  let begin = SystemTime::now();
  let get_time =
    || SystemTime::now().duration_since(begin).unwrap().as_millis();

  let _ = graph.read_scores(
    "",
    "U4aafe73f277b",
    "B",
    true,
    2147483647.0,
    false,
    0.0,
    false,
    0,
    100,
  );

  let _ = graph.read_scores(
    "QuestForGlory",
    "U4aafe73f277b",
    "B",
    true,
    2147483647.0,
    false,
    0.0,
    false,
    0,
    100,
  );

  let duration = get_time();

  println!("Duration: {} msec", duration);

  assert!(duration < 200);
}

#[test]
fn vsids_write_edge() {
  let mut graph = AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 500,
    ..AugMultiGraphSettings::default()
  });

  graph.write_put_edge("", "U1", "U4", 3.0, -1);
  graph.write_put_edge("", "U1", "U2", 3.0, 0);
  graph.write_put_edge("", "U1", "U3", 1.0, 20);
  let u12 = graph.read_node_score("", "U1", "U2");
  let u13 = graph.read_node_score("", "U1", "U3");

  assert!(
    u12[0].2 < u13[0].2,
    "Assert that thanks to magnitude, U3 has a higher score than U2"
  );

  // Test deletion of too small edges
  graph.write_put_edge("", "U1", "U4", 1.0, 200);
  let u12_final = graph.read_node_score("", "U1", "U2");
  let u13_final = graph.read_node_score("", "U1", "U3");
  assert!(
    u12_final.is_empty() || u12_final[0].2 == 0.0,
    "U1->U2 edge should not exist"
  );
  assert!(
    u13_final.is_empty() || u13_final[0].2 == 0.0,
    "U1->U3 edge should not exist"
  );
}

#[test]
fn vsids_edges_churn() {
  let mut graph = default_graph();
  graph.vsids.bump_factor = 2.0;

  // Test for correct rescaling and dynamic deletion of smaller edges when
  // adding many edges of ever-increasing magnitude
  for n in 0..1000 {
    let dst = format!("U{}", n + 2);
    graph.write_put_edge("", "U1", &*dst, 1.0, n);
  }

  //  FIXME: This test is too low-level.
  //         It should not rely on edge_weight_normalized.
  //         Such low-level functions are not stable.

  // Check that only the most recent edges remain
  for n in 0..1000 {
    let dst = format!("U{}", n + 2);
    let src_id = *graph.node_ids.get("U1").unwrap();
    let dst_id = *graph.node_ids.get(&dst).unwrap();
    let edge = graph
      .subgraph_from_context("")
      .edge_weight_normalized(src_id, dst_id);
    if n >= 990 {
      // Assuming the last 10 edges remain
      assert!(edge > 0.0, "Edge U1->{} should exist", dst);
    } else {
      assert_eq!(edge, 0.0, "Edge U1->{} should not exist", dst);
    }
  }
}

#[test]
fn regression_recalculate_out_of_bounds() {
  let mut graph = default_graph();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);

  graph.write_recalculate_zero();
}

#[test]
fn regression_delete_self_reference_panic() {
  let mut graph = default_graph();
  graph.write_put_edge("", "Ud57e58e4b20d", "U000000000000", 1.0, -1);
  graph.write_delete_edge("", "U000000000000", "U000000000000", -1);
}

#[test]
fn regression_beacons_clustering() {
  let mut graph = default_graph_zero();

  put_testing_edges_2(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> = graph.read_scores(
    "",
    "U6d2f25cc4264",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  for x in res.iter() {
    println!("{:?}", x);
  }

  let count = res.len();

  for (n, x) in res.iter().enumerate() {
    assert!(x.4 >= 1);
    assert!(x.4 <= 100);

    let percentile = (1 + ((count - n - 1) * 100) / count) as i32;

    assert!(x.4 >= percentile - 25);
    assert!(x.4 <= percentile + 25);
  }
}

//  NOTE: This test is too slow in debug, so we disable it.
#[cfg(not(debug_assertions))]
#[test]
fn regression_oom() {
  let mut graph = AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 10000,
    zero_opinion_num_walks: 10000,
    ..AugMultiGraphSettings::default()
  });

  put_testing_edges_3(&mut graph);

  graph.write_recalculate_zero();
}
#[test]
fn omit_neg_edges_scores_setting() {
  // Create a graph with omit_neg_edges_scores enabled
  let mut graph_omit = AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 50,
    zero_opinion_num_walks: 100,
    omit_neg_edges_scores: true,
    ..AugMultiGraphSettings::default()
  });

  // Create a graph with omit_neg_edges_scores disabled (default)
  let mut graph_include = AugMultiGraph::new(AugMultiGraphSettings {
    num_walks: 50,
    zero_opinion_num_walks: 100,
    omit_neg_edges_scores: false,
    ..AugMultiGraphSettings::default()
  });

  // Add the same edges to both graphs
  // Add the same edges to both graphs
  let edges = vec![
    ("U1".to_string(), "U2".to_string(), -1.0), // Negative edge from U1 to U2
    ("U1".to_string(), "U3".to_string(), 10.0), // Positive edge from U1 to U3
    ("U3".to_string(), "U2".to_string(), 1.0),  // Positive edge from U3 to U2
  ];

  // Make sure that U2 will be removed even in case there is zero opinion for it
  graph_include.write_set_zero_opinion("", "U2", 10.0);
  graph_omit.write_set_zero_opinion("", "U2", 10.0);

  for (src, dst, weight) in edges {
    graph_omit.write_put_edge("", &src, &dst, weight, -1);
    graph_include.write_put_edge("", &src, &dst, weight, -1);
  }

  // Get scores for U1 in both graphs
  let scores_omit = graph_omit.read_scores(
    "",
    "U1",
    "U",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let scores_include = graph_include.read_scores(
    "",
    "U1",
    "U",
    false,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  // Check if U2 is present in both result sets
  let find_node_score =
    |scores: &Vec<(String, String, Weight, Weight, Cluster, Cluster)>,
     node: &str|
     -> Option<Weight> {
      for (_, n, score, _, _, _) in scores {
        if n == node {
          return Some(*score);
        }
      }
      None
    };

  let u2_score_include = find_node_score(&scores_include, "U2");
  let u2_score_omit = find_node_score(&scores_omit, "U2");

  // U2 should have a score in the graph that includes negative edges
  assert!(
    u2_score_include.is_some(),
    "U2 should have a score when negative edges are included"
  );
  assert!(
    u2_score_omit.is_none(),
    "U2 should not have a score when negative edges are omitted"
  );
}
