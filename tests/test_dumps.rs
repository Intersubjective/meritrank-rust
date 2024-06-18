#[cfg(test)]
mod tests {
    use csv::ReaderBuilder;
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

    use meritrank::{MeritRank, MyGraph, Node, NodeId};
    use std::collections::HashMap;


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
            meritrank.calculate(0.into(), 250)?;
            // meritrank.calculate(1.into(), 1000)?;
            // meritrank.calculate(2.into(), 1000);

            let rating: HashMap<NodeId, f64> = meritrank.get_ranks(0.into(), None).unwrap_or({
                vec![
                    (0.into(), 0.0),
                    (1.into(), 0.0),
                    (2.into(), 0.0),
                ]
            }).into_iter().collect();

            // check rating
            eprintln!(
                "Rating for node 0: {}, from dump: {}",
                rating.get(&0.into()).unwrap_or(&0.0),
                rank0
            );
            eprintln!(
                "Rating for node 1: {}, from dump: {}",
                rating.get(&1.into()).unwrap_or(&0.0),
                rank1
            );
            eprintln!(
                "Rating for node 2: {}, from dump: {}",
                rating.get(&2.into()).unwrap_or(&0.0),
                rank2
            );

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
    fn test_meritrank_long() -> Result<(), Box<dyn Error>> {
        let file_path_gz = "tests/dumps/OUT4.csv.gz";
        let file = File::open(file_path_gz)?;
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);
        let mut csv_reader = ReaderBuilder::new().from_reader(reader);

        // Read the graph state from the CSV file
        for result in csv_reader.records() {
            let record = result?;

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
            let mut complete_graph = MyGraph::new();
            let node_ids: Vec<NodeId> = vec![0.into(), 1.into(), 2.into(), 3.into()];

            for node_id in &node_ids {
                complete_graph.add_node(Node::new(*node_id));
            }

            complete_graph.add_edge(0.into(), 1.into(), w0_1);
            complete_graph.add_edge(0.into(), 2.into(), w0_2);
            complete_graph.add_edge(0.into(), 3.into(), w0_3);
            complete_graph.add_edge(1.into(), 0.into(), w1_0);
            complete_graph.add_edge(1.into(), 2.into(), w1_2);
            complete_graph.add_edge(1.into(), 3.into(), w1_3);
            complete_graph.add_edge(2.into(), 0.into(), w2_0);
            complete_graph.add_edge(2.into(), 1.into(), w2_1);
            complete_graph.add_edge(2.into(), 3.into(), w2_3);
            complete_graph.add_edge(3.into(), 0.into(), w3_0);
            complete_graph.add_edge(3.into(), 1.into(), w3_1);
            complete_graph.add_edge(3.into(), 2.into(), w3_2);

            let mut meritrank = MeritRank::new(complete_graph).unwrap();

            // calculate merit rank
            meritrank.calculate(0.into(), 25)?;

            let rating: Vec<(NodeId, f64)> =
                meritrank.get_ranks(0.into(), None).unwrap_or_default();

            // check rating
            eprintln!(
                "Rating for node 0: {}, from dump: {}",
                rating.get(0).unwrap_or(&(0.into(), 0.0)).1,
                rank0
            );
            eprintln!(
                "Rating for node 1: {}, from dump: {}",
                rating.get(1).unwrap_or(&(1.into(), 0.0)).1,
                rank1
            );
            eprintln!(
                "Rating for node 2: {}, from dump: {}",
                rating.get(2).unwrap_or(&(2.into(), 0.0)).1,
                rank2
            );
            eprintln!(
                "Rating for node 3: {}, from dump: {}",
                rating.get(3).unwrap_or(&(3.into(), 0.0)).1,
                rank3
            );

            // You can add assertions if needed
            // assert_eq!(rating[&0.into()], rank0);
            // assert_eq!(rating[&1.into()], rank1);
            // assert_eq!(rating[&2.into()], rank2);
            // assert_eq!(rating[&3.into()], rank3);
        }

        Ok(())
    }

    #[test]
    fn test_meritrank_incremental_short() -> Result<(), Box<dyn Error>> {
        let file_path = "tests/dumps/OUT3.csv";
        let reader = BufReader::new(File::open(file_path)?);
        let mut csv_reader = ReaderBuilder::new().from_reader(reader);

        let mut meritrank_opt: Option<MeritRank> = None;

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
                    eprintln!("Ошибка при чтении записи: {}", e);
                    return Ok(());
                }
            };
            println!("{:?}", record);

            if let Some(meritrank) = &mut meritrank_opt {
                let source: NodeId = record[0].trim().parse()?;
                let destination: NodeId = record[1].trim().parse()?;
                let weight: f64 = record[2].trim().parse()?;

                // then recalculate merit rank for updated graph
                meritrank.add_edge(source, destination, weight);
            } else {
                let mut graph = MyGraph::new();
                graph.add_node(0.into());
                graph.add_node(1.into());
                graph.add_node(2.into());

                let weights: Vec<f64> = record
                    .iter()
                    .skip(3)
                    .take(6)
                    .map(|field| field.trim().parse())
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()?;

                graph.add_edge(0.into(), 1.into(), weights[0]);
                graph.add_edge(0.into(), 2.into(), weights[1]);
                graph.add_edge(1.into(), 0.into(), weights[2]);
                graph.add_edge(1.into(), 2.into(), weights[3]);
                graph.add_edge(2.into(), 0.into(), weights[4]);
                graph.add_edge(2.into(), 1.into(), weights[5]);

                meritrank_opt = Some(MeritRank::new(graph)?);
                meritrank_opt.as_mut().unwrap().calculate(0.into(), 1000)?;
            }

            let rating: HashMap<NodeId, f64> = meritrank_opt
                .as_ref()
                .ok_or("MeritRank not initialized")?
                .get_ranks(0.into(), None)
                .unwrap_or_default()
                .into_iter()
                .collect();

            let rank0: f64 = record[9].trim().parse()?;
            let rank1: f64 = record[10].trim().parse()?;
            let rank2: f64 = record[11].trim().parse()?;

            eprintln!(
                "Rating for node 0: {}, from dump: {}",
                rating.get(&0.into()).unwrap_or(&0.0),
                rank0
            );
            eprintln!(
                "Rating for node 1: {}, from dump: {}",
                rating.get(&1.into()).unwrap_or(&0.0),
                rank1
            );
            eprintln!(
                "Rating for node 2: {}, from dump: {}",
                rating.get(&2.into()).unwrap_or(&0.0),
                rank2
            );
        }

        Ok(())
    }


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
                meritrank.add_edge(source, destination, weight);
            } else {
                let mut graph = MyGraph::new();
                let node_ids: Vec<NodeId> = vec![0.into(), 1.into(), 2.into(), 3.into()];

                for &node_id in &node_ids {
                    graph.add_node(node_id.into());
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
                    graph.add_edge(src.into(), dest.into(), weights[i]);
                }

                meritrank_opt = Some(MeritRank::new(graph)?);
                meritrank_opt.as_mut().unwrap().calculate(0.into(), 500)?;
            }

            let rating: Vec<(NodeId, f64)> = meritrank_opt
                .as_ref()
                .ok_or("MeritRank not initialized")?
                .get_ranks(0.into(), None)
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
                    rating.get(i).unwrap_or(&(i.into(), 0.0)).1,
                    expected_rank
                );
            }
        }

        Ok(())
    }
}
