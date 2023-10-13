/// Number Round
/// Round number to n decimal places
pub fn round_float(num: f64, decimals: i32) -> f64 {
  let multiplier: f64 = 10f64.powi(decimals);
  (num * multiplier).round() / multiplier
}

/// Log Returns
/// Takes in a series and calculates log returns
pub fn log_returns(series: &Vec<f64>, is_buffer: bool) -> Vec<f64> {
  let mut log_rets: Vec<f64> = match is_buffer {
      true => vec![0.0],
      false => vec![],
  };

  let s: Vec<f64> = series.windows(2).map(|w| {
      (w[1] / w[0]).ln()
  }).collect();

  log_rets.extend(s);
  
  log_rets
}

/// Convert Log to Simple Returns
/// Converts log returns to simple returns
pub fn log_to_simple_returns(log_rets: &Vec<f64>) -> Vec<f64> {
  log_rets.iter().map(|&r| f64::exp(r) - 1.0).collect()
}
