// Format description for triples from OUT3, and similarly for quadruples from OUT4:
// OUT3/OUT4 contain all possible transitions between 3-graphs and 4-graphs, respectively,
// up to isomorphism. For each transition, there are scores from reference Python
// implementation of MeritRank - both incremental and non-incremental.
//
// The first 3 numbers represent the edge change in the format: <src>, <dst>, <weight>
// For example, 0, 1, -1 means an edge from node 0 to node 1 is added
// (or overwritten) with a weight of -1.
// Or for example, 2, 1, 0 means an edge from node 2 to node 1 is removed.
//
// The next 6 numbers represent the state of the edge weights after the change, in the format:
// [(0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)]
// For example, the entry -1, 1, 0, 0, 0, 1 means the following edges with their weights are present:
// 0->1 (weight -1), 0->2 (weight 1), 2->1 (weight 1)
//
// The next 3 numbers represent the ranking for the three nodes computed from
// scratch (non-incremental)
// <rank 0>, <rank 1>, <rank 2>
//
// The next 3 numbers represent the ranking computed incrementally,
// as the result of the transition from the previous step
// (previous row in the table) to the current one, based on the edge
// change recorded in the first 3 numbers of the table
// <rank_inc 0>, <rank_inc 1>, <rank_inc 2>
//
// For the OUT4 table, the edge weight state format is:
// [(0, 1), (0, 2), (0, 3), (1, 0), (1, 2), (1, 3), (2, 0), (2, 1), (2, 3), (3, 0), (3, 1), (3, 2)]
//
// Note that the initial state is not provided - this is because, firstly,
// it was tedious to write an exception in the loop (one line, lol),
// and secondly, because the state in the last row corresponds
// to the state in the first row. This sequence cyclically traverses
// the entire graph of possible graph states.
// Approximately like this (here one letter represents one specific graph/state):
// A -> B -> C -> D -> C -> B -> A




#[cfg(test)]
mod tests {
  use csv::{ReaderBuilder, StringRecord};
  use flate2::read::GzDecoder;
  use std::error::Error;
  use std::fs::File;
  #[allow(unused_imports)]
  use std::io::{self, prelude::*, BufReader};

  #[test]
  fn csv_test() -> Result<(), Box<dyn Error>> {
    let file_path = "tests/dumps/OUT3.csv";
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    for result in csv_reader.records() {
      let record = result?;
      // Process each record as needed
      println!("{:?}", record);
    }

    Ok(())
  }

  #[test]
  fn csv_test_gunzip() -> Result<(), Box<dyn Error>> {
    let file_path_gz = "tests/dumps/OUT4.csv.gz";
    let file = File::open(file_path_gz)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);
    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    for result in csv_reader.records() {
      let record = result?;
      // Process each record as needed
      println!("{:?}", record.len());
    }

