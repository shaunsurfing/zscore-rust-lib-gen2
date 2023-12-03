use crate::SmartError;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::metrics::{
  cointegration_test_eg,
  half_life_mean_reversion,
  spread_static_std,
  spread_dynamic_kalman,
  rolling_zscore,
  rolling_cointegration,
  rolling_correlation, pearson_correlation_coefficient
};

use super::statistics::calculate_relaitonship;

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub enum SpreadType {
  Static,
  Dynamic
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct Relationship {
  pub beta_x_to_y: f64,
  pub beta_y_to_x: f64,
  pub annual_vol_y: f64,
  pub annual_vol_x: f64,
  pub vol_ratio_x_to_y: f64
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct Coint {
  pub is_coint: bool,
  pub test_statistic: f64,
  pub critical_values: (f64, f64, f64),
  pub p_value: f64
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct Statistics {
  pub coint: Coint,
  pub corr: f64,
  pub half_life: f64,
  pub hedge_ratio: f64,
  pub spread: Vec<f64>,
  pub zscore: Vec<f64>,
  pub relationship: Relationship,
  pub coint_roll: Vec<f64>,
  pub corr_roll: Vec<f64>
}

impl Statistics {

  /// Calculate Statistics
  /// Calculates cointegration, spread etc
  pub fn calculate_statistics(
    series_0: &Vec<f64>, 
    series_1: &Vec<f64>, 
    calc_type: SpreadType, 
    z_score_w: usize, 
    roll_w: usize,
  ) -> Result<Self, SmartError> {

    // Guard: Ensure lengh > 0
    if series_0.len() == 0 { return Err(SmartError::RuntimeCheck("Series_0 length zero".to_string())) }
    if series_1.len() == 0 { return Err(SmartError::RuntimeCheck("Series_1 length zero".to_string())) }

    // Cointegration
    let coint: Coint = match cointegration_test_eg(&series_0, &series_1) {
      Ok(coint) => coint,
      Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error cointegration: {}", e)))
    };

    // Correlation
    let corr: f64 = match pearson_correlation_coefficient(&series_0, &series_1) {
      Ok(corr) => corr,
      Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error cointegration: {}", e)))
    };

    // Extract Hedge Ratio and Spread
    let (spread, hedge_ratio) = match calc_type {
      SpreadType::Static => {
        match spread_static_std(&series_0, &series_1) {
          Ok((spread, hedge_ratio)) => (spread, hedge_ratio),
          Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error spread_static: {}", e)))
        }
      },
      SpreadType::Dynamic => {
        match spread_dynamic_kalman(&series_0, &series_1) {
          Ok((spread, hedge_ratio)) => (spread, hedge_ratio),
          Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error spread_dyn: {}", e)))
        }
      }
    };

    // Half Life
    let half_life: f64 = match half_life_mean_reversion(&spread) {
      Ok(half_life) => half_life,
      Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error half_life: {}", e)))
    };

    // ZScore Rolling
    let zscore: Vec<f64> = match rolling_zscore(&spread, z_score_w) {
      Ok(zscore) => zscore,
      Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error zscore_roll: {}", e)))
    };

    // Coint Rolling
    let coint_roll: Vec<f64> = match rolling_cointegration(&series_0, &series_1, roll_w) {
      Ok(zscore) => zscore,
      Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error coint_roll: {}", e)))
    };

    // Corr Rolling
    let corr_roll: Vec<f64> = match rolling_correlation(&series_0, &series_1, roll_w) {
      Ok(zscore) => zscore,
      Err(e) => return Err(SmartError::RuntimeCheck(format!("Statistics calculation error corr_roll: {}", e)))
    };

    // Relationship
    let trading_days: usize = 252;
    let relationship: Relationship = calculate_relaitonship(&series_0, &series_1, trading_days).map_err(|e| SmartError::RuntimeCheck(e.to_string()))?;

    // Consolidate Result
    let stats: Self = Self {
      coint,
      corr,
      half_life,
      hedge_ratio,
      spread,
      zscore,
      relationship,
      coint_roll,
      corr_roll
    };

    Ok(stats)
  }
}
