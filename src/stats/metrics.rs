use crate::SmartError;
use super::mackinnon::{critical_values_mackinnon_cointegration, p_value_mackinnon_cointegration};
use super::models::Coint;
use super::regression::simple_linear_regression;
use super::statistics::{simple_kalman_filter, calculate_adf_test_statistic};

/// Half Life Mean Reversion
/// Time it takes for process to revert to half its initial deviation
pub fn half_life_mean_reversion(series: &Vec<f64>) -> Result<f64, SmartError> {
  if series.len() <= 1 {
      return Err(SmartError::RuntimeCheck("Series length must be greater than 1.".to_string()));
  }

  let difference: Vec<f64> = series.windows(2).map(|x| x[1] - x[0]).collect();
  let lagged_series: Vec<f64> = series[..(series.len() - 1)].to_vec();

  let ((_, beta_1), _residuals) = simple_linear_regression(&lagged_series, &difference)?;
  
  // check if beta_1 is zero to prevent division by zero error
  if beta_1.abs() < std::f64::EPSILON {
      return Err(SmartError::RuntimeCheck("Cannot calculate half life. Beta_1 value is too close to zero.".to_string()));
  }

  let half_life: f64 = -f64::ln(2.0) / beta_1;
  
  Ok(half_life)
}

/// Calculate Static Hedge Ratio
pub fn intercept_hedge_ratio_static(series_0: &Vec<f64>, series_1: &Vec<f64>) -> Result<(f64, f64), SmartError> {
  let ((intercept, hedge_ratio), _) = simple_linear_regression(&series_1, &series_0)?;
  Ok((intercept, hedge_ratio))
}

/// Spread With Hedge Ratio
/// Calculates the spread for two series and given Hedge Ratio
pub fn spread_static_std(series_0: &Vec<f64>, series_1: &Vec<f64>) -> Result<(Vec<f64>, f64), SmartError> {

  // Guard: Ensure length matches
  if series_0.len() != series_1.len() {
    return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Input vectors have different sizes")));
  }

  // Calculate intercept and hedge ratio (slope)
  let (intercept, hedge_ratio) = intercept_hedge_ratio_static(&series_0, &series_1)?;

  // Compute spread - [series_1 - series_0 * hedge_ratio]
  let spread: Vec<f64> = series_0.iter().zip(series_1.iter()).map(|(&x, &y)| x - (hedge_ratio * y) - intercept).collect();

  // Return result
  Ok((spread, hedge_ratio))
}


/// Spread With Dynamic Hedge Ratio
/// Calculates the spread for two series and given a Dynamic Hedge Ratio Vector
/// Use if you already know the dynamic hedge ratio
pub fn spread_dynamic_kalman(series_0: &Vec<f64>, series_1: &Vec<f64>) -> Result<(Vec<f64>, f64), SmartError> {

  // Guard: Ensure length matches
  if series_0.len() != series_1.len() {
    return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Input vectors have different sizes")));
  }

  // Extract Hedge Ratio
  let dyn_hedge_ratio: Vec<f64> = simple_kalman_filter(series_0, series_1);

  // Guard: Ensure Dynamic Hedge Ratio length matches
  if series_0.len() != dyn_hedge_ratio.len() {
    return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Hedge Ratio vector should match length of time series")));
  }
  
  // Compute dynamic spread - [series_0 - series_1 * hedge_ratio]
  let dyn_spread: Vec<f64> = series_0.iter().zip(series_1.iter()).zip(dyn_hedge_ratio.iter())
    .map(|((&x, &y), &hedge_ratio_i)| x - hedge_ratio_i * y)
    .collect();

  // Extract last hedge_ratio value
  let hedge_ratio: f64 = dyn_hedge_ratio.iter().last().unwrap_or(&0.0).clone();

  // Return result
  Ok((dyn_spread, hedge_ratio))
}

/// ZScore
/// Calculates the ZScore given a spread
pub fn rolling_zscore(series: &Vec<f64>, window: usize) -> Result<Vec<f64>, SmartError> {
  let mut z_scores: Vec<f64> = vec![0.0; window]; // Padding with 0.0 for the first (window) elements

  // Guard: Ensure correct window size
  if window > series.len() {
    return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Window size is greater than vector length")));
  }

  // Calculate z-scores for each window
  for i in window..series.len() {
    let window_data: &[f64] = &series[i-window..i];
    let mean: f64 = window_data.iter().sum::<f64>() / window_data.len() as f64;
    let var: f64 = window_data.iter().map(|&val| (val - mean).powi(2)).sum::<f64>() / (window_data.len()-1) as f64;
    let std_dev: f64 = var.sqrt();
    if std_dev == 0.0 {
        return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Standard deviation is zero")));
    }
    let z_score = (series[i] - mean) / std_dev;
    z_scores.push(z_score);
  }
  Ok(z_scores)
}

