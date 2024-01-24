use super::models::WinRate;
use super::utils::{log_to_simple_returns, round_float};
use serde::{Deserialize, Serialize};
use ts_rs::TS;


#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct BacktestMetrics {
  pub arr: f64,
  pub drawdowns: Vec<f64>,
  pub equity_curve: Vec<f64>,
  pub max_drawdown: f64,
  pub mean_return: f64,
  pub sharpe_ratio: f64,
  pub sortino_ratio: f64,
  pub total_return: f64,
  pub win_rate_stats: WinRate
}

#[derive(Debug)]
pub struct Evaluation {
  pub log_returns: Vec<f64>,
  pub cum_norm_returns: Vec<f64>,
  pub win_rate_stats: WinRate,
}

impl Evaluation {
  pub fn new(log_returns: Vec<f64>, cum_norm_returns: Vec<f64>, win_rate_stats: WinRate) -> Self {
    Self {
      log_returns,
      cum_norm_returns,
      win_rate_stats,
    }
  }

  // Annual Rate of Return
  fn annual_rate_of_return(&self) -> f64 {
    let mean_return: f64 = self.mean_return();
    let periods_per_year: f64 = 252.0; // for daily returns
    (1.0 + mean_return).powf(periods_per_year) - 1.0
  }

  /// Drawdowns
  fn drawdowns(&self) -> Vec<f64> {
    let norm_returns: Vec<f64> = self.cum_norm_returns.clone();
    let mut drawdowns: Vec<f64> = Vec::new();
    let mut max_return_so_far: f64 = norm_returns[0];
    for r in norm_returns {
      if r > max_return_so_far {
        max_return_so_far = r;
      }
      let drawdown: f64 = max_return_so_far - r;
      drawdowns.push(-drawdown);
    }
    
    drawdowns
  }

  /// Mean Return
  /// Takes in log returns and provides a linear mean return value
  fn mean_return(&self) -> f64 {
    let filtered_vec: Vec<&f64> = self.log_returns.iter().filter(|&&x| x != 0.0).collect();
    let sum: f64 = filtered_vec.iter().fold(0.0, |a, b| a + **b);
    let count: usize = filtered_vec.len();
    
    let log_ret = match count {
      0 => 0.0,
      _ => sum / (count as f64),
    };

    f64::exp(log_ret) - 1.0
  }

  /// Sharpe Ratio
  fn sharpe_ratio(&self, risk_free_rate_annual: f64) -> f64 {
    let n: f64 = self.log_returns.len() as f64;
    if n == 0.0 { return 0.0; }

    let annual_trading_days = 252.0;

    // Convert the annual risk-free rate to a daily rate
    let risk_free_rate_daily = (1.0 + risk_free_rate_annual).powf(1.0 / annual_trading_days) - 1.0;

    let mean: f64 = self.log_returns.iter().sum::<f64>() / n;
    // Adjust the mean by subtracting the daily risk-free rate
    let adjusted_mean = mean - risk_free_rate_daily;

    let variance: f64 = self.log_returns.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n;
    if variance == 0.0 { return 0.0; }

    // Calculate the annualized Sharpe ratio
    adjusted_mean * annual_trading_days.sqrt() / variance.sqrt()
  }

  /// Sortino Ratio without risk-free rate
  fn sortino_ratio(&self, risk_free_rate_annual: f64) -> f64 {
  let n: f64 = self.log_returns.len() as f64;
  if n == 0.0 { return 0.0; }

  let annual_trading_days = 252.0;

  // Convert the annual risk-free rate to a daily rate
  let risk_free_rate_daily = (1.0 + risk_free_rate_annual).powf(1.0 / annual_trading_days) - 1.0;

  let mean: f64 = self.log_returns.iter().sum::<f64>() / n;
  // Adjust the mean by subtracting the daily risk-free rate
  let adjusted_mean = mean - risk_free_rate_daily;

  // Calculate the downside deviation
  let downside_deviation: f64 = self.log_returns.iter()
    .filter(|&&x| x < risk_free_rate_daily) // consider only returns less than the risk-free rate
    .map(|&x| (x - risk_free_rate_daily).powi(2))
    .sum::<f64>() / n;

  if downside_deviation == 0.0 { return 0.0; }

  // Calculate the annualized Sortino ratio
  adjusted_mean * annual_trading_days.sqrt() / downside_deviation.sqrt()
}


  /// Total Return
  fn total_return(&self) -> f64 {
    self.cum_norm_returns[self.cum_norm_returns.len() - 1]
  }

  // Max Drawdown
  fn calculate_max_drawdown(&self) -> f64 {
    let mut max_drawdown = 0.0;
    let equity_curve: Vec<f64> = self.log_returns.iter().scan(1.0, |state, &log_return| { *state *= log_return.exp(); Some(*state) }).collect();
    let mut peak = equity_curve[0];

    for &value in equity_curve.iter() {
      if value > peak {
        peak = value;
      }
      let drawdown = (peak - value) / peak;
      if drawdown > max_drawdown {
        max_drawdown = drawdown;
      }
    }

    max_drawdown
}

  /// Run Evaluation Metrics
  /// Calculates metrics and returns net evaluation serialized
  pub fn run_evaluation_metrics(&self) -> BacktestMetrics {

    let arr: f64 = round_float(self.annual_rate_of_return(), 2);
    let drawdowns: Vec<f64> = self.drawdowns().iter().map(|f| round_float(*f, 3)).collect();
    let equity_curve: Vec<f64> = self.cum_norm_returns.iter().map(|f| round_float(*f, 4)).collect();
    let max_drawdown: f64 = -round_float(self.calculate_max_drawdown(), 2);
    let mean_return: f64 = round_float(self.mean_return(), 3);
    let sharpe_ratio: f64 = round_float(self.sharpe_ratio(0.015), 2);
    let sortino_ratio: f64 = round_float(self.sortino_ratio(0.015), 2);
    let total_return: f64 = round_float(self.total_return(), 2);
    let win_rate_stats: WinRate = self.win_rate_stats.to_owned();

    BacktestMetrics { arr, drawdowns, equity_curve, max_drawdown, mean_return, 
      sharpe_ratio, sortino_ratio, total_return, win_rate_stats }
  }
}