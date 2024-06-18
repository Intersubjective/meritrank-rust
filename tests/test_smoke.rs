#[cfg(test)]
mod tests {
    use meritrank::{MeritRank, MyGraph, NodeId};

    #[test]
    fn meritrank_smoke() {
      let mut rank = MeritRank::new(MyGraph::new()).unwrap();

      rank.add_node(NodeId::UInt(0));
      rank.add_node(NodeId::UInt(1));
      rank.add_edge(NodeId::UInt(0),NodeId::UInt( 1), 1.0);
      rank.add_node(NodeId::UInt(2));
      rank.add_node(NodeId::UInt(3));
      rank.add_edge(NodeId::UInt(2),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(4));
      rank.add_node(NodeId::UInt(5));
      rank.add_edge(NodeId::UInt(4),NodeId::UInt( 5), 9.0);
      rank.add_node(NodeId::UInt(6));
      rank.add_node(NodeId::UInt(7));
      rank.add_edge(NodeId::UInt(6),NodeId::UInt( 7), 7.0);
      rank.add_node(NodeId::UInt(8));
      rank.add_edge(NodeId::UInt(8),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(9));
      rank.add_node(NodeId::UInt(10));
      rank.add_edge(NodeId::UInt(9), NodeId::UInt(10), 3.0);
      rank.add_node(NodeId::UInt(11));
      rank.add_edge(NodeId::UInt(11),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(12));
      rank.add_edge(NodeId::UInt(12),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(13));
      rank.add_node(NodeId::UInt(14));
      rank.add_edge(NodeId::UInt(13), NodeId::UInt(14), -1.0);
      rank.add_node(NodeId::UInt(15));
      rank.add_edge(NodeId::UInt(15),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(16));
      rank.add_node(NodeId::UInt(17));
      rank.add_edge(NodeId::UInt(16), NodeId::UInt(17), -1.0);
      rank.add_node(NodeId::UInt(18));
      rank.add_edge(NodeId::UInt(18),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(19));
      rank.add_node(NodeId::UInt(20));
      rank.add_edge(NodeId::UInt(19), NodeId::UInt(20), 1.0);
      rank.add_node(NodeId::UInt(21));
      rank.add_node(NodeId::UInt(22));
      rank.add_edge(NodeId::UInt(21), NodeId::UInt(22), 1.0);
      rank.add_node(NodeId::UInt(23));
      rank.add_node(NodeId::UInt(24));
      rank.add_edge(NodeId::UInt(23), NodeId::UInt(24), 0.0);
      rank.add_node(NodeId::UInt(25));
      rank.add_edge(NodeId::UInt(9), NodeId::UInt(25), 1.0);
      rank.add_node(NodeId::UInt(26));
      rank.add_node(NodeId::UInt(27));
      rank.add_edge(NodeId::UInt(26), NodeId::UInt(27), 1.0);
      rank.add_node(NodeId::UInt(28));
      rank.add_edge(NodeId::UInt(28),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(29));
      rank.add_edge(NodeId::UInt(29),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(30));
      rank.add_edge(NodeId::UInt(30), NodeId::UInt(22), 4.0);
      rank.add_node(NodeId::UInt(31));
      rank.add_node(NodeId::UInt(32));
      rank.add_edge(NodeId::UInt(31), NodeId::UInt(32), 1.0);
      rank.add_node(NodeId::UInt(33));
      rank.add_edge(NodeId::UInt(33),NodeId::UInt( 3), 1.0);
      rank.add_node(NodeId::UInt(34));
      rank.add_edge(NodeId::UInt(13), NodeId::UInt(34), -1.0);
      rank.add_node(NodeId::UInt(35));
      rank.add_edge(NodeId::UInt(9), NodeId::UInt(35), 1.0);
      rank.add_edge(NodeId::UInt(35), NodeId::UInt(14), -1.0);

      rank.calculate(NodeId::UInt(18), 100).unwrap();
      rank.calculate(NodeId::UInt(6), 100).unwrap();
      rank.calculate(NodeId::UInt(0), 100).unwrap();
      rank.calculate(NodeId::UInt(30), 100).unwrap();
      rank.calculate(NodeId::UInt(4), 100).unwrap();
      rank.calculate(NodeId::UInt(31), 100).unwrap();
      rank.calculate(NodeId::UInt(16), 100).unwrap();
      rank.calculate(NodeId::UInt(8), 100).unwrap();
      rank.calculate(NodeId::UInt(35), 100).unwrap();
      rank.calculate(NodeId::UInt(21), 100).unwrap();
      rank.calculate(NodeId::UInt(2), 100).unwrap();
      rank.calculate(NodeId::UInt(28), 100).unwrap();
      rank.calculate(NodeId::UInt(12), 100).unwrap();
      rank.calculate(NodeId::UInt(33), 100).unwrap();
      rank.calculate(NodeId::UInt(15), 100).unwrap();
      rank.calculate(NodeId::UInt(11), 100).unwrap();
      rank.calculate(NodeId::UInt(3), 100).unwrap();
      rank.calculate(NodeId::UInt(19), 100).unwrap();
      rank.calculate(NodeId::UInt(23), 100).unwrap();
      rank.calculate(NodeId::UInt(9), 100).unwrap();
      rank.calculate(NodeId::UInt(27), 100).unwrap();
      rank.calculate(NodeId::UInt(29), 100).unwrap();

      rank.add_edge(NodeId::UInt(3), NodeId::UInt(35), 0.6857579217349542);
    }
}
