use crate::aug_multi_graph::*;

use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use tar::Archive;

#[derive(Debug, Deserialize)]
struct Edge {
  context:   String,
  src:       String,
  dst:       String,
  weight:    f64,
  magnitude: i64,
}

fn read_csv_from_tar_gz(
  tar_gz_path: &str,
  target_file: &str,
) -> Vec<Edge> {
  let file = File::open(tar_gz_path).unwrap();
  let decompressor = GzDecoder::new(BufReader::new(file));
  let mut archive = Archive::new(decompressor);

  for entry in archive.entries().unwrap() {
    let file = entry.unwrap();
    if file.path().unwrap().to_str() == Some(target_file) {
      let reader = csv::Reader::from_reader(file);
      return reader.into_deserialize().map(|r| r.unwrap()).collect();
    }
  }

  panic!("File '{}' not found in archive", target_file);
}

fn put_testing_edges(
  graph: &mut AugMultiGraph,
  file_name: &str,
) {
  for edge in read_csv_from_tar_gz("src/edges.tar.gz", file_name) {
    graph.write_put_edge(
      &edge.context,
      &edge.src,
      &edge.dst,
      edge.weight,
      edge.magnitude,
    );
  }
}

pub fn put_testing_edges_0(graph: &mut AugMultiGraph) {
  put_testing_edges(graph, "edges0.csv");
}

pub fn put_testing_edges_1(graph: &mut AugMultiGraph) {
  put_testing_edges(graph, "edges1.csv");
}

pub fn put_testing_edges_2(graph: &mut AugMultiGraph) {
  put_testing_edges(graph, "edges2.csv");
}

#[cfg(not(debug_assertions))]
pub fn put_testing_edges_3(graph: &mut AugMultiGraph) {
  put_testing_edges(graph, "edges3.csv");
}

pub fn put_testing_edges_4(graph: &mut AugMultiGraph) {
  put_testing_edges(graph, "edges4.csv");
}
