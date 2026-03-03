use serde::Deserialize;
use std::path::Path;

fn put_edge_(
  src: &str,
  dst: &str,
  weight: f64,
) {
  let _ =
    crate::mr_put_edge(Some(src), Some(dst), Some(weight), None, Some(-1))
      .unwrap();
}

#[derive(Debug, Deserialize)]
struct Edge {
  src:    String,
  dst:    String,
  weight: f64,
}

pub fn reset_sync() {
  crate::mr_reset().unwrap();
  crate::mr_sync(Some(1000)).unwrap();
}

pub fn put_testing_edges() {
  let path = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("testdata")
    .join("edges.csv");
  let mut reader = csv::Reader::from_path(&path).unwrap();
  for result in reader.deserialize() {
    let edge: Edge = result.unwrap();
    put_edge_(&edge.src, &edge.dst, edge.weight);
  }
}