    Ok(())
  }

  use meritrank_core::{MeritRank, Graph, NodeId, assert_approx_eq};
  use std::collections::HashMap;


  #[ignore]
  #[test]
  #[allow(unused_mut)]
  #[allow(unused_variables)]
  fn test_meritrank_short() -> Result<(), Box<dyn Error>> {
    let file_path = "tests/dumps/OUT3.csv";
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    // Read the graph state from the CSV file
    for result in csv_reader.records() {
      let record = result?;

      let w0_1: f64 = record[3].trim().parse()?;
      let w0_2: f64 = record[4].trim().parse()?;
      let w1_0: f64 = record[5].trim().parse()?;
      let w1_2: f64 = record[6].trim().parse()?;
      let w2_0: f64 = record[7].trim().parse()?;
      let w2_1: f64 = record[8].trim().parse()?;

      let rank0: f64 = record[9].trim().parse()?;
      let rank1: f64 = record[10].trim().parse()?;
      let rank2: f64 = record[11].trim().parse()?;

      // Create the complete graph
      let mut complete_graph = Graph::new();
      let incremental_node_ids: Vec<NodeId> = vec![0, 1, 2];

      for node_id in incremental_node_ids {
        complete_graph.get_new_nodeid();
      }

      let _ = complete_graph.set_edge(0, 1, w0_1);
      let _ = complete_graph.set_edge(0, 2, w0_2);
      let _ = complete_graph.set_edge(1, 0, w1_0);
      let _ = complete_graph.set_edge(1, 2, w1_2);
      let _ = complete_graph.set_edge(2, 0, w2_0);
      let _ = complete_graph.set_edge(2, 1, w2_1);

      let mut meritrank = MeritRank::new(complete_graph);

      // calculate merit rank
      // ACHTUNG! To pass, this test requires at least 10,000 walks!
      meritrank.calculate(0, 10000)?;
      // meritrank.calculate(2, 1000)?;
      // meritrank.calculate(3, 1000);

      let rating: HashMap<NodeId, f64> = meritrank.get_ranks(0, None).unwrap_or({
        vec![
          (0, 0.0),
          (1, 0.0),
          (2, 0.0),
        ]
      }).into_iter().collect();

      // check rating
      let r0 = rating.get(&0).unwrap_or(&0.0);
      eprintln!(
        "Rating for node 0: {}, from dump: {}",
        r0,
        rank0
      );
      assert_approx_eq!(r0, rank0, 0.1);

      let r1 = rating.get(&1).unwrap_or(&0.0);
      eprintln!(
        "Rating for node 1: {}, from dump: {}",
        r1,
        rank1
      );
      assert_approx_eq!(r1, rank1, 0.1);

      let r2 = rating.get(&2).unwrap_or(&0.0);
      eprintln!(
        "Rating for node 2: {}, from dump: {}",
        r2,
        rank2
      );
      assert_approx_eq!(r2, rank2, 0.1);
    }

    Ok(())
  }

  // ACTHUNG! The current version of OUT4 is broken! GIGO: nodes' positions
  // are transposed for some result, which makes for unreliable comparisons
  #[ignore]
  #[test]
  fn test_meritrank_long() -> Result<(), Box<dyn Error>> {
    let file_path_gz = "tests/dumps/OUT4.csv.gz";
    let file = File::open(file_path_gz)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);
    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    let mut limit_entries = 100;

    // Read the graph state from the CSV file
    for result in csv_reader.records(){
      let record: StringRecord = result?;


      limit_entries -=1;
      if limit_entries == 0{break}

      let w0_1: f64 = record[3].trim().parse()?;
      let w0_2: f64 = record[4].trim().parse()?;
      let w0_3: f64 = record[5].trim().parse()?;
      let w1_0: f64 = record[6].trim().parse()?;
      let w1_2: f64 = record[7].trim().parse()?;
      let w1_3: f64 = record[8].trim().parse()?;
      let w2_0: f64 = record[9].trim().parse()?;
      let w2_1: f64 = record[10].trim().parse()?;
      let w2_3: f64 = record[11].trim().parse()?;
      let w3_0: f64 = record[12].trim().parse()?;
      let w3_1: f64 = record[13].trim().parse()?;
      let w3_2: f64 = record[14].trim().parse()?;


      let rank0: f64 = record[15].trim().parse()?;
      let rank1: f64 = record[16].trim().parse()?;
      let rank2: f64 = record[17].trim().parse()?;
      let rank3: f64 = record[18].trim().parse()?;

      // Create the complete graph
      let mut complete_graph = Graph::new();
      let node_ids: Vec<NodeId> = vec![0, 1, 2, 3];

      for _ in &node_ids {
        complete_graph.get_new_nodeid();
      }

      let _ = complete_graph.set_edge(0, 1, w0_1);
      let _ = complete_graph.set_edge(0, 2, w0_2);
      let _ = complete_graph.set_edge(0, 3, w0_3);
      let _ = complete_graph.set_edge(1, 0, w1_0);
      let _ = complete_graph.set_edge(1, 2, w1_2);
      let _ = complete_graph.set_edge(1, 3, w1_3);
      let _ = complete_graph.set_edge(2, 0, w2_0);
      let _ = complete_graph.set_edge(2, 1, w2_1);
      let _ = complete_graph.set_edge(2, 3, w2_3);
      let _ = complete_graph.set_edge(3, 0, w3_0);
      let _ = complete_graph.set_edge(3, 1, w3_1);
      let _ = complete_graph.set_edge(3, 2, w3_2);

      let mut meritrank = MeritRank::new(complete_graph);

      // calculate merit rank
      meritrank.calculate(0, 10000)?;

      let rating: Vec<(NodeId, f64)> =
        meritrank.get_ranks(0, None).unwrap_or_default();

      // check rating
      eprintln!(
        "Rating for node 0: {}, from dump: {}",
        rating.get(0).unwrap_or(&(0, 0.0)).1,
        rank0
      );
      eprintln!(
        "Rating for node 1: {}, from dump: {}",
        rating.get(1).unwrap_or(&(1, 0.0)).1,
        rank1
      );
      eprintln!(
        "Rating for node 2: {}, from dump: {}",
        rating.get(2).unwrap_or(&(2, 0.0)).1,
        rank2
      );
      eprintln!(
        "Rating for node 3: {}, from dump: {}",
        rating.get(3).unwrap_or(&(3, 0.0)).1,
        rank3
      );

      // You can add assertions if needed
      //assert_approx_eq!(rating.get(0).unwrap_or(&(0, 0.0)).1, rank0, 0.1);
      //assert_approx_eq!(rating.get(1).unwrap_or(&(1, 0.0)).1, rank1, 0.1);
      //assert_approx_eq!(rating.get(2).unwrap_or(&(2, 0.0)).1, rank2, 0.1);
      //assert_approx_eq!(rating.get(3).unwrap_or(&(3, 0.0)).1, rank3, 0.1);
    }

    Ok(())
  }

  #[ignore]
  #[test]
  fn test_meritrank_incremental_short() -> Result<(), Box<dyn Error>> {
    let file_path = "tests/dumps/OUT3.csv";
    let reader = BufReader::new(File::open(file_path)?);
    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    let mut meritrank_opt : Option<MeritRank> = None;

    let records_to_skip = 0;
    for (index, result) in csv_reader.records().enumerate() {
      if index < records_to_skip {
        continue;
      }

      let record = match result {
        Ok(record) => {
          record
        }
        Err(e) => {
          eprintln!("Error when reading record: {}", e);
          return Ok(());
        }
      };
      println!("{:?}", record);

      if let Some(meritrank) = &mut meritrank_opt {
        let source: NodeId = record[0].trim().parse()?;
        let destination: NodeId = record[1].trim().parse()?;
        let weight: f64 = record[2].trim().parse()?;

        // then recalculate merit rank for updated graph
        meritrank.set_edge(source, destination, weight);
      } else {
        let mut graph = Graph::new();
        graph.get_new_nodeid();
        graph.get_new_nodeid();
        graph.get_new_nodeid();

        let weights: Vec<f64> = record
          .iter()
          .skip(3)
          .take(6)
          .map(|field| field.trim().parse())
          .collect::<Result<Vec<_>, _>>()?
          .try_into()?;

        let _ = graph.set_edge(0, 1, weights[0]);
        let _ = graph.set_edge(0, 2, weights[1]);
        let _ = graph.set_edge(1, 0, weights[2]);
        let _ = graph.set_edge(1, 2, weights[3]);
        let _ = graph.set_edge(2, 0, weights[4]);
        let _ = graph.set_edge(2, 1, weights[5]);

        meritrank_opt = Some(MeritRank::new(graph));
        meritrank_opt.as_mut().unwrap().calculate(0, 20000)?;
      }

      let rating: HashMap<NodeId, f64> = meritrank_opt
        .as_ref()
        .ok_or("MeritRank not initialized")?
        .get_ranks(0, None)
        .unwrap_or_default()
        .into_iter()
        .collect();

      let rank0: f64 = record[9].trim().parse()?;
      let rank1: f64 = record[10].trim().parse()?;
      let rank2: f64 = record[11].trim().parse()?;

      // check rating
      let r0 = rating.get(&0).unwrap_or(&0.0);
      eprintln!(
        "Rating for node 0: {}, from dump: {}",
        r0,
        rank0
      );
      assert_approx_eq!(r0, rank0, 0.1);

      let r1 = rating.get(&1).unwrap_or(&0.0);
      eprintln!(
        "Rating for node 1: {}, from dump: {}",
        r1,
        rank1
      );
      assert_approx_eq!(r1, rank1, 0.1);

      let r2 = rating.get(&2).unwrap_or(&0.0);
      eprintln!(
        "Rating for node 2: {}, from dump: {}",
        r2,
        rank2
      );
      assert_approx_eq!(r2, rank2, 0.1);
    }

    Ok(())
  }


  #[ignore]
  #[test]
  fn test_meritrank_incremental_long() -> Result<(), Box<dyn Error>> {
    let file_path_gz = "tests/dumps/OUT4.csv.gz";
    let file = File::open(file_path_gz)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);
    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    let mut meritrank_opt: Option<MeritRank> = None;

    for result in csv_reader.records() {
      let record = result?;
      println!("{:?}", record);

      if let Some(meritrank) = &mut meritrank_opt {
        let source: NodeId = record[0].trim().parse()?;
        let destination: NodeId = record[1].trim().parse()?;
        let weight: f64 = record[2].trim().parse()?;

        // then recalculate merit rank for updated graph
        meritrank.set_edge(source, destination, weight);
      } else {
        let mut graph = Graph::new();
        let node_ids: Vec<NodeId> = vec![0, 1, 2, 3];

        for _ in &node_ids {
          graph.get_new_nodeid();
        }

        let weights: Vec<f64> = record
          .iter()
          .skip(3)
          .take(12)
          .map(|field| field.trim().parse())
          .collect::<Result<Vec<_>, _>>()?
          .try_into()?;

        let edges = [
          (0, 1), (0, 2), (0, 3),
          (1, 0), (1, 2), (1, 3),
          (2, 0), (2, 1), (2, 3),
          (3, 0), (3, 1), (3, 2)
        ];

        for (i, &(src, dest)) in edges.iter().enumerate() {
          let _ = graph.set_edge(src, dest, weights[i]);
        }

        meritrank_opt = Some(MeritRank::new(graph));
        meritrank_opt.as_mut().unwrap().calculate(0, 500)?;
      }

      let rating: Vec<(NodeId, f64)> = meritrank_opt
        .as_ref()
        .ok_or("MeritRank not initialized")?
        .get_ranks(0, None)
        .unwrap_or_default();

      let expected_ranks: [f64; 4] = [
        record[15].trim().parse()?,
        record[16].trim().parse()?,
        record[17].trim().parse()?,
        record[18].trim().parse()?
      ];

      for (i, &expected_rank) in expected_ranks.iter().enumerate() {
        eprintln!(
          "Rating for node {}: {}, from dump: {}",
          i,
          rating.get(i).unwrap_or(&(i as NodeId, 0.0)).1,
          expected_rank
        );
      }
    }

    Ok(())
  }
}
