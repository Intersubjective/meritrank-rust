#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs::File;
    #[allow(unused_imports)]
    use std::io::{self, prelude::*, BufReader};
    use flate2::read::GzDecoder;
    use csv::ReaderBuilder;

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


    use meritrank::{Node, NodeId, MyGraph, MeritRank};

    #[test]
    #[allow(unused_mut)]
    #[allow(unused_variables)]
    fn meritrank_test() -> Result<(), Box<dyn Error>> {
        let file_path = "tests/dumps/OUT3.csv";
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut csv_reader = ReaderBuilder::new().from_reader(reader);


        // complete_graph.add_node(0.into());
        // complete_graph.add_node(1.into());
        // complete_graph.add_node(2.into());

        // Create the incremental graph
        let mut incremental_graph = MyGraph::new();

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
            let mut complete_graph = MyGraph::new();
            let incremental_node_ids: Vec<NodeId> = vec![0.into(), 1.into(), 2.into()];

            for node_id in incremental_node_ids {
                complete_graph.add_node(Node::new(node_id));
            }

            complete_graph.add_edge(0.into(), 1.into(), w0_1);
            complete_graph.add_edge(0.into(), 2.into(), w0_2);
            complete_graph.add_edge(1.into(), 0.into(), w1_0);
            complete_graph.add_edge(1.into(), 2.into(), w1_2);
            complete_graph.add_edge(2.into(), 0.into(), w2_0);
            complete_graph.add_edge(2.into(), 1.into(), w2_1);

            let mut meritrank = MeritRank::new(complete_graph).unwrap();

            // calculate merit rank
            meritrank.calculate(0.into(), 2500)?;
            // meritrank.calculate(1.into(), 1000)?;
            // meritrank.calculate(2.into(), 1000);

            let rating: HashMap<NodeId, f64> = meritrank.get_ranks(0.into(), None).unwrap_or({
                let mut hashmap = HashMap::new();
                hashmap.insert(0.into(), 0.0);
                hashmap.insert(1.into(), 0.0);
                hashmap.insert(2.into(), 0.0);
                hashmap
            });

            // check rating
            eprintln!("Rating for node 0: {}, from dump: {}",
                      rating.get(&0.into()).unwrap_or(&0.0), rank0);
            eprintln!("Rating for node 1: {}, from dump: {}",
                      rating.get(&1.into()).unwrap_or(&0.0), rank1);
            eprintln!("Rating for node 2: {}, from dump: {}",
                      rating.get(&2.into()).unwrap_or(&0.0), rank2);

            // assert_eq!(rating[&0.into()], rank0);
            // assert_eq!(rating[&1.into()], rank1);
            // assert_eq!(rating[&2.into()], rank2);
        }

        Ok(())

        // // calculate merit rank
        // let rating = match newrank.calculate(3.into(), 10000) {
        //     Ok(r) => r,
        //     Err(e) => panic!("Error: {}", e),
        // };

        // print rating
        // println!("Rating: {:?}", rating);
    }

    #[test]
    #[allow(unused_mut)]
    #[allow(unused_variables)]
    fn meritrank_test_incremental() -> Result<(), Box<dyn Error>> {
        let file_path = "tests/dumps/OUT3.csv";
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut csv_reader = ReaderBuilder::new().from_reader(reader);

        // complete_graph.add_node(0.into());
        // complete_graph.add_node(1.into());
        // complete_graph.add_node(2.into());

        // Create the incremental graph
        let mut incremental_graph = MyGraph::new();

        // Read the graph state from the CSV file
        for result in csv_reader.records() {
            let record = result?;

            if incremental_graph.is_empty() {
                // add nodes
                incremental_graph.add_node(0.into());
                incremental_graph.add_node(1.into());
                incremental_graph.add_node(2.into());

                // parse edges
                let w0_1: f64 = record[3].trim().parse()?;
                let w0_2: f64 = record[4].trim().parse()?;
                let w1_0: f64 = record[5].trim().parse()?;
                let w1_2: f64 = record[6].trim().parse()?;
                let w2_0: f64 = record[7].trim().parse()?;
                let w2_1: f64 = record[8].trim().parse()?;

                incremental_graph.add_edge(0.into(), 1.into(), w0_1);
                incremental_graph.add_edge(0.into(), 2.into(), w0_2);
                incremental_graph.add_edge(1.into(), 0.into(), w1_0);
                incremental_graph.add_edge(1.into(), 2.into(), w1_2);
                incremental_graph.add_edge(2.into(), 0.into(), w2_0);
                incremental_graph.add_edge(2.into(), 1.into(), w2_1);
            } else {
                let source: NodeId = record[0].trim().parse()?;
                let destination: NodeId = record[1].trim().parse()?;
                let weight: f64 = record[2].trim().parse()?;

                incremental_graph.add_edge(source, destination, weight);
            }

            // calculate merit rank
            let mut meritrank = MeritRank::new(incremental_graph.clone()).unwrap();

            meritrank.calculate(0.into(), 2500)?;

            let rating: HashMap<NodeId, f64> = meritrank.get_ranks(0.into(), None).unwrap_or({
                let mut hashmap = HashMap::new();
                hashmap.insert(0.into(), 0.0);
                hashmap.insert(1.into(), 0.0);
                hashmap.insert(2.into(), 0.0);
                hashmap
            });

            let rank0: f64 = record[9].trim().parse()?;
            let rank1: f64 = record[10].trim().parse()?;
            let rank2: f64 = record[11].trim().parse()?;

            // check rating
            eprintln!("Rating for node 0: {}, from dump: {}",
                      rating.get(&0.into()).unwrap_or(&0.0), rank0);
            eprintln!("Rating for node 1: {}, from dump: {}",
                      rating.get(&1.into()).unwrap_or(&0.0), rank1);
            eprintln!("Rating for node 2: {}, from dump: {}",
                      rating.get(&2.into()).unwrap_or(&0.0), rank2);


            // // Create the complete graph
            // let mut complete_graph = MyGraph::new();
            // let incremental_node_ids: Vec<NodeId> = vec![0.into(), 1.into(), 2.into()];
            //
            // for node_id in incremental_node_ids {
            //     complete_graph.add_node(Node::new(node_id));
            // }
            //
            // complete_graph.add_edge(0.into(), 1.into(), w0_1);
            // complete_graph.add_edge(0.into(), 2.into(), w0_2);
            // complete_graph.add_edge(1.into(), 0.into(), w1_0);
            // complete_graph.add_edge(1.into(), 2.into(), w1_2);
            // complete_graph.add_edge(2.into(), 0.into(), w2_0);
            // complete_graph.add_edge(2.into(), 1.into(), w2_1);
            //
            // let mut meritrank = MeritRank::new(complete_graph).unwrap();
            //
            // // calculate merit rank
            // meritrank.calculate(0.into(), 250000)?;
            // // meritrank.calculate(1.into(), 1000)?;
            // // meritrank.calculate(2.into(), 1000);
            //
            // let rating: HashMap<NodeId, f64> = meritrank.get_ranks(0.into(), None).unwrap_or({
            //     let mut hashmap = HashMap::new();
            //     hashmap.insert(0.into(), 0.0);
            //     hashmap.insert(1.into(), 0.0);
            //     hashmap.insert(2.into(), 0.0);
            //     hashmap
            // });
            //
            // // check rating
            // eprintln!("Rating for node 0: {}, from dump: {}",
            //           rating.get(&0.into()).unwrap_or(&0.0), rank0);
            // eprintln!("Rating for node 1: {}, from dump: {}",
            //           rating.get(&1.into()).unwrap_or(&0.0), rank1);
            // eprintln!("Rating for node 2: {}, from dump: {}",
            //           rating.get(&2.into()).unwrap_or(&0.0), rank2);
            //
            // // assert_eq!(rating[&0.into()], rank0);
            // // assert_eq!(rating[&1.into()], rank1);
            // // assert_eq!(rating[&2.into()], rank2);
        }

        Ok(())

        // // calculate merit rank
        // let rating = match newrank.calculate(3.into(), 10000) {
        //     Ok(r) => r,
        //     Err(e) => panic!("Error: {}", e),
        // };

        // print rating
        // println!("Rating: {:?}", rating);
    }
}
