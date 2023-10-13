use crate::SmartError;

/// ADF T Statistic
/// Calculates the T-Statistic for ADF
pub fn calculate_adf_test_statistic(residuals: Vec<f64>, residuals_diff: Vec<f64>) -> Result<f64, SmartError> {

  let x: &[f64] = &residuals[..residuals.len() - 1];
  let y: &[f64] = &residuals_diff[..];

  let x_bar: f64 = x.iter().sum::<f64>() / x.len() as f64;
  let y_bar: f64 = y.iter().sum::<f64>() / y.len() as f64;

  let beta_hat_num: f64 = x.iter().zip(y.iter()).map(|(&x, &y)| (x - x_bar) * (y - y_bar)).sum::<f64>();
  let beta_hat_denom: f64 = x.iter().map(|&x| (x - x_bar).powi(2)).sum::<f64>();
  let beta_hat: f64 = beta_hat_num / beta_hat_denom;

  let alpha_hat: f64 = y_bar - beta_hat * x_bar;

  let y_hat: Vec<f64> = x.iter().map(|&x| alpha_hat + beta_hat * x).collect();
  let sse: f64 = y.iter().zip(y_hat.iter()).map(|(&y, &yh)| (y - yh).powi(2)).sum::<f64>();

  let se_beta_hat_denom: f64 = (y.len() - 2) as f64 * x.iter().map(|&x| (x - x_bar).powi(2)).sum::<f64>();
  let se_beta_hat: f64 = (sse / se_beta_hat_denom).sqrt();
  let adf_stat: f64 = beta_hat / se_beta_hat;
  Ok(adf_stat)
}

/// Simple Kalman Filter
/// Returns kalman filter for multiple series
pub fn simple_kalman_filter(series_0: &Vec<f64>, series_1: &Vec<f64>) -> Vec<f64> {

  assert_eq!(series_0.len(), series_1.len(), "Series lengths do not match!");

  let mut hedge_ratios = Vec::new();

  let a: f64 = 1.0;
  let b: f64 = 1.0;
  let q: f64 = 0.0001;
  let r: f64 = 1.0;
  let mut p: f64 = 1.0;
  let mut x: f64 = 0.0; // state (estimated as the hedge ratio)

  for i in 0..series_0.len() {
    let y: f64 = series_0[i] / series_1[i]; // observation

    // Prediction
    let x_hat = a * x; // hedge ratio prediction
    p = a * p * a + q;

    // Update
    let k: f64 = p * b / (b * p * b + r);
    x = x_hat + k * (y - b * x_hat); // update hedge ratio
    p = (1.0 - k * b) * p; 

    hedge_ratios.push(x);
  }

  hedge_ratios
}
