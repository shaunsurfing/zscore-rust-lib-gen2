use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

use crate::SmartError;

use super::symbols::request_symbols;

/*
  Entry Models
*/

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct DataCriteria {
  pub exchange: Exchange,
  pub asset_0: String,
  pub asset_1: String,
  pub interval_period: IntervalPeriod
}

/*
  Quote Models
*/

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct QuoteExch {
  pub binance: f64,
  pub binance_us: f64,
  pub bybit: f64,
  pub coinbase: f64,
  pub dydx: f64,
  pub twelve: f64,
}

/*
  Candles Models
*/

/// Interval
/// Value = Interval
/// (u8 = number in interval)
/// (u16 = period in days)
#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub enum IntervalPeriod {
  Min(u8, u32), // interval, period in minutes
  Hour(u8, u32), // interval, period in hours
  Day(u8, u32), // interval, period in days
}

impl IntervalPeriod {
  pub fn as_string(&self) -> String {
    match &self {
      Self::Min(x, y) => format!("[Min][{},{}]", x, y),
      Self::Hour(x, y) => format!("[Hour][{},{}]", x, y),
      Self::Day(x, y) => format!("[Day][{},{}]", x, y),
    }
  }
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct CallItem {
  pub from_time: i64,
  pub to_time: i64
}


#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct ExchInt { 
  pub exchange_str: String, 
  pub default_period: u32
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, TS)]
#[ts(export)]
pub enum AssetType {
  Crypto,
  Etf,
  Forex,
  Indices,
  Stock
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, TS)]
#[ts(export)]

pub enum Exchange {
  Binance,
  BinanceUs,
  ByBit,
  Coinbase,
  Dydx,
  Twelve
}

impl Exchange {
  pub fn create_from_string(exchange_str: &str) -> Self {
    match exchange_str {
      "Binance" => Exchange::Binance,
      "BinanceUs" => Exchange::BinanceUs,
      "Coinbase" => Exchange::Coinbase,
      "Dydx" => Exchange::Dydx,
      "Fmp" => Exchange::Twelve,
      _ => panic!("Incorrect or unknown exchange")
    }
  }

  pub fn as_string(&self) -> String {
    match self {
      Exchange::Binance => "Binance".to_string(),
      Exchange::BinanceUs => "BinanceUs".to_string(),
      Exchange::ByBit => "ByBit".to_string(),
      Exchange::Coinbase => "Coinbase".to_string(),
      Exchange::Dydx => "Dydx".to_string(),
      Exchange::Twelve => "Twelve".to_string()
    }
  }

  /// Default Ticker Assets
  pub fn default_assets(&self) -> (String, String) {
    let asset_1: String = match self {
      Exchange::Binance | Exchange::BinanceUs | Exchange::ByBit => "BTCUSDT".to_string(),
      Exchange::Coinbase | Exchange::Dydx  => "BTC-USD".to_string(),
      Exchange::Twelve  => "USD/GBP".to_string()
    };

    let asset_2: String = match self {
      Exchange::Binance | Exchange::BinanceUs | Exchange::ByBit  => "ETHUSDT".to_string(),
      Exchange::Coinbase | Exchange::Dydx  => "ETH-USD".to_string(),
      Exchange::Twelve  => "USD/GBP".to_string()
    };

    (asset_1, asset_2)
  }

  /// Default Interval Period
  pub fn default_interval_period(&self) -> IntervalPeriod {
    IntervalPeriod::Hour(1, 700)
  }

  /// Available Assets
  pub async fn available_assets(&self, asset_type: Option<AssetType>) -> Result<Vec<String>, SmartError> {
    let available_assets: Vec<String> = request_symbols(&self, asset_type).await?;
    Ok(available_assets)
  }

  /// Available Intervals
  pub fn available_intervals(&self, default_period: u32) -> HashMap<&str, IntervalPeriod> {
    let mut intervals_hm: HashMap<&str, IntervalPeriod> = HashMap::new();

    match self {
      Exchange::Binance | Exchange::BinanceUs => {
        intervals_hm.insert("5min", IntervalPeriod::Min(5, default_period));
        intervals_hm.insert("15min", IntervalPeriod::Min(15, default_period));
        intervals_hm.insert("30min", IntervalPeriod::Min(30, default_period));
        intervals_hm.insert("1hour", IntervalPeriod::Hour(1, default_period));
        intervals_hm.insert("2hour", IntervalPeriod::Hour(2, default_period));
        intervals_hm.insert("4hour", IntervalPeriod::Hour(4, default_period));
        intervals_hm.insert("6hour", IntervalPeriod::Hour(6, default_period));
        intervals_hm.insert("8hour", IntervalPeriod::Hour(8, default_period));
        intervals_hm.insert("12hour", IntervalPeriod::Hour(12, default_period));
        intervals_hm.insert("1day", IntervalPeriod::Day(1, default_period));
      },
      Exchange::ByBit => {
        intervals_hm.insert("5", IntervalPeriod::Min(5, default_period));
        intervals_hm.insert("15", IntervalPeriod::Min(15, default_period));
        intervals_hm.insert("30", IntervalPeriod::Min(30, default_period));
        intervals_hm.insert("60", IntervalPeriod::Hour(1, default_period));
        intervals_hm.insert("120", IntervalPeriod::Hour(2, default_period));
        intervals_hm.insert("240", IntervalPeriod::Hour(4, default_period));
        intervals_hm.insert("360", IntervalPeriod::Hour(6, default_period));
        intervals_hm.insert("720", IntervalPeriod::Hour(12, default_period));
        intervals_hm.insert("D", IntervalPeriod::Day(1, default_period));
      },
      Exchange::Coinbase => {
        intervals_hm.insert("5min", IntervalPeriod::Min(5, default_period));
        intervals_hm.insert("15min", IntervalPeriod::Min(15, default_period));
        intervals_hm.insert("1hour", IntervalPeriod::Hour(1, default_period));
        intervals_hm.insert("6hour", IntervalPeriod::Hour(6, default_period));
        intervals_hm.insert("1day", IntervalPeriod::Day(1, default_period));
      },
      Exchange::Dydx => {
        intervals_hm.insert("5MIN", IntervalPeriod::Min(5, default_period));
        intervals_hm.insert("15MINS", IntervalPeriod::Min(15, default_period));
        intervals_hm.insert("30MINS", IntervalPeriod::Min(30, default_period));
        intervals_hm.insert("1HOUR", IntervalPeriod::Hour(1, default_period));
        intervals_hm.insert("4HOURS", IntervalPeriod::Hour(4, default_period));
        intervals_hm.insert("1DAY", IntervalPeriod::Day(1, default_period));
      },
      Exchange::Twelve => {
        intervals_hm.insert("5m", IntervalPeriod::Min(5, default_period));
        intervals_hm.insert("15min", IntervalPeriod::Min(15, default_period));
        intervals_hm.insert("30min", IntervalPeriod::Min(30, default_period));
        intervals_hm.insert("1h", IntervalPeriod::Hour(1, default_period));
        intervals_hm.insert("2h", IntervalPeriod::Hour(2, default_period));
        intervals_hm.insert("4h", IntervalPeriod::Hour(4, default_period));
        intervals_hm.insert("1day", IntervalPeriod::Day(1, default_period));
      }
    };
    intervals_hm
  }

}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct DydxCandleObj {
  pub baseTokenVolume: String,
  pub close: String,
  pub high: String,
  pub low: String,
  pub market: String,
  pub open: String,
  pub resolution: String,
  pub startedAt: String,
  pub startingOpenInterest: String,
  pub trades: String,
  pub updatedAt: String,
  pub usdVolume: String,
}

#[derive(Debug, Deserialize)]
pub struct DydxCandle {
  pub candles: Vec<DydxCandleObj>
}

/*
  Price Controller Models
*/

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct HistoricalPrices {
  pub prices: Vec<f64>,
  pub labels: Vec<u64>
}

#[derive(Debug, Deserialize, Serialize, Clone, TS)]
#[ts(export)]
pub struct PairPrices {
  pub series_0: Vec<f64>,
  pub series_1: Vec<f64>,
  pub labels: Vec<u64> 
}