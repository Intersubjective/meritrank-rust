use std::fmt;
use crate::{RandomWalk, WalkStorage};

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

    write!(f, "WalkStorage {{ walks: {:?} }}", self.get_walks())
  }
}

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
