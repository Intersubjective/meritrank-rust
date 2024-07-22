// // Use the constants in your code
// if ASSERT {
//   println!("Assertion enabled");
// }
//
// if OPTIMIZE_INVALIDATION {
//   println!("Invalidation optimization enabled");
// }

use meritrank::{MeritRank, Graph, NodeId};

fn main() {
  println!("Hello, world!");

  // create graph
  let mut graph = Graph::new();



  let mut nodes: Vec<NodeId> = Vec::new();

  for _ in 0..5 {
    nodes.push(graph.get_new_nodeid());

  }

  let _ = graph.add_edge(nodes[1], nodes[2], 0.98).unwrap();
  let _ = graph.add_edge(nodes[2], nodes[3], 1.00).unwrap();
  let _ = graph.add_edge(nodes[3], nodes[4], 1.00).unwrap();

  // create merit rank
  let mut newrank = match MeritRank::new(graph) {
    Ok(g) => {
      println!("Graph created");
      g
    }
    Err(e) => panic!("MeritRank Error: {}", e),
  };

  // calculate merit rank
  match newrank.calculate(1, 100) {
    Ok(_) => {
      println!("Calculation successful.");
    }
    Err(e) => {
      eprintln!("Calculation Error: {}", e);
    }
  }

  let node_scores = newrank.get_node_score(1, 5);

  println!("Node scores: {:?}", node_scores);

  nodes.push(newrank.get_new_nodeid());

  println!("Adding edges 2 -> 4");
  newrank.add_edge(nodes[2], nodes[4], 1.0);
  println!("Adding edges 3 -> 4");
  newrank.add_edge(nodes[3], nodes[4], -1.0);
  println!("Adding edges 4 -> 5");
  newrank.add_edge(nodes[4], nodes[5], 1.0);
  println!("Adding edges 3 -> 5");
  newrank.add_edge(nodes[3], nodes[5], -1.0);

  // calculate merit rank
  let ratings = newrank
    .get_ranks(1, None)
    .unwrap_or_default();

  // print rating
  println!("Rating: {:?}", ratings);
}
