// // Use the constants in your code
// if ASSERT {
//     println!("Assertion enabled");
// }
//
// if OPTIMIZE_INVALIDATION {
//     println!("Invalidation optimization enabled");
// }

use meritrank::{MeritRank, MyGraph, Node};

fn main() {
    println!("Hello, world!");

    // create graph
    let mut graph = MyGraph::new();

    // add nodes
    graph.add_node(Node::new(1.into()));
    graph.add_node(Node::new(2.into()));
    graph.add_node(Node::new(3.into()));
    graph.add_node(Node::new(4.into()));

    // graph.add_node(Node::new(5.into()));

    graph.add_edge(1.into(), 2.into(), 0.98);
    graph.add_edge(2.into(), 3.into(), 1.0);
    graph.add_edge(3.into(), 4.into(), 1.0);
    // graph.add_edge(4.into(), 1.into(), 1.0);

    // graph.add_edge(3.into(), 4.into(), 1.0);
    // graph.add_edge(4.into(), 1.into(), 1.0);

    // create merit rank
    let mut newrank = match MeritRank::new(graph) {
        Ok(g) => {
            println!("Graph created");
            g
        }
        Err(e) => panic!("MeritRank Error: {}", e),
    };

    // calculate merit rank
    match newrank.calculate(1.into(), 100) {
        Ok(_) => {
            println!("Calculation successful.");
        }
        Err(e) => {
            eprintln!("Calculation Error: {}", e);
        }
    }

    let node_scores = newrank.get_node_score(1.into(), 5.into());

    println!("Node scores: {:?}", node_scores);

    newrank.add_node(5.into());

    // add edges
    // newrank.add_edge(1.into(), 2.into(), 0.7);
    // newrank.add_edge(2.into(), 3.into(), 1.0);
    // newrank.add_edge(2.into(), 4.into(), 1.0);
    // newrank.add_edge(3.into(), 4.into(), 1.0);
    // newrank.add_edge(2.into(), 3.into(), -1.0);
    // newrank.add_edge(2.into(), 4.into(), 1.0);
    println!("Adding edges 2 -> 4");
    newrank.add_edge(2.into(), 4.into(), 1.0);
    println!("Adding edges 3 -> 4");
    newrank.add_edge(3.into(), 4.into(), -1.0);
    println!("Adding edges 4 -> 5");
    newrank.add_edge(4.into(), 5.into(), 1.0);
    println!("Adding edges 3 -> 5");
    newrank.add_edge(3.into(), 5.into(), -1.0);
    // newrank.add_edge(3.into(), 5.into(), 1.0);
    // newrank.add_edge(4.into(), 5.into(), 1.0);
    // newrank.add_edge(4.into(), 5.into(), 1.0);
    // newrank.add_edge(4.into(), 5.into(), 1.0);
    // newrank.add_edge(4.into(), 5.into(), 0.0);

    // calculate merit rank
    let ratings = newrank
        .get_ranks(1.into(), None)
        .unwrap_or_default();

    // print rating
    println!("Rating: {:?}", ratings);
}
