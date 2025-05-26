use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[cfg(test)]
#[macro_export]
macro_rules! assert_approx_eq {
    ($a:expr, $b:expr, $rel_tol:expr) => {
        {
            let diff = ($a - $b).abs();
            let max_ab = $a.abs().max($b.abs());
            assert!(
                diff <= max_ab * $rel_tol,
                "assertion failed: `(left ≈ right)`\n  left: `{}`, right: `{}`, diff: `{}`, max_ab: `{}`, relative tolerance: `{}`",
                $a, $b, diff, max_ab, $rel_tol
            );
        }
    };
}

#[derive(Debug, Deserialize)]
pub struct Edge {
  pub src:    usize,
  pub dst:    usize,
  pub weight: f64,
}

pub fn read_edges_from_csv(file_path: &str) -> Vec<Edge> {
  let file = File::open(file_path).unwrap();
  let reader = BufReader::new(file);
  csv::ReaderBuilder::new()
    .trim(csv::Trim::All)
    .from_reader(reader)
    .deserialize()
    .collect::<Result<Vec<Edge>, _>>()
    .expect("Failed to read CSV data")
}
