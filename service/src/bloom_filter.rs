use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use crate::log::*;

pub fn bloom_filter_bits(
  size: usize,
  num_hashes: usize,
  id: usize,
) -> Vec<u64> {
  let mut v: Vec<u64> = vec![0; size];

  for n in 1..=num_hashes {
    let mut h = DefaultHasher::new();
    h.write_u16(n as u16);
    h.write_u64(id as u64);
    let hash = h.finish();

    let u64_index = ((hash / 64u64) as usize) % size;
    let bit_index = hash % 64u64;

    v[u64_index] |= 1u64 << bit_index;
  }

  v
}

pub fn bloom_filter_add(
  mask: &mut [u64],
  bits: &[u64],
) {
  if mask.len() != bits.len() {
    log_error!("Invalid arguments");
    return;
  }

  for i in 0..mask.len() {
    mask[i] |= bits[i];
  }
}

pub fn bloom_filter_contains(
  mask: &[u64],
  bits: &[u64],
) -> bool {
  if mask.len() != bits.len() {
    log_error!("Invalid arguments");
    return false;
  }

  for i in 0..mask.len() {
    if (mask[i] & bits[i]) != bits[i] {
      return false;
    }
  }

  true
}
