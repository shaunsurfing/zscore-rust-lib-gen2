use crate::SmartError;
use crate::backtest::utils::log_returns;
use super::models::Relationship;

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

/// Covar Calculation
/// Required for beta calculation
pub fn calculate_covariance(log_returns_x: &[f64], log_returns_y: &[f64]) -> Result<f64, SmartError> {
  if log_returns_x.len() != log_returns_y.len() {
      return Err(SmartError::RuntimeCheck("Datasets x and y must have the same length".to_string()));
  }

  let n = log_returns_x.len() as f64;
  let mean_x: f64 = log_returns_x.iter().sum::<f64>() / n;
  let mean_y: f64 = log_returns_y.iter().sum::<f64>() / n;

  let covariance: f64 = log_returns_x.iter()
    .zip(log_returns_y.iter())
    .map(|(&xi, &yi)| (xi - mean_x) * (yi - mean_y))
    .sum::<f64>() / (n - 1.0);

  Ok(covariance)
}

/// Var Calculation
/// Required for beta calculation
pub fn calculate_variance(log_returns: &[f64]) -> f64 {
  let n = log_returns.len() as f64;
  let mean: f64 = log_returns.iter().sum::<f64>() / n;
  let variance: f64 = log_returns.iter()
    .map(|&x| (x - mean).powi(2))
    .sum::<f64>() / (n - 1.0);
  variance
}

/// Historical Volatility
/// Calculates historical annual volatility for a given asset
pub fn calculate_historical_annual_volatility(log_returns: &[f64], trading_days: usize) -> f64 {
  // Assuming log_returns is already a series of log returns
  let mean_return: f64 = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
  let variance: f64 = log_returns.iter()
      .map(|&x| (x - mean_return).powi(2))
      .sum::<f64>() / (log_returns.len() - 1) as f64;
  let daily_volatility: f64 = variance.sqrt();

  // Annualize the daily volatility
  daily_volatility * (trading_days as f64).sqrt()
}

/// Beta Coeff Calculation
/// Used to determine the beta coeff for two assets in respect to one another
pub fn calculate_beta_coefficient(log_returns_x: &[f64], log_returns_y: &[f64]) -> Result<f64, SmartError> {
  let covariance = calculate_covariance(log_returns_x, log_returns_y)?;
  let market_variance = calculate_variance(log_returns_y);
  Ok(covariance / market_variance)
}

/// Volatility Ratio
/// Used to determine the volatility ratio of two assets
pub fn volatility_ratio(log_returns_y: &Vec<f64>, log_returns_x: &Vec<f64>, trading_days: usize) -> f64 {
  let y_volatility = calculate_historical_annual_volatility(log_returns_y, trading_days);
  let x_volatility = calculate_historical_annual_volatility(log_returns_x, trading_days);
  x_volatility / y_volatility
}

/// Calculate Relationship
/// Relationship workings for prices
pub fn calculate_relaitonship(y: &[f64], x: &[f64], trading_days: usize) -> Result<Relationship, SmartError> {
  let log_returns_y = log_returns(&y.to_vec(), false);
  let log_returns_x = log_returns(&x.to_vec(), false);
  let beta_x_to_y: f64 = calculate_beta_coefficient(&log_returns_x, &log_returns_y).map_err(|e| SmartError::RuntimeCheck(e.to_string()))?;
  let beta_y_to_x: f64 = calculate_beta_coefficient(&log_returns_y, &log_returns_x).map_err(|e| SmartError::RuntimeCheck(e.to_string()))?;
  let annual_vol_y: f64 = calculate_historical_annual_volatility(&log_returns_y, trading_days);
  let annual_vol_x: f64 = calculate_historical_annual_volatility(&log_returns_x, trading_days);
  let vol_ratio_x_to_y: f64 = volatility_ratio(&log_returns_y, &log_returns_x, trading_days);
  let relationship: Relationship = Relationship { beta_x_to_y, beta_y_to_x, annual_vol_y, annual_vol_x, vol_ratio_x_to_y };
  Ok(relationship)
}

