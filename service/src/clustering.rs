use crate::log::*;
use crate::log_error;
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::sync::atomic::Ordering;

pub struct KMeans {
    data: Vec<(usize, f64)>,
    k: usize,
    max_iterations: usize,
    tolerance: f64,
    rng: ThreadRng,
}

impl KMeans {
    pub fn new(data: Vec<(usize, f64)>, k: usize, max_iterations: usize, tolerance: f64) -> Self {
        Self {
            data,
            k,
            max_iterations,
            tolerance,
            rng: thread_rng(),
        }
    }

    pub fn distance(a: &(usize, f64), b: &(usize, f64)) -> f64 {
        let dx = (a.0 as isize - b.0 as isize) as f64;
        let dy = a.1 - b.1;
        (dx * dx + dy * dy).sqrt() + 1e-9
    }

    pub fn run(&mut self) -> Vec<f64> {
        if self.data.is_empty() {
            log_error!("Data points are empty");
            return vec![];
        }

        let mut centroids = self.kmeans_plus_plus_initialization();
        let mut assignments = vec![0.0; self.data.len()];
        let mut changed = true;

        for _ in 0..self.max_iterations {
            if !changed {
                break;
            }
            changed = false;

            for (i, &point) in self.data.iter().enumerate() {
                let (closest, _) = centroids
                    .iter()
                    .enumerate()
                    .map(|(j, &centroid)| {
                        (
                            j as f64,
                            Self::distance(&(point.0, point.1), &(centroid.0 as usize, centroid.1)),
                        )
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .unwrap();

                if assignments[i] != closest {
                    assignments[i] = closest;
                    changed = true;
                }
            }

            let mut new_centroids = vec![(0.0, 0.0); self.k];
            let mut counts = vec![0; self.k];

            for (i, &assignment) in assignments.iter().enumerate() {
                let cluster_idx = assignment as usize;
                new_centroids[cluster_idx].0 += self.data[i].0 as f64;
                new_centroids[cluster_idx].1 += self.data[i].1;
                counts[cluster_idx] += 1;
            }

            let mut total_shift = 0.0;
            for (i, centroid) in new_centroids.iter_mut().enumerate() {
                if counts[i] > 0 {
                    let new_x = centroid.0 / counts[i] as f64;
                    let new_y = centroid.1 / counts[i] as f64;
                    total_shift +=
                        (new_x - centroids[i].0).powi(2) + (new_y - centroids[i].1).powi(2);
                    centroids[i] = (new_x, new_y);
                } else {
                    *centroid = (
                        self.data[self.rng.gen_range(0..self.data.len())].0 as f64,
                        self.data[self.rng.gen_range(0..self.data.len())].1,
                    );
                }
            }

            if total_shift < self.tolerance {
                break;
            }
        }

        assignments
    }

    fn kmeans_plus_plus_initialization(&mut self) -> Vec<(f64, f64)> {
        let mut centroids = Vec::with_capacity(self.k);
        let mut distances: Vec<f64> = vec![f64::MAX; self.data.len()];

        centroids.push((
            self.data[self.rng.gen_range(0..self.data.len())].0 as f64,
            self.data[self.rng.gen_range(0..self.data.len())].1,
        ));

        for _ in 1..self.k {
            for (i, &point) in self.data.iter().enumerate() {
                let dist = Self::distance(
                    &(point.0, point.1),
                    &(
                        centroids.last().unwrap().0 as usize,
                        centroids.last().unwrap().1,
                    ),
                );
                distances[i] = distances[i].min(dist);
            }

            let sum: f64 = distances.iter().sum();
            let target = self.rng.gen_range(0.0..sum);
            let mut cumulative_sum = 0.0;

            for (i, &d) in distances.iter().enumerate() {
                cumulative_sum += d;
                if cumulative_sum >= target {
                    centroids.push((self.data[i].0 as f64, self.data[i].1));
                    break;
                }
            }
        }
        centroids
    }
}
