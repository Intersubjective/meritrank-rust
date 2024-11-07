use crate::log::*;
use crate::log_error;
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::sync::atomic::Ordering;

pub struct KMeans {
  data: Vec<f64>, 
  k: usize,
  max_iterations: usize,
  tolerance: f64,
  rng: ThreadRng,
}

impl KMeans {
  pub fn new(
    data: Vec<f64>,
    k: usize,
    max_iterations: usize,
    tolerance: f64,
  ) -> Self {
    Self {
      data,
      k,
      max_iterations,
      tolerance,
      rng: thread_rng(),
    }
  }

  pub fn distance(
    a: f64,
    b: f64,
  ) -> f64 {
    (a - b).abs()
  }

  pub fn run(&mut self) -> Vec<f64> {
    if self.data.is_empty() {
      log_error!("Erorr. Data is empty");
      return vec![];
    }

    let mut centroids = self.kmeans_plus_plus_initialization();
    let mut assignments = vec![0.0; self.data.len()];
    let mut changed = true;

    for _ in 0..self.max_iterations+100 {
      if !changed {
        break;
      }
      changed = false;

      for (i, &point) in self.data.iter().enumerate() {
        let (closest, _) = centroids
          .iter()
          .enumerate()
          .map(|(j, &centroid)| (j as f64, Self::distance(point, centroid)))
          .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
          .unwrap();

        if assignments[i] != closest {
          assignments[i] = closest;
          changed = true;
        }
      }

      let mut new_centroids = vec![0.0; self.k];
      let mut counts = vec![0; self.k];

      for (i, &assignment) in assignments.iter().enumerate() {
        let cluster_idx = assignment as usize;
        new_centroids[cluster_idx] += self.data[i];
        counts[cluster_idx] += 1;
      }

      let mut total_shift = 0.0;
      for (i, centroid) in new_centroids.iter_mut().enumerate() {
        if counts[i] > 0 {
          let new_centroid = *centroid / counts[i] as f64;
          total_shift += (new_centroid - centroids[i]).powi(2);
          centroids[i] = new_centroid;
        } else {
          *centroid = self.data[self.rng.gen_range(0..self.data.len())];
        }
      }

      if total_shift < self.tolerance {
        break;
      }
    }

    assignments
  }

  fn kmeans_plus_plus_initialization(&mut self) -> Vec<f64> {
    let mut centroids = Vec::with_capacity(self.k);
    let mut distances: Vec<f64> = vec![f64::MAX; self.data.len()];
    centroids.push(self.data[self.rng.gen_range(0..self.data.len())]);

    for _ in 1..self.k {
      for (i, &point) in self.data.iter().enumerate() {
        let dist = Self::distance(point, *centroids.last().unwrap());
        distances[i] = distances[i].min(dist);
      }

      let sum: f64 = distances.iter().sum();
      let target = self.rng.gen_range(0.0..sum);
      let mut cumulative_sum = 0.0;

      for (i, &d) in distances.iter().enumerate() {
        cumulative_sum += d;
        if cumulative_sum >= target {
          centroids.push(self.data[i]);
          break;
        }
      }
    }
    centroids
  }
}
