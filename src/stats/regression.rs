use crate::SmartError;
use statrs;
use statrs::distribution::{FisherSnedecor, ContinuousCDF, StudentsT};

/// Residuals
/// Calculates the differences between the actual and predicted values
pub fn calculate_residuals(x: &Vec<f64>, y: &Vec<f64>, beta_0: f64, beta_1: f64) -> Vec<f64> {
  x.iter().zip(y.iter())
    .map(|(&x_i, &y_i)| y_i - (beta_0 + beta_1 * x_i)).collect()
}

/// T and P-Values
/// Calculates the t and p values for the beta coefficients B0 and B1
/// Scenario: B1 p-value < 0.5:
///   The slope of the regression line has a significant effect on the dependant variable y
///   For each unit in x, the predicted value in y increases by B1 units
pub fn calculate_coefficients_t_and_p_values(x: &Vec<f64>, beta_0: f64, beta_1: f64, see: f64) -> ((f64, f64), (f64, f64)) {
  let n: f64 = x.len() as f64;
  let x_bar: f64 = x.iter().sum::<f64>() / n;
  let sum_squared_x_minus_x_bar: f64 = x.iter().map(|&x_i| (x_i - x_bar).powi(2)).sum();
  let se_beta_0: f64 = see * ((1.0 / n + x_bar.powi(2) / sum_squared_x_minus_x_bar).sqrt());
  let se_beta_1: f64 = see / (sum_squared_x_minus_x_bar.sqrt());
  let t_beta_0: f64 = beta_0 / se_beta_0;
  let t_beta_1: f64 = beta_1 / se_beta_1;

  // calculate p-values
  let dof: f64 = n - 2.0;  // degrees of freedom
  let t_dist: StudentsT = StudentsT::new(0.0, 1.0, dof).unwrap();
  let p_beta_0: f64 = 2.0 * (1.0 - t_dist.cdf(t_beta_0.abs()));
  let p_beta_1: f64 = 2.0 * (1.0 - t_dist.cdf(t_beta_1.abs()));

  ((t_beta_0, p_beta_0), (t_beta_1, p_beta_1))
}

/// F-Statistic
/// Indicates whether there is a relationship between our predictor and response variable
pub fn calculate_f_statistic(x: &Vec<f64>, y: &Vec<f64>, beta_0: f64, beta_1: f64) -> (f64, f64) {
  let n: f64 = x.len() as f64;
  let p: f64 = 1.0;  // For simple linear regression, p = 1
  let y_bar: f64 = y.iter().sum::<f64>() / n;
  let tss: f64 = y.iter().map(|&y_i| (y_i - y_bar).powi(2)).sum();
  let rss: f64 = x.iter().zip(y.iter())
      .map(|(&x_i, &y_i)| {
          let y_hat_i = beta_0 + beta_1 * x_i;
          (y_i - y_hat_i).powi(2)
      }).sum();
  let f_statistic: f64 = ((tss - rss) / p) / (rss / (n - p - 1.0));

  // Calculate p-value
  let dof1: f64 = p as f64;  // degrees of freedom for numerator (p)
  let dof2: f64 = n - p - 1.0;  // degrees of freedom for denominator (n - p - 1)
  let f_dist: FisherSnedecor = FisherSnedecor::new(dof1, dof2).unwrap();
  let p_value: f64 = 1.0 - f_dist.cdf(f_statistic);
  (f_statistic, p_value)
}

/// Standard Error of the Estimate
/// A measure of the accuracy of the predictions made with a regression line
pub fn calculate_see(x: &Vec<f64>, y: &Vec<f64>, beta_0: f64, beta_1: f64) -> f64 {
  let n: f64 = x.len() as f64;
  let sum_squared_residuals: f64 = x.iter().zip(y.iter())
      .map(|(&x_i, &y_i)| {
          let y_hat_i = beta_0 + beta_1 * x_i;
          (y_i - y_hat_i).powi(2)
      }).sum();
  let see: f64 = (sum_squared_residuals / (n - 2.0)).sqrt();
  see
}

/// R-Squared
/// Proportion of the variance in y that is predictable from the independant variable
/// A value of 1 indicates a perfect firt where as 0 means it explains no variability
pub fn calculate_r_squared(x: &Vec<f64>, y: &Vec<f64>) -> f64 {
  let n: f64 = x.len() as f64;
  let sum_x: f64 = x.iter().sum();
  let sum_y: f64 = y.iter().sum();
  let sum_xx: f64 = x.iter().map(|&x| x.powi(2)).sum();
  let sum_yy: f64 = y.iter().map(|&y| y.powi(2)).sum();
  let sum_xy: f64 = x.iter().zip(y.iter()).map(|(&x, &y)| x * y).sum();
  let r: f64 = (n * sum_xy - sum_x * sum_y) / ((n * sum_xx - sum_x.powi(2)) * (n * sum_yy - sum_y.powi(2))).sqrt();
  let r_squared = r.powi(2);
  r_squared
}

/// Simple Linear Regresison
/// y - dependant variable
/// x - independant variable
/// beta_0 - intercept (predicted value of y when x is zero)
/// beta_1 - slope (amount y will change for each unit change of x)
/// If is_stats is set to false, only beta_1 and beta_0 will be returned
pub fn simple_linear_regression(x: &Vec<f64>, y: &Vec<f64>) -> Result<((f64, f64), Vec<f64>), SmartError> {
  if x.len() != y.len() {
    return Err(SmartError::RuntimeCheck("Input vectors have different sizes".to_string()));
  }
  
  let n: f64 = x.len() as f64;
  let sum_x: f64 = x.iter().sum();
  let sum_y: f64 = y.iter().sum();
  let sum_xx: f64 = x.iter().map(|&x| x.powi(2)).sum();
  let sum_xy: f64 = x.iter().zip(y.iter()).map(|(&x, &y)| x * y).sum();

  let denominator: f64 = n * sum_xx - sum_x.powi(2);
  if denominator.abs() < std::f64::EPSILON {
    return Err(SmartError::RuntimeCheck("The variance of x values is zero".to_string()));
  }
    
  let beta_1: f64 = (n * sum_xy - sum_x * sum_y) / denominator;
  let beta_0: f64 = sum_y / n - beta_1 * sum_x / n;
  
  let residuals: Vec<f64> = calculate_residuals(x, y, beta_0, beta_1);

  Ok(((beta_0, beta_1), residuals))
}
