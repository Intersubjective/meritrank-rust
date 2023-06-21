MeritRank
=========

MeritRank is a Rust library for computing merit scores and rankings in a directed graph.

Features
--------

- Efficient computation of merit scores for nodes in a graph.
- Ranking nodes based on their merit scores.
- Flexible configuration options for customizing the ranking process.
- Support for weighted graphs and personalized rankings.

Installation
------------

To use `meritrank` in your Rust project, add the following line to your `Cargo.toml` file:

```toml
[dependencies]
meritrank = "0.4.0.1.3"
```

Usage
-----

To use `meritrank`, you need to create a graph and compute the merit scores for its nodes. Here's a basic example:

```rust
use meritrank::{MyGraph, MeritRank};

fn main() {
    // Create a graph
    let mut graph = MyGraph::new();
    graph.add_edge("A", "B");
    graph.add_edge("B", "C");
    graph.add_edge("C", "D");
    graph.add_edge("D", "E");
    graph.add_edge("E", "F");
    
    // Compute merit scores
    let merit_rank = MeritRank::new(&graph);
    let scores = merit_rank.compute_scores();
    
    // Get the ranked nodes
    let ranked_nodes = merit_rank.get_ranked_nodes();
    
    // Print the scores and ranks
    for (node, score) in scores.iter() {
        println!("Node: {}, Score: {}", node, score);
    }
    
    for (rank, node) in ranked_nodes.iter().enumerate() {
        println!("Rank: {}, Node: {}", rank + 1, node);
    }
}
```

This example creates a simple graph and computes the merit scores for its nodes using the `MeritRank` struct. It then retrieves the ranked nodes based on their scores.

Documentation
-------------

For detailed usage instructions and API reference, please refer to the [documentation](https://docs.rs/meritrank) [*expected*].

Contributing
------------

Contributions are welcome! If you have any bug reports, feature requests, or suggestions, please open an issue on the [GitHub repository](https://github.com/vsradkevich/meritrank). Pull requests are also encouraged.

License
-------

`meritrank` is licensed under the MIT License. See the [LICENSE](https://github.com/vsradkevich/meritrank/blob/main/LICENSE) file for more information.

Maintainer
----------

`meritrank` is actively maintained by [Vladimir Radkevich](https://github.com/vsradkevich). Feel free to reach out if you have any questions or need assistance.

Enjoy using `meritrank` for ranking nodes in your graphs!