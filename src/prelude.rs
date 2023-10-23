use wasm_bindgen::prelude::wasm_bindgen;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::SmartError;
use super::backtest::evaluation::BacktestMetrics;
use super::backtest::models::{Backtest, BacktestCriteria, LongSeries, TriggerIndicator, Relation};
use super::pricing::models::{AssetType, DataCriteria, Exchange, PairPrices, QuotePrice};
use super::pricing::symbols::request_symbols;
use super::pricing::entry::fetch_prices;
use super::pricing::quotes::request_quote;
use super::pricing::quotemulti::request_multi_quote;
use super::stats::models::{SpreadType, Statistics, Coint};
use super::stats::metrics::{
  spread_dynamic_kalman, spread_static_std, rolling_zscore, 
  cointegration_test_eg, pearson_correlation_coefficient, half_life_mean_reversion
};

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct StatsCriteria {
  pub spread_type: SpreadType,
  pub zscore_window: usize,
  pub roll_window: usize
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct AnalysisCriteria {
  pub data_criteria: DataCriteria,
  pub stats_criteria: Option<StatsCriteria>,
  pub backtest_criteria: Option<BacktestCriteria>
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct PairAnalysis {
  pub prices: PairPrices,
  pub stats: Statistics,
  pub bt_metrics: BacktestMetrics
}

/// Single Quote
/// Retrieves a single quote from an exchange provider
pub async fn single_quote(exchange: &Exchange, symbol: &str, twelve_api_key: Option<&str>) -> Result<f64, SmartError> {
  request_quote(exchange, symbol, twelve_api_key).await
}

/// Single Quote
/// Retrieves a single quote from an exchange provider
pub async fn multi_symbol_quote(exchange: &Exchange, symbols: Vec<&str>, twelve_api_key: Option<&str>) -> Result<Vec<QuotePrice>, SmartError> {
  request_multi_quote(exchange, symbols, twelve_api_key).await
}

/// Full Analysis From Pair Prices
/// Retrieves Stats, Eval Metrics and ML Metrics given the pair prices
pub async fn full_analysis_from_pair_prices(
  prices: PairPrices, 
  stats_criteria_opt: Option<StatsCriteria>,
  backtest_criteria_opt: Option<BacktestCriteria>
) -> Result<PairAnalysis, SmartError> {

  let (calc_type, z_score_w, roll_w) = match stats_criteria_opt {
    Some(st) => (st.spread_type, st.zscore_window, st.roll_window),
    None => (SpreadType::Dynamic, 35, 90)
  };

  let stats: Statistics = Statistics::calculate_statistics(
    &prices.series_0, 
    &prices.series_1, 
    calc_type, 
    z_score_w,
    roll_w
  )?;

  let backtest_criteria: BacktestCriteria = match backtest_criteria_opt {
    Some(bt) => bt,
    None => BacktestCriteria {
      indicator_values: stats.zscore.clone(),
      trigger_indicator: TriggerIndicator::Zscore,
      relation: Relation::Ignore,
      cost_per_leg: Some(0.0005),
      rets_weighting_s0_perc: 0.5,
      long_series: LongSeries::Series0,
      stop_loss: 0.0,
      long_thresh: -1.5,
      long_close_thresh: 0.0,
      short_thresh: 1.5,
      short_close_thresh: 0.0
    },
  };

  let backtest: Backtest = Backtest::new(
    &prices.series_0,
    &prices.series_1,
    backtest_criteria
  );

  let bt_metrics: BacktestMetrics = backtest.run_backtest()?;

  Ok(PairAnalysis { prices, stats, bt_metrics })
}

/// Pair Prices
/// Retrieves Prices
pub async fn pair_prices(data_criteria: DataCriteria, twelve_api_key: Option<&str>) -> Result<PairPrices, SmartError> {
  fetch_prices(
    &data_criteria.interval_period, 
    &data_criteria.exchange, 
    &data_criteria.asset_0, 
    &data_criteria.asset_1, 
    twelve_api_key
  ).await
}

/// Full Pair Analysis
/// Retrieves Prices, Stats, Eval Metrics and ML Metrics
pub async fn full_pair_analysis(analysis_criteria: AnalysisCriteria, twelve_api_key: Option<&str>) -> Result<PairAnalysis, SmartError> {
  let prices: PairPrices = pair_prices(analysis_criteria.data_criteria, twelve_api_key).await?;
  let analysis: PairAnalysis = full_analysis_from_pair_prices(
    prices, 
    analysis_criteria.stats_criteria, 
    analysis_criteria.backtest_criteria
  ).await?;
  Ok(analysis)
}

/*
  WASM
  Web Assembly Calls
*/

/// WASM Entry - Exchange Tickers
/// Provides 
#[wasm_bindgen]
pub async fn wasm_exchange_tickers(json_input: String) -> Result<String, String> {
  let exchange: Exchange = serde_json::from_str::<Exchange>(&json_input).map_err(|e| e.to_string())?;
  let asset_type: AssetType = AssetType::Crypto;
  let symbols: Vec<String> = request_symbols(&exchange, Some(asset_type)).await
    .map_err(|e| e.to_string())?;
  Ok(serde_json::to_string(&symbols).unwrap_or_else(|e| e.to_string()))
}

/// WASM Entry - Exchange Single Quote
/// Extracts status for a single exchange
#[wasm_bindgen]
pub async fn wasm_exchange_single_quote(exchange: String, symbol: String) -> Result<String, String> {
  let exchange: Exchange = Exchange::create_from_string(exchange.as_str());

  let quote: f64 = single_quote(&exchange, symbol.as_str(), None).await
    .map_err(|e| e.to_string())?;

  Ok(quote.to_string())
}

/// WASM Entry - Multi Symbol Quote
/// Extracts status for multiple symbols
#[wasm_bindgen]
pub async fn wasm_multi_symbol_quote(exchange: String, symbols: String) -> Result<String, String> {
  let exchange: Exchange = Exchange::create_from_string(exchange.as_str());
  let symbols: Vec<&str> = serde_json::from_str::<Vec<&str>>(&symbols).map_err(|e| e.to_string())?;

  let quotes: Vec<QuotePrice> = multi_symbol_quote(&exchange, symbols, None).await
    .map_err(|e| e.to_string())?;

  let quote_json: String = serde_json::to_string::<Vec<QuotePrice>>(&quotes).map_err(|e| e.to_string())?;
  Ok(quote_json)
}

/// WASM Entry - Exchange Quotes
/// Extracts status for all public data exchanges (thus excluding Twelve)
#[wasm_bindgen]
pub async fn wasm_exchange_quotes() -> Result<String, String> {

  let symbol_binance = Exchange::Binance.default_assets().0;
  let symbol_bybit = Exchange::ByBit.default_assets().0;
  let symbol_coinbase = Exchange::Coinbase.default_assets().0;
  let symbol_dydx = Exchange::Dydx.default_assets().0;
  let request_quote_1 = request_quote(&Exchange::Binance, symbol_binance.as_str(), None);
  let request_quote_2 = request_quote(&Exchange::BinanceUs, symbol_binance.as_str(), None);
  let request_quote_3 = request_quote(&Exchange::ByBit, symbol_bybit.as_str(), None);
  let request_quote_4 = request_quote(&Exchange::Coinbase, symbol_coinbase.as_str(), None);
  let request_quote_5 = request_quote(&Exchange::Dydx, symbol_dydx.as_str(), None);
  let futures = vec!(request_quote_1, request_quote_2, request_quote_3, request_quote_4, request_quote_5);

  let results: Vec<Result<f64, String>> = futures::future::join_all(futures)
    .await
    .into_iter()
    .map(|res| res.map_err(|e| e.to_string()))
    .collect();

  // Convert the Vec<Result<f64, String>> to JSON String
  Ok(serde_json::to_string(&results).unwrap_or_else(|e| e.to_string()))
}

/// WASM Entry - Pair Prices
/// Retrieves Prices for given pair
#[wasm_bindgen]
pub async fn wasm_pair_prices(json_input: String, twelve_api_key: Option<String>) -> Result<String, String> {
  let data_criteria: DataCriteria = serde_json::from_str(&json_input).map_err(|e| e.to_string())?;
  let pair_prices: PairPrices = pair_prices(data_criteria, twelve_api_key.as_deref()).await.map_err(|e| e.to_string())?;
  Ok(serde_json::to_string::<PairPrices>(&pair_prices).map_err(|e| e.to_string())?)
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
struct QuickStats {
  spread: Vec<f64>,
  zscore: Vec<f64>,
  hedge_ratio: f64,
  half_life: f64
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
struct StatsOutput {
  stats_static: QuickStats,
  stats_dynamic: QuickStats,
  coint: Coint,
  corr: f64
}

/// WASM Entry - Provides Spread
/// Calculates Spread based on prices
#[wasm_bindgen]
pub async fn wasm_quick_stats(json_input: String, zscore_window_str: String) -> Result<String, String> {
  let pair_prices: PairPrices = serde_json::from_str(&json_input).map_err(|e| e.to_string())?;
  let zscore_window: usize = zscore_window_str.parse::<usize>().map_err(|e| e.to_string())?;

  let (spread_static, hedge_ratio_static) = match spread_static_std(&pair_prices.series_0, &pair_prices.series_1) {
    Ok((spread, hedge_ratio)) => (spread, hedge_ratio),
    Err(e) => return Err(format!("Statistics calculation error spread_static: {}", e))
  };

  let (spread_dynamic, hedge_ratio_dynamic) = match spread_dynamic_kalman(&pair_prices.series_0, &pair_prices.series_1) {
    Ok((spread, hedge_ratio)) => (spread, hedge_ratio),
    Err(e) => return Err(format!("Statistics calculation error spread_dyn: {}", e))
  };

  let zscore_static: Vec<f64> = rolling_zscore(&spread_static, zscore_window).map_err(|e| e.to_string())?;
  let zscore_dynamic: Vec<f64> = rolling_zscore(&spread_dynamic, zscore_window).map_err(|e| e.to_string())?;

  let half_life_static = half_life_mean_reversion(&spread_static).map_err(|e| e.to_string())?;
  let half_life_dynamic = half_life_mean_reversion(&spread_dynamic).map_err(|e| e.to_string())?;

  let coint: Coint = cointegration_test_eg(&pair_prices.series_0, &pair_prices.series_1).map_err(|e| e.to_string())?;
  let corr: f64 = pearson_correlation_coefficient(&pair_prices.series_0, &pair_prices.series_1).map_err(|e| e.to_string())?;
  
  let stats_static: QuickStats = QuickStats { 
    spread: spread_static,
    zscore: zscore_static,
    hedge_ratio: hedge_ratio_static,
    half_life: half_life_static
  };

  let stats_dynamic: QuickStats = QuickStats { 
    spread: spread_dynamic,
    zscore: zscore_dynamic,
    hedge_ratio: hedge_ratio_dynamic,
    half_life: half_life_dynamic
  };

  let stats_output: StatsOutput = StatsOutput { stats_static, stats_dynamic, coint, corr };

  Ok(serde_json::to_string::<StatsOutput>(&stats_output).map_err(|e| e.to_string())?)
}

/// WASM Entry - Backtest from Pair Prices
/// Performs backtest from prices and Backtest Criteria
#[wasm_bindgen]
pub async fn wasm_quick_backtest(pair_prices_json: String, bt_criteria_json: String) -> Result<String, String> {

  // Deserialize - Pair Prices
  let pair_prices: PairPrices = serde_json::from_str::<PairPrices>(&pair_prices_json).map_err(|e| e.to_string())?;

  // Deserialize - Backtest Criteria
  let bt_criteria: BacktestCriteria = serde_json::from_str::<BacktestCriteria>(&bt_criteria_json).map_err(|e| e.to_string())?;

  // Structure Backtest
  let backtest: Backtest = Backtest::new(
    &pair_prices.series_0,
    &pair_prices.series_1,
    bt_criteria
  );

  // Perform Backtest
  let bt_metrics: BacktestMetrics = backtest.run_backtest().map_err(|e| e.to_string())?;

  // Serialize
  let bt_metrics_json: String = serde_json::to_string::<BacktestMetrics>(&bt_metrics).map_err(|e| e.to_string())?;
  Ok(bt_metrics_json)
}


/// WASM Entry - Full Pair Analysis
/// Only for use on exchanges as no api key should be sent via wasm
#[wasm_bindgen]
pub async fn wasm_full_pair_analysis_crypto(json_input: String) -> Result<String, String> {

  // Deserialize
  let analysis_criteria_res: Result<AnalysisCriteria, String> = serde_json::from_str::<AnalysisCriteria>(&json_input)
    .map_err(|e| e.to_string());

  let Ok(analysis_criteria) = analysis_criteria_res else { return Err(analysis_criteria_res.err().unwrap()) };

  // Perform Function
  let analysis_res: Result<PairAnalysis, String> = full_pair_analysis(analysis_criteria, None)
    .await.map_err(|e| e.to_string());

  let Ok(analysis) = analysis_res else { return Err(analysis_res.err().unwrap()) };

  // Serialize
  let json_analysis_res: Result<String, String> = serde_json::to_string::<PairAnalysis>(&analysis)
    .map_err(|e| e.to_string());

  json_analysis_res
}


#[cfg(test)]
mod tests {
  use super::*;
  use crate::pricing::models::{DataCriteria, Exchange, IntervalPeriod};

  #[tokio::test]
  async fn it_performs_full_pair_analysis() {

    let asset_0: String = "BTCUSDT".to_string();
    let asset_1: String = "ETHUSDT".to_string();
    let exchange: Exchange = Exchange::Binance;
    let interval_period: IntervalPeriod = IntervalPeriod::Day(1, 1000);

    let data_criteria: DataCriteria = DataCriteria { 
      exchange, 
      asset_0, 
      asset_1, 
      interval_period
    };

    let analysis_criteria: AnalysisCriteria = AnalysisCriteria {
      data_criteria,
      stats_criteria: None,
      backtest_criteria: None
    };

    let json_input: String = serde_json::to_string::<AnalysisCriteria>(&analysis_criteria).unwrap();

    let analysis: String = wasm_full_pair_analysis_crypto(json_input).await.unwrap();

    let json_decoded: PairAnalysis = serde_json::from_str::<PairAnalysis>(&analysis).unwrap();
    assert!(json_decoded.bt_metrics.win_rate_stats.win_rate > 0.0);
    // dbg!(json_decoded.bt_metrics.win_rate_stats);
  }

  #[tokio::test]
  async fn it_extracts_single_quote() {
    let res = wasm_exchange_single_quote("Binance".to_string(), "BTCUSDT".to_string()).await.unwrap();
    dbg!(res);
  }

  #[tokio::test]
  async fn it_extracts_multi_symbol_quote() {
    let symbols: Vec<&str> = vec!["BTCUSDT", "ETHUSDT", "LINKUSDT"];
    let symbols_json: String = serde_json::to_string::<Vec<&str>>(&symbols).unwrap();
    let res = wasm_multi_symbol_quote("ByBit".to_string(), symbols_json).await.unwrap();
    dbg!(res);
  }

  #[tokio::test]
  async fn it_extracts_exchange_quotes() {
    let res = wasm_exchange_quotes().await.unwrap();
    dbg!(res);
  }

  #[tokio::test]
  async fn it_performs_backtest() {

    let asset_0: String = "ETCUSDT".to_string();
    let asset_1: String = "ETHUSDT".to_string();
    let exchange: Exchange = Exchange::Binance;
    let interval_period: IntervalPeriod = IntervalPeriod::Hour(1, 710);

    let data_criteria: DataCriteria = DataCriteria { 
      exchange, 
      asset_0, 
      asset_1, 
      interval_period
    };

    let prices: PairPrices = pair_prices(data_criteria, None).await.unwrap();
    let (spread, _) = spread_static_std(&prices.series_0, &prices.series_1).unwrap();
    let zscore = rolling_zscore(&spread, 35).unwrap();
    
    let bt_criteria: BacktestCriteria = BacktestCriteria {
      indicator_values: zscore,
      trigger_indicator: TriggerIndicator::Zscore,
      relation: Relation::Ignore,
      cost_per_leg: Some(0.0005),
      rets_weighting_s0_perc: 0.5,
      long_series: LongSeries::Series0,
      stop_loss: 0.0,
      long_thresh: -2.0,
      long_close_thresh: 0.0,
      short_thresh: 2.0,
      short_close_thresh: 0.0
    };

    dbg!(&bt_criteria.indicator_values.len());

    let pair_prices_json = serde_json::to_string(&prices).unwrap();
    let bt_criteria_json = serde_json::to_string(&bt_criteria).unwrap();;
    let res_json = wasm_quick_backtest(pair_prices_json, bt_criteria_json.to_string()).await.unwrap();
    let res = serde_json::from_str::<BacktestMetrics>(&res_json).unwrap();
    dbg!(res.equity_curve.len());
  }


}
