#[cfg.test]
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