/// Correlation
/// Using Pearsons Correlation Coefficient
pub fn pearson_correlation_coefficient(x: &Vec<f64>, y: &Vec<f64>) -> Result<f64, SmartError> {
  if x.len() != y.len() {
    return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Input vectors have different sizes")));
  }

  let mean_x: f64 = x.iter().sum::<f64>() / x.len() as f64;
  let mean_y: f64 = y.iter().sum::<f64>() / y.len() as f64;
  
  let covariance: f64 = x.iter().zip(y.iter())
    .map(|(x_i, y_i)| (x_i - mean_x) * (y_i - mean_y))
    .sum::<f64>() / (x.len() - 1) as f64;

  let std_dev_x: f64 = (x.iter().map(|x_i| (x_i - mean_x).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
  let std_dev_y: f64 = (y.iter().map(|y_i| (y_i - mean_y).powi(2)).sum::<f64>() / y.len() as f64).sqrt();

  let corr: f64 = covariance / (std_dev_x * std_dev_y);

  Ok(corr)
}


/// Cointegration Test Based on Engle Granger 2-Step Approach
/// Provides test statistic, critical values, pvalue and also hedge ratio
pub fn cointegration_test_eg(series_0: &Vec<f64>, series_1: &Vec<f64>) -> Result<Coint, SmartError> {
    
  let (_, residuals) = simple_linear_regression(series_0, series_1)?;

  let residuals_diff: Vec<f64> = residuals.windows(2).map(|w| w[1] - w[0]).collect();

  let t_stat: f64 = calculate_adf_test_statistic(residuals, residuals_diff)?;

  let (cv_1pct, cv_5pct, cv_10pct) = critical_values_mackinnon_cointegration();

  let adf_p_value: f64 = p_value_mackinnon_cointegration(t_stat);

  let is_cointegrated: bool = t_stat < cv_5pct as f64 && adf_p_value < 0.05;
  
  let coint: Coint = Coint {
    is_coint: is_cointegrated,
    test_statistic: t_stat,
    critical_values: (cv_1pct, cv_5pct, cv_10pct),
    p_value: adf_p_value
  };

  Ok(coint)
}

/// Rolling Correlation
/// Calculates the Rolling Correlation for a given window
pub fn rolling_correlation(series_1: &Vec<f64>, series_2: &Vec<f64>, window: usize) -> Result<Vec<f64>, SmartError> {
  let mut correlations: Vec<f64> = vec![0.0; window]; // Padding with 0.0 for the first (window) elements

  // Guard: Ensure series length matches
  if series_1.len() != series_2.len() {
    return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Input vectors have different sizes")));
  }

  // Guard: Ensure correct window size
  if window > series_1.len() {
    return Err(SmartError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Window size is greater than vector length")));
  }

  // Calculate rolling cointegration for each window
  for i in window..series_1.len() {
    let series_1_i: &Vec<f64> = &series_1[i-window..i].to_vec();
    let series_2_i: &Vec<f64> = &series_2[i-window..i].to_vec();
    let corr: f64 = pearson_correlation_coefficient(series_1_i, series_2_i)?;
    correlations.push(corr);
  }
  Ok(correlations)
}

/// Rolling Cointegration
/// Calculates the Rolling Cointegration in terms of test-stat minus c-value for a given window
pub fn rolling_cointegration(series_1: &Vec<f64>, series_2: &Vec<f64>, window: usize) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
  let mut t_distances: Vec<f64> = vec![0.0; window]; // Padding with 0.0 for the first (window) elements

  // Guard: Ensure series length matches
  if series_1.len() != series_2.len() {
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Input vectors have different sizes")));
  }

  // Guard: Ensure correct window size
  if window > series_1.len() {
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Window size is greater than vector length")));
  }

  // Calculate rolling cointegration for each window
  for i in window..series_1.len() {
    let series_1_i: &Vec<f64> = &series_1[i-window..i].to_vec();
    let series_2_i: &Vec<f64> = &series_2[i-window..i].to_vec();
    let coint: Coint = cointegration_test_eg(series_1_i, series_2_i)?;
    let t_stat: f64 = coint.test_statistic;
    let c_value: f64 = coint.critical_values.1 as f64;
    let t_distance: f64 = -(t_stat - c_value);
    t_distances.push(t_distance);
  }
  Ok(t_distances)
}
