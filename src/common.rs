use crate::constants::EPSILON;

pub fn sign<T>(x: T) -> i32
where
    T: Into<f64> + Copy,
{
    let x: f64 = x.into();

    if x >= EPSILON {
        return 1;
    } else if x <= -EPSILON {
        return -1;
    }
    return 0;
}
