use std::error::Error;

pub fn ctx(c: Option<&str>) -> &str {
  c.unwrap_or("")
}

pub fn require<T>(
  v: Option<T>,
  name: &str,
) -> Result<T, Box<dyn Error + 'static>> {
  v.ok_or_else(|| format!("{name} should not be null").into())
}

pub fn timeout_u64(t: Option<i64>) -> Option<u64> {
  t.map(|x| x as u64)
}

pub fn validate_bounds(
  lt: Option<f64>,
  lte: Option<f64>,
  gt: Option<f64>,
  gte: Option<f64>,
) -> Result<(f64, bool, f64, bool), Box<dyn Error + 'static>> {
  if lt.is_some() && lte.is_some() {
    return Err("either lt or lte is allowed!".into());
  }
  if gt.is_some() && gte.is_some() {
    return Err("either gt or gte is allowed!".into());
  }
  Ok((
    lt.unwrap_or(lte.unwrap_or(i32::MAX.into())),
    lte.is_some(),
    gt.unwrap_or(gte.unwrap_or(i32::MIN.into())),
    gte.is_some(),
  ))
}
