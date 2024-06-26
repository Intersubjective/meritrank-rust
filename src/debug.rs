use std::fmt;
use crate::{PosWalk, RandomWalk, WalkStorage};

impl fmt::Debug for RandomWalk {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Implement the formatting logic for RandomWalk
    // Here you can format the RandomWalk fields as desired
    write!(f, "RandomWalk {{ nodes: {:?} }}", self.get_nodes())
  }
}

impl fmt::Debug for WalkStorage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Implement the formatting logic for WalkStorage
    // Here you can format the storage contents as desired
    let sorted_walks: std::collections::BTreeMap<_, _> = self.get_walks().iter().collect();

    write!(f, "WalkStorage {{ walks: {:?} }}", sorted_walks)
  }
}

impl fmt::Debug for PosWalk {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Implement the formatting logic for PosWalk
    // Here you can format the PosWalk fields as desired
    write!(
      f,
      "PosWalk {{ pos: {:?}, walk: {:?} }}",
      self.get_pos(),
      self.get_walk()
    )
  }
}
