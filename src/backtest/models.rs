use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::SmartError;
use crate::stats::metrics::{cointegration_test_eg, pearson_correlation_coefficient};
use crate::stats::models::Coint;
use super::evaluation::{Evaluation, BacktestMetrics};
use super::utils::log_returns;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, TS)]
#[ts(export)]
pub enum LongSeries {
  Series0, // Asset0
  Series1 // Asset1
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, TS)]
#[ts(export)]
pub enum TriggerIndicator {
  Zscore,
  Spread
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, TS)]
#[ts(export)]
pub enum Relation {
  Coint,
  Corr,
  Ignore
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct BacktestCriteria {
  pub indicator_values: Vec<f64>,
  pub trigger_indicator: TriggerIndicator,
  pub relation: Relation,
  pub cost_per_leg: Option<f64>,
  pub rets_weighting_s0_perc: f64,
  pub long_series: LongSeries,
  pub stop_loss: f64,
  pub long_thresh: f64,
  pub long_close_thresh: f64,
  pub short_thresh: f64,
  pub short_close_thresh: f64
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct WinRate {
  pub win_rate: f64,
  pub opened: u32,
  pub closed: u32,
  pub closed_profit: u32
}

#[derive(Debug)]
pub struct Backtest {
  pub series_0: Vec<f64>, 
  pub series_1: Vec<f64>, 
  pub series_0_mul: f64, // for determining long or short
  pub bt_criteria: BacktestCriteria
}

impl Backtest {
  pub fn new(
    series_0: &Vec<f64>, 
    series_1: &Vec<f64>, 
    bt_criteria: BacktestCriteria
  ) -> Self {

    // Guard: Ensure correct lengths
    assert_eq!(series_0.len(), series_1.len());
    assert_eq!(series_0.len(), bt_criteria.indicator_values.len());

    // Guard: Ensure correct thresholds
    assert!(bt_criteria.long_thresh <= bt_criteria.short_thresh);
    assert!(bt_criteria.long_close_thresh >= bt_criteria.long_thresh);
    assert!(bt_criteria.short_close_thresh <= bt_criteria.short_thresh);

    // Series 0 multiplication factor
    let series_0_mul: f64 = if bt_criteria.long_series == LongSeries::Series0 { 1.0 } else { -1.0 };

    Self {
      series_0: series_0.clone(),
      series_1: series_1.clone(),
      series_0_mul,
      bt_criteria
    }
  }

  /// Create Signals
  /// Generates Signals and Relevant Baktest Information
  fn create_signals(&self) -> Result<(Vec<i32>, Vec<f64>, WinRate), SmartError> {

    // Initialize
    let mut is_open: bool = false;
    let mut last: i32 = 0;
    let mut signals: Vec<i32> = vec![0];
    let mut trading_open_costs: Vec<f64> = vec![0.0];
    let mut trading_close_costs: Vec<f64> = vec![0.0];

    let mut tracked_profit: f64 = 0.0;
    let mut opened: u32 = 0;
    let mut closed: u32 = 0;
    let mut closed_profit: u32 = 0;

    let rolling_window: usize = 90; // used for cointegration check
    let corr_thresh: f64 = 0.8; // used for correlation check

    let cost_per_leg: f64 = match self.bt_criteria.cost_per_leg { Some(c) => c, None => 0.0 };

    for i in 1..self.bt_criteria.indicator_values.len() {

      // Extract Indicator Value
      let ind_val: f64 = self.bt_criteria.indicator_values[i];

      // Handle Returns Calc (helps check if profit for win rate)
      let ser_0_ret: f64 = (self.series_0[i] / self.series_0[i - 1] - 1.0) * self.series_0_mul;
      let ser_1_ret: f64 = (self.series_1[i] / self.series_1[i - 1] - 1.0) * -self.series_0_mul;
      
      // Confirm Long and Short Open Triggers
      let mut is_long_trigger: bool = false;
      let mut is_short_trigger: bool = false;
      if !is_open {

        let is_relation = match &self.bt_criteria.relation {
          Relation::Coint => {
            if i >= rolling_window {
              let series_0_i: &Vec<f64> = &self.series_0[i-rolling_window..i].to_vec();
              let series_1_i: &Vec<f64> = &self.series_1[i-rolling_window..i].to_vec();
              let coint: Coint = cointegration_test_eg(series_0_i, series_1_i)?;
              coint.is_coint
            } else {
              false
            }
          },
          Relation::Corr => {
            if i >= rolling_window {
              let series_0_i: &Vec<f64> = &self.series_0[i-rolling_window..i].to_vec();
              let series_1_i: &Vec<f64> = &self.series_1[i-rolling_window..i].to_vec();
              let corr: f64 = pearson_correlation_coefficient(series_0_i, series_1_i)?;
              corr.abs() >= corr_thresh
            } else {
              false
            }
          },
          Relation::Ignore => true
        };

        if is_relation {
          if ind_val <= self.bt_criteria.long_thresh { is_long_trigger = true; }
          if ind_val >= self.bt_criteria.short_thresh { is_short_trigger = true; }
        }
      }
      
      // Confirm Long and Short Close Triggers
      let mut is_long_close_trigger: bool = false;
      let mut is_short_close_trigger: bool = false;
      if is_open {
        if ind_val >= self.bt_criteria.long_close_thresh && last == 1 { is_long_close_trigger = true; }
        if ind_val <= self.bt_criteria.short_close_thresh && last == -1 { is_short_close_trigger = true; }

        // Handle stop loss
        // Net returns also adjusted for stop loss later on
        if self.bt_criteria.stop_loss != 0.0 {
          if tracked_profit <= self.bt_criteria.stop_loss {
            is_long_close_trigger = true;
            is_short_close_trigger = true;
          }
        }
      }

      // Open Long
      if is_long_trigger {
        is_open = true;
        last = 1;
        signals.push(1);
        trading_open_costs.push(cost_per_leg * 2.0);
        trading_close_costs.push(0.0);

        tracked_profit = -cost_per_leg * 2.0;
        opened += 1;
        continue;
      }

      // Open Short
      if is_short_trigger {
        is_open = true;
        last = -1;
        signals.push(-1);
        trading_open_costs.push(cost_per_leg * 2.0);
        trading_close_costs.push(0.0);

        tracked_profit = -cost_per_leg * 2.0;
        opened += 1;
        continue;
      }

      // Close Long or Short
      if is_long_close_trigger || is_short_close_trigger {
        is_open = false;
        
        last = 0;
        signals.push(0);
        trading_close_costs.push(cost_per_leg * 2.0);
        trading_open_costs.push(0.0);
        
        tracked_profit += -cost_per_leg * 2.0;
        if tracked_profit > 0.0 { closed_profit += 1; } 
        tracked_profit = 0.0;
        closed += 1;
        continue;
      }

      // Check Current Profit
      if is_open {
        tracked_profit += ser_0_ret + ser_1_ret;
      } else {
        tracked_profit = 0.0;
      }

      // Update Signals and Costs
      signals.push(last);
      trading_open_costs.push(0.0);
      trading_close_costs.push(0.0);
    }

    // Shift signals by 1 to avoid lookahead bias
    if let Some(_) = signals.pop() { signals.insert(0, 0); }
    if let Some(_) = trading_open_costs.pop() { trading_open_costs.insert(0, 0.0); }

    // Combine trading costs for open and close fees
    let trading_costs: Vec<f64> = trading_open_costs.iter().zip(trading_close_costs.iter())
        .map(|(&x, &y)| x + y)
        .collect();

    // Structure Win Rate Metrics
    let win_rate: f64 = closed_profit as f64 / closed as f64;
    let win_rate_metrics: WinRate = WinRate { win_rate, opened, closed, closed_profit };

    Ok((signals, trading_costs, win_rate_metrics))
  }

  /// Strategy Returns
  /// Calculates Returns based on Signals and Trading Costs
  fn strategy_returns(&self, signals: Vec<i32>, trading_costs: Vec<f64>) -> (Vec<f64>, Vec<f64>) {

    // Calculate weighting ratio
    let s0_weighting_rate: f64 = 2.0 * self.bt_criteria.rets_weighting_s0_perc;
    let s1_weighting_rate: f64 = 2.0 - s0_weighting_rate;

    // Calculate log returns
    let log_rets_0: Vec<f64> = log_returns(&self.series_0, true);
    let log_rets_1: Vec<f64> = log_returns(&self.series_1, true);

    
    // Calculate strategy log returns - series 0
    let series_0_r: Vec<f64> = log_rets_0.iter().zip(signals.iter())
    .map(|(&x, &y)| x * y as f64 * self.series_0_mul * s0_weighting_rate)
    .collect();
  
    // Calculate strategy log returns - series 1
    let series_1_r: Vec<f64> = log_rets_1.iter().zip(signals.iter())
      .map(|(&x, &y)| x * y as f64 * -self.series_0_mul * s1_weighting_rate)
      .collect();

    // Calculate strategy log returns - net
    let mut net_lrets: Vec<f64> = series_0_r.iter()
      .zip(series_1_r.iter())
      .zip(trading_costs.iter())
      .map(|((&x, &y), &z)| x + y - z)
      .collect();

    // Adjust net returns for stop loss
    // Net returns also adjusted for stop loss later on
    if self.bt_criteria.stop_loss != 0.0 {
      for i in 0..net_lrets.len() {
        if (net_lrets[i].exp() + 1.0) < self.bt_criteria.stop_loss {
          net_lrets[i] = 0.0;
        }
      }
    }

    // Calculate strategy cumulative log returns - net
    let net_cum_rets: Vec<f64> = net_lrets.iter()
      .scan(0.0, |state, &x| {
          *state += x;
          Some(*state)
      })
      .map(|cum_log_ret| f64::exp(cum_log_ret) - 1.0)
      .collect();

    // Return output
    (net_lrets, net_cum_rets)
  }

  /// Run Backtest
  /// Entrypoint for running backtest
  pub fn run_backtest(&self) -> Result<BacktestMetrics, SmartError> {
    let (signals, trading_costs, win_rate) = self.create_signals()?;
    let (net_lrets, net_cum_rets) = self.strategy_returns(signals, trading_costs);
    let evaluation: Evaluation = Evaluation::new(net_lrets, net_cum_rets, win_rate);
    let eval_metrics: BacktestMetrics = evaluation.run_evaluation_metrics();
    Ok(eval_metrics)
  }
}
