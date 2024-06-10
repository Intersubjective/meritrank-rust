#[cfg(test)]
mod tests {
    use meritrank::{MeritRank, MyGraph};

    #[test]
    fn meritrank_smoke() {
      let mut rank = MeritRank::new(MyGraph::new()).unwrap();

      rank.add_node(0);
      rank.add_node(1);
      rank.add_edge(0, 1, 1.0);
      rank.add_node(2);
      rank.add_node(3);
      rank.add_edge(2, 3, 1.0);
      rank.add_node(4);
      rank.add_node(5);
      rank.add_edge(4, 5, 9.0);
      rank.add_node(6);
      rank.add_node(7);
      rank.add_edge(6, 7, 7.0);
      rank.add_node(8);
      rank.add_edge(8, 3, 1.0);
      rank.add_node(9);
      rank.add_node(10);
      rank.add_edge(9, 10, 3.0);
      rank.add_node(11);
      rank.add_edge(11, 3, 1.0);
      rank.add_node(12);
      rank.add_edge(12, 3, 1.0);
      rank.add_node(13);
      rank.add_node(14);
      rank.add_edge(13, 14, -1.0);
      rank.add_node(15);
      rank.add_edge(15, 3, 1.0);
      rank.add_node(16);
      rank.add_node(17);
      rank.add_edge(16, 17, -1.0);
      rank.add_node(18);
      rank.add_edge(18, 3, 1.0);
      rank.add_node(19);
      rank.add_node(20);
      rank.add_edge(19, 20, 1.0);
      rank.add_node(21);
      rank.add_node(22);
      rank.add_edge(21, 22, 1.0);
      rank.add_node(23);
      rank.add_node(24);
      rank.add_edge(23, 24, 0.0);
      rank.add_node(25);
      rank.add_edge(9, 25, 1.0);
      rank.add_node(26);
      rank.add_node(27);
      rank.add_edge(26, 27, 1.0);
      rank.add_node(28);
      rank.add_edge(28, 3, 1.0);
      rank.add_node(29);
      rank.add_edge(29, 3, 1.0);
      rank.add_node(30);
      rank.add_edge(30, 22, 4.0);
      rank.add_node(31);
      rank.add_node(32);
      rank.add_edge(31, 32, 1.0);
      rank.add_node(33);
      rank.add_edge(33, 3, 1.0);
      rank.add_node(34);
      rank.add_edge(13, 34, -1.0);
      rank.add_node(35);
      rank.add_edge(9, 35, 1.0);
      rank.add_edge(35, 14, -1.0);

      rank.calculate(18, 100).unwrap();
      rank.calculate(6, 100).unwrap();
      rank.calculate(0, 100).unwrap();
      rank.calculate(30, 100).unwrap();
      rank.calculate(4, 100).unwrap();
      rank.calculate(31, 100).unwrap();
      rank.calculate(16, 100).unwrap();
      rank.calculate(8, 100).unwrap();
      rank.calculate(35, 100).unwrap();
      rank.calculate(21, 100).unwrap();
      rank.calculate(2, 100).unwrap();
      rank.calculate(28, 100).unwrap();
      rank.calculate(12, 100).unwrap();
      rank.calculate(33, 100).unwrap();
      rank.calculate(15, 100).unwrap();
      rank.calculate(11, 100).unwrap();
      rank.calculate(3, 100).unwrap();
      rank.calculate(19, 100).unwrap();
      rank.calculate(23, 100).unwrap();
      rank.calculate(9, 100).unwrap();
      rank.calculate(27, 100).unwrap();
      rank.calculate(29, 100).unwrap();

      rank.add_edge(3, 35, 0.6857579217349542);
    }
}
