use indexmap::IndexMap;
use rand::prelude::*;
use std::collections::HashMap;
use std::f64;

const EPS: f64 = 0.1; // convergence threshold

pub type PsNodeId = usize;
type Weight = f64;
pub type PushsumAdjMap = IndexMap<PsNodeId, PsNode>;

#[derive(Clone)]
pub struct PsNode {
  pub(crate) s:     Vec<Weight>, // per-choice mass
  w:                Weight,      // scalar weight
  pub(crate) edges: HashMap<PsNodeId, Weight>, // (neighbour id, probability)
}

impl PsNode {
  fn estimate(&self) -> Vec<Weight> {
    self.s.iter().map(|&v| v / self.w).collect()
  }
}

fn fire(
  nodes: &mut PushsumAdjMap,
  src_node_ind: usize,
) {
  if let Some((node_id, node)) = nodes.get_index(src_node_ind) {
    let mut updates: Vec<(PsNodeId, Weight, Vec<Weight>)> = Vec::new();
    let orig_s = node.s.clone();
    let orig_w = node.w;

    for (&dst_node, &prob) in &node.edges {
      if prob == 0.0 {
        continue;
      }

      let mut delta_s = vec![0.0; orig_s.len()];
      for (choice, &mass) in orig_s.iter().enumerate() {
        delta_s[choice] = mass * prob;
      }
      let delta_w = orig_w * prob;

      updates.push((*node_id, -delta_w, delta_s.iter().map(|&x| -x).collect()));
      updates.push((dst_node, delta_w, delta_s));
    }

    for (id, delta_w, delta_s) in updates {
      if let Some(node) = nodes.get_mut(&id) {
        node.w += delta_w;
        for (i, delta) in delta_s.iter().enumerate() {
          node.s[i] += delta;
        }
      }
    }
  }
}
fn fire_random(
  nodes: &mut PushsumAdjMap,
  amount: f64,
  rng: &mut ThreadRng,
) {
  if nodes.len() < 2 {
    return; // Not enough nodes for random transfer
  }

  let node_count = nodes.len();
  let src_index = rng.gen_range(0..node_count);
  let mut dst_index;
  loop {
    dst_index = rng.gen_range(0..node_count);
    if dst_index != src_index {
      break;
    }
  }

  let (src_id, dst_id) = {
    let src_id = *nodes.get_index(src_index).unwrap().0;
    let dst_id = *nodes.get_index(dst_index).unwrap().0;
    (src_id, dst_id)
  };

  let (delta_s, delta_w) = {
    let src_node = nodes.get(&src_id).unwrap();
    let mut delta_s = vec![0.0; src_node.s.len()];
    for (i, &s) in src_node.s.iter().enumerate() {
      delta_s[i] = s * amount;
    }
    let delta_w = src_node.w * amount;
    (delta_s, delta_w)
  };

  // Update source node
  if let Some(node) = nodes.get_mut(&src_id) {
    node.w -= delta_w;
    for (i, delta) in delta_s.iter().enumerate() {
      node.s[i] -= delta;
    }
  }

  // Update destination node
  if let Some(node) = nodes.get_mut(&dst_id) {
    node.w += delta_w;
    for (i, delta) in delta_s.iter().enumerate() {
      node.s[i] += delta;
    }
  }
}
fn l1_distance(
  a: &[Weight],
  b: &[Weight],
) -> Weight {
  a.iter().zip(b).map(|(&x, &y)| (x - y).abs()).sum()
}

pub fn calculate_consensus(
  mut nodes: PushsumAdjMap,
  num_steps: usize,
) -> Option<Vec<Weight>> {
  let mut rng = thread_rng();
  let node_count = nodes.len();
  let m = nodes.values().next().map(|n| n.s.len()).unwrap_or(0);

  for step in 1..=num_steps {
    let i = rng.gen_range(0..node_count);
    fire(&mut nodes, i);
    // We transfer a tiny amount of mass to other nodes to
    // breach the sinks and add a bit of connectivity to everyone
    const TINY_AMOUNT: f64 = 1e-2; // Adjust this value as needed
    fire_random(&mut nodes, TINY_AMOUNT, &mut rng);

    if step % node_count == 0 {
      let mean = calculate_mean(&nodes, m);
      let spread = calculate_spread(&nodes, &mean);
      println!("Consensus distribution: {:?} {:?}", mean, spread);

      if spread < EPS {
        println!("Converged at step {step}");
        println!("Consensus distribution: {:?}", mean);
        return Some(mean);
      }

      if step == num_steps {
        println!(
                    "Reached {num_steps} steps without full convergence; variance={spread}"
                );
        return None;
      }
    }
  }
  None
}

fn calculate_mean(
  nodes: &PushsumAdjMap,
  m: usize,
) -> Vec<Weight> {
  let mut mean = vec![0.0; m];
  for node in nodes.values() {
    for (k, &p) in node.estimate().iter().enumerate() {
      mean[k] += p;
    }
  }
  for v in &mut mean {
    *v /= nodes.len() as Weight;
  }
  mean
}

fn calculate_spread(
  nodes: &PushsumAdjMap,
  mean: &[Weight],
) -> Weight {
  nodes
    .values()
    .map(|n| l1_distance(&n.estimate(), mean))
    .fold(0.0_f64, f64::max)
}

#[cfg(test)]
impl PsNode {
  pub fn new_hot(
    weight_choices: Vec<Weight>,
    edges: HashMap<PsNodeId, Weight>,
  ) -> Self {
    let total_weight: Weight = weight_choices.iter().sum();

    Self {
      s: weight_choices,
      w: total_weight,
      edges,
    }
  }
}

#[test]
fn test_pushsum() {
  const MAX_STEPS: usize = 100;

  let rows: Vec<HashMap<PsNodeId, Weight>> = vec![
    [(0, 0.15), (1, 0.25), (2, 0.20), (3, 0.20), (4, 0.20)]
      .iter()
      .cloned()
      .collect(),
    [(1, 0.15), (0, 0.25), (2, 0.20), (3, 0.20), (4, 0.20)]
      .iter()
      .cloned()
      .collect(),
    [(2, 0.15), (0, 0.25), (1, 0.20), (3, 0.20), (4, 0.20)]
      .iter()
      .cloned()
      .collect(),
    [(3, 0.15), (0, 0.25), (1, 0.20), (2, 0.20), (4, 0.20)]
      .iter()
      .cloned()
      .collect(),
    [(4, 0.15), (0, 0.25), (1, 0.20), (2, 0.20), (3, 0.20)]
      .iter()
      .cloned()
      .collect(),
  ];

  let init_choice = vec![0_usize, 1, 2, 0, 1];

  let m_arg = std::env::args()
    .nth(1)
    .and_then(|s| s.parse::<usize>().ok());
  let m =
    m_arg.unwrap_or_else(|| init_choice.iter().copied().max().unwrap_or(0) + 1);

  if m == 0 {
    eprintln!("Number of choices must be > 0");
    return;
  }

  let nodes: PushsumAdjMap = rows
    .into_iter()
    .enumerate()
    .map(|(idx, edges)| {
      let mut weight_choices = vec![0.0; m];
      weight_choices[init_choice[idx]] = 1.0;
      (idx, PsNode::new_hot(weight_choices, edges))
    })
    .collect();

  let result = calculate_consensus(nodes, MAX_STEPS);
  if let Some(mean) = result {
    println!("Consensus distribution: {:?}", mean);
  } else {
    println!("No consensus reached");
  }
}
