// // Use the constants in your code
// if ASSERT {
//     println!("Assertion enabled");
// }
//
// if OPTIMIZE_INVALIDATION {
//     println!("Invalidation optimization enabled");
// }


use meritrank::{Node, MyGraph, MeritRank};

fn main() {
    println!("Hello, world!");

    let mut graph = MyGraph::new();

    graph.add_node(Node::new(1.into()));
    graph.add_node(Node::new(2.into()));
    graph.add_node(Node::new(3.into()));
    graph.add_node(Node::new(4.into()));

    let mut newrank = match MeritRank::new(graph) {
        Ok(g) => {
            println!("Graph created");
            g
        }
        Err(e) => panic!("Error: {}", e),
    };

    newrank.add_edge(1.into(), 2.into(), 1.0);
    newrank.add_edge(1.into(), 3.into(), 1.0);
    newrank.add_edge(1.into(), 4.into(), 1.0);

    // calculate merit rank
    let rating = match newrank.calculate(3.into(), 10000) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    // print rating
    println!("Rating: {:?}", rating);
}