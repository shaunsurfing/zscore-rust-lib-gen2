
use crate::SmartError;
use super::utils::{api_request, sleep};
use super::times::{get_world_time_utc, subtract_time, convert_timestamp_to_iso, convert_iso_to_timestamp};
use super::models::{Exchange, DydxCandle, IntervalPeriod, HistoricalPrices, CallItem};

/// API DOCUMENTATION:
/// Binance: https://binance-docs.github.io/apidocs/futures/en/#change-log
/// BinanceUs: https://docs.binance.us/#get-order-book-depth
/// ByBit: https://bybit-exchange.github.io/docs/api-explorer/v5/market/kline
/// Coinbase: https://docs.cloud.coinbase.com/exchange/reference/
/// Dydx: https://dydxprotocol.github.io/v3-teacher/#public-http-api
/// Twelve: https://twelvedata.com/docs

/*
  Price Builder Models
  These are used to allow for url structuring and querying
*/

#[derive(Debug)]
pub struct CandleBuilder {
  pub symbol: String,
  pub interval: IntervalPeriod,
  pub exchange: Exchange,
  pub max_limit: i64,
  pub query_url: String
}

impl CandleBuilder {
  pub fn new(
    symbol: String, 
    interval: IntervalPeriod, 
    exchange: Exchange,
    twelve_api_key: Option<&str>
  ) -> Self {
    let max_limit: i64 = Self::get_max_limit(&exchange);

    let query_url: String = match exchange {
      Exchange::Binance => "https://fapi.binance.com/fapi/v1/klines?symbol={symbol}&interval={interval}&startTime={fromTime}&endTime={toTime}&limit={limit}".to_string(), // Limit 1000
      Exchange::BinanceUs => "https://api.binance.us/api/v3/klines?symbol={symbol}&interval={interval}&startTime={fromTime}&endTime={toTime}&limit={limit}".to_string(), // Limit 1000
      Exchange::ByBit => "https://api.bybit.com/v5/market/kline?category=linear&symbol={symbol}&interval={interval}&start={fromTime}&end={toTime}&limit={limit}".to_string(), // Limit 200
      Exchange::Coinbase => "https://api.exchange.coinbase.com/products/{symbol}/candles?granularity={interval}&start={fromTime}&end={toTime}".to_string(), // Limit 300
      Exchange::Dydx => "https://api.dydx.exchange/v3/candles/{symbol}?resolution={interval}&fromISO={fromTime}&toISO={toTime}&limit={limit}".to_string(), // Limit 100
      Exchange::Twelve => {
        match twelve_api_key {
          Some(api_key) => {
            let base_url: &str = "https://api.twelvedata.com/time_series?interval={interval}&symbol={symbol}&start_date={fromTime}&end_date={toTime}&outputsize={limit}&timezone=utc"; // Limit 5000
            format!("{}&apikey={}", base_url, api_key)
          },
          None => panic!("Must provide an API key for Twelve provider")
        }
      }
    };

    Self {
      symbol,
      interval,
      exchange,
      max_limit,
      query_url
    }
  }

  /// Get Max Limit
  /// Identifies max rows to be returned given exchange
  pub fn get_max_limit(exchange: &Exchange) -> i64 {

    // Buffer used to ensure adequite coverage of from and to times
    // Any duplicates will be removed subsequently
    let buffer: i64 = 5;

    match exchange {
      Exchange::Binance | Exchange::BinanceUs => 1000 - buffer,
      Exchange::ByBit => 200 - buffer,
      Exchange::Coinbase => 300 - buffer,
      Exchange::Dydx => 100 - buffer,
      Exchange::Twelve => 5000 - buffer
    }
  }

  /// Getters
  pub fn get_symbol(&self) -> String { self.symbol.clone() }
  pub fn get_interval(&self) -> IntervalPeriod { self.interval.clone() }
  pub fn get_exchange(&self) -> Exchange { self.exchange.clone() }

  /// Retrieve Request URL
  /// Retrieve the base url for making respective call
  fn get_request_url(&self) -> String {
    self.query_url.clone()
  }

  /// Structure Interval
  /// Converts Interval details into exchange readable str
  fn structure_interval<'a>(&self) -> Result<&'a str, SmartError> {
    use Exchange::{Binance, BinanceUs, ByBit, Coinbase, Dydx, Twelve};
    use IntervalPeriod::{Min, Hour, Day};

    let interval: &str = match (&self.exchange, &self.interval) {
      (Binance | BinanceUs, Min(int, _)) if *int == 5 => "5m",
      (Binance | BinanceUs, Min(int, _)) if *int == 15 => "15m",
      (Binance | BinanceUs, Min(int, _)) if *int == 30 => "30m",
      (Binance | BinanceUs, Hour(int, _)) if *int == 1 => "1h",
      (Binance | BinanceUs, Hour(int, _)) if *int == 2 => "2h",
      (Binance | BinanceUs, Hour(int, _)) if *int == 4 => "4h",
      (Binance | BinanceUs, Hour(int, _)) if *int == 6 => "6h",
      (Binance | BinanceUs, Hour(int, _)) if *int == 8 => "8h",
      (Binance | BinanceUs, Hour(int, _)) if *int == 12 => "12h",
      (Binance | BinanceUs, Day(int, _)) if *int == 1 => "1d",

      (ByBit, Min(int, _)) if *int == 5 => "5",
      (ByBit, Min(int, _)) if *int == 15 => "15",
      (ByBit, Min(int, _)) if *int == 30 => "30",
      (ByBit, Hour(int, _)) if *int == 1 => "60",
      (ByBit, Hour(int, _)) if *int == 2 => "120",
      (ByBit, Hour(int, _)) if *int == 4 => "240",
      (ByBit, Hour(int, _)) if *int == 6 => "360",
      (ByBit, Hour(int, _)) if *int == 12 => "720",
      (ByBit, Day(int, _)) if *int == 1 => "D",

      (Coinbase, Min(int, _)) if *int == 5 => "300",
      (Coinbase, Min(int, _)) if *int == 15 => "900",
      (Coinbase, Hour(int, _)) if *int == 1 => "3600",
      (Coinbase, Hour(int, _)) if *int == 6 => "21600",
      (Coinbase, Day(int, _)) if *int == 1 => "86400",

      (Dydx, Min(int, _)) if *int == 5 => "5MINS",
      (Dydx, Min(int, _)) if *int == 15 => "15MINS",
      (Dydx, Min(int, _)) if *int == 30 => "30MINS",
      (Dydx, Hour(int, _)) if *int == 1 => "1HOUR",
      (Dydx, Hour(int, _)) if *int == 4 => "4HOURS",
      (Dydx, Day(int, _)) if *int == 1 => "1DAY",

      (Twelve, Min(int, _)) if *int == 5 => "5min",
      (Twelve, Min(int, _)) if *int == 15 => "15min",
      (Twelve, Min(int, _)) if *int == 30 => "30min",
      (Twelve, Hour(int, _)) if *int == 1 => "1h",
      (Twelve, Hour(int, _)) if *int == 2 => "2h",
      (Twelve, Hour(int, _)) if *int == 4 => "4h",
      (Twelve, Day(int, _)) if *int == 1 => "1day",

      _ => return Err(SmartError::RuntimeCheck("Interval exchange match not found".to_string()))
    };

    Ok(interval)
  }

  /// Calculates two items for call count needed
  /// First Item: The number of calls required at the max limit
  /// Second Item: The final amount of rows required on the last call
  fn calculate_call_count(&self) -> (usize, i64) {

    let total_factor: f32 = match self.interval {
      IntervalPeriod::Min(int, minutes) => {
        minutes as f32 / int as f32
      },
      IntervalPeriod::Hour(int, hours) => {
        hours as f32 / int as f32
      },
      IntervalPeriod::Day(int, days) => {
        days as f32 / int as f32
      }
    };

    // Calculate call count required given calls needed and max limit
    let call_ratio: f32 = total_factor / self.max_limit as f32;
    let iterations: usize = call_ratio.floor() as usize;
    let final_n: f32 = call_ratio.rem_euclid(1.0) * self.max_limit as f32;

    // Return iterations and final n
    (iterations, final_n as i64)
  }

  /// Set Calls Required as Vector
  /// Structures vector of times required
  pub async fn calls_required(&self) -> Result<Vec<CallItem>, SmartError> {

    // Initialize
    let mut call_items: Vec<CallItem> = vec![];
    let (iterations, final_n) = self.calculate_call_count();

    // Set end time
    let unix_time: i64 = get_world_time_utc()?;
    let mut end_time: i64 = subtract_time(unix_time, &self.interval, &0);

    // Structure times
    for _ in 0..iterations {
      let start_time: i64 = subtract_time(end_time, &self.interval, &self.max_limit);
      let call_item: CallItem = CallItem {
        from_time: start_time,
        to_time: end_time,
      };

      call_items.push(call_item);
      end_time = start_time;
    }

    // Add final number if less than max required
    if final_n > 0 {
      let start_time: i64 = subtract_time(end_time, &self.interval, &final_n);
      let call_item: CallItem = CallItem {
        from_time: start_time,
        to_time: end_time,
      };
      call_items.push(call_item);
    }

    // Reverse times
    call_items.reverse();
    Ok(call_items)
  }

  /// Format call times
  /// Format call times depending on exchange
  fn format_call_times(&self, timestamp: i64, is_offset: bool) -> String {
    use Exchange::{Binance, BinanceUs, ByBit, Coinbase, Dydx, Twelve};

    // Offset to ensure adequate coverage of from and to times
    // Different exchanges provide different coverage depending on times
    // Therefore, providing more than needed and then removing duplicates later on
    let offset: i64 = if is_offset { 10 } else { 0 };

    match self.exchange {
      Binance | BinanceUs | ByBit => {
        let new_timestamp: i64 = timestamp * 1000;
        new_timestamp.to_string()
      },
      Coinbase => timestamp.to_string(),
      Dydx => convert_timestamp_to_iso(timestamp - offset),
      Twelve => timestamp.to_string()
    }
  }

  /// Remove duplicate candles
  /// Removes any duplicate candles depending on exchange quirks
  fn remove_duplicates(&self, labels: &mut Vec<u64>, prices: &mut Vec<f64>) {
    let mut indices_to_remove: Vec<usize> = vec![];

    // Start from the end
    // Removing elements from the beginning would shift the remaining indices
    let len = labels.len();
    for i in (1..len).rev() {
        if labels[i] == labels[i-1] {
            indices_to_remove.push(i);
        }
    }

    // Remove identified indices
    for &index in indices_to_remove.iter() {
        labels.remove(index);
        prices.remove(index);
    }
  }

  /// Deserialize Candles - Binance
  /// Deserializes candles into time labels and prices - Binance
  async fn deserialize_candles_binance(&self, res_data: reqwest::Response) -> Result<(Vec<u64>, Vec<f64>), SmartError>  {
    let candles_json: Vec<serde_json::Value> = res_data.json().await?;
    let mut prices: Vec<f64> = vec![];
    let mut labels: Vec<u64> = vec![];
    for candle in candles_json.iter() {
      let close: f64 = candle[4].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
      let label: u64 = match candle[0].as_u64() {
        Some(val) => val / 1000,
        None => 0
      };
      prices.push(close);
      labels.push(label);
    }
    Ok((labels, prices))
  }

  /// Deserialize Candles - ByBit
  /// Deserializes candles into time labels and prices - ByBit
  async fn deserialize_candles_bybit(&self, res_data: reqwest::Response) -> Result<(Vec<u64>, Vec<f64>), SmartError>  {
    let candles_json: serde_json::Value = res_data.json().await?;
    let mut prices: Vec<f64> = vec![];
    let mut labels: Vec<u64> = vec![];
    if let Some(candles_json) = candles_json.get("result").and_then(|res| res.get("list")).and_then(|list| list.as_array()) {
      for candle in candles_json.iter() {
        if let Some(candle_array) = candle.as_array() {
          let close: f64 = candle_array.get(4).and_then(|s| s.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);
          let label: u64 = candle_array.get(0).and_then(|s| s.as_str()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0) / 1000;
          
          prices.push(close);
          labels.push(label);
        }
      }
    }
    labels.reverse();
    prices.reverse();
    Ok((labels, prices))
  }

  /// Deserialize API Response - Coinbase
  /// Deserializes candles into time labels and prices - Coinbase
  async fn deserialize_candles_coinbase(&self, res_data: reqwest::Response) -> Result<(Vec<u64>, Vec<f64>), SmartError>  {
    let candles_json: Vec<serde_json::Value> = res_data.json().await?;
    let mut prices: Vec<f64> = vec![];
    let mut labels: Vec<u64> = vec![];
    for candle in candles_json.iter() {
      let close: f64 = match candle[4].as_f64() {
        Some(val) => val,
        None => 0.0
      };
      let label: u64 = match candle[0].as_u64() {
        Some(val) => val,
        None => 0
      };
      prices.push(close);
      labels.push(label);
    }
    labels.reverse();
    prices.reverse();
    Ok((labels, prices))
  }

  /// Deserialize API Response - Dydx
  /// Deserializes candles into time labels and prices - Dydx
  async fn deserialize_candles_dydx(&self, res_data: reqwest::Response) -> Result<(Vec<u64>, Vec<f64>), SmartError>  {
    let candles_json: serde_json::Value = res_data.json().await?;
    let candles: DydxCandle = serde_json::from_value(candles_json)?;

    let mut prices: Vec<f64> = vec![];
    let mut labels: Vec<u64> = vec![];

    for candle in candles.candles {
      let close: f64 = candle.close.parse()?;
      let label_str: String = candle.startedAt;
      let label: u64 = convert_iso_to_timestamp(label_str, "%Y-%m-%dT%H:%M:%S%.3f%z");
      prices.push(close);
      labels.push(label);
    }

    labels.reverse();
    prices.reverse();

    Ok((labels, prices))
  }

  /// Deserialize API Response - Twelve
  /// Deserializes candles into time labels and prices - Coinbase
  async fn deserialize_candles_twelve(&self, res_data: reqwest::Response) -> Result<(Vec<u64>, Vec<f64>), SmartError>  {
    let data: serde_json::Value = res_data.json().await?;
    let mut prices: Vec<f64> = vec![];
    let mut labels: Vec<u64> = vec![];
    if let Some(values) = data.get("values") {
      for value in values.as_array().unwrap() {
        let close: f64 = match value["close"].as_str() {
          Some(val) => val.parse().unwrap_or(0.0),
          None => 0.0
        };
        let label_str: String = match value["datetime"].as_str() {
          Some(val) => val.to_string(),
          None => "".to_string()
        };

        let label: u64 = convert_iso_to_timestamp(label_str, "%Y-%m-%dT%H:%M:%S%z");
        prices.push(close);
        labels.push(label);
      }
    }
    labels.reverse();
    prices.reverse();
    Ok((labels, prices))
}

  /// Deserialize API Response based on exchange
  /// Deserializes the API response into a price array
  async fn deserialize_api_response_candles(&self, res_data: reqwest::Response) -> Result<(Vec<u64>, Vec<f64>), SmartError> {
    let (labels, prices) = match self.exchange {
      Exchange::Binance | Exchange::BinanceUs => self.deserialize_candles_binance(res_data).await?,
      Exchange::ByBit => self.deserialize_candles_bybit(res_data).await?,
      Exchange::Coinbase => self.deserialize_candles_coinbase(res_data).await?,
      Exchange::Dydx => self.deserialize_candles_dydx(res_data).await?,
      Exchange::Twelve => self.deserialize_candles_twelve(res_data).await?
    };

    Ok((labels, prices))
  }

  /// Fetch Prices - candles
  /// Retrieves prices required for candles
  pub async fn fetch_prices_candles(&self) -> Result<HistoricalPrices, SmartError> {

    // Get request_url
    let mut request_url: String = self.get_request_url();

    // Structure interval
    let interval_str: &str = self.structure_interval()?;

    // Extract max limit
    let max_limit: String = Self::get_max_limit(&self.exchange).to_string();
    
    // Replace url placeholders
    request_url = request_url.replace("{symbol}", &self.symbol);
    request_url = request_url.replace("{interval}", interval_str);
    request_url = request_url.replace("{limit}", &max_limit);

    // Get calls required
    let calls_required: Vec<CallItem> = self.calls_required().await?;

    // Make API calls
    let mut url: String;
    let mut labels_full: Vec<u64> = vec![];
    let mut prices_full: Vec<f64> = vec![];
    let mut call_count:u8 = 0;
    for call in calls_required {

      // Handle sleeping - protects API rate limit usage
      call_count += 1;
      match call_count {
        1..=2 => sleep(50).await,
        3..=7 => sleep(500).await,
        8..=12 => sleep(1000).await,
        13..=20 => sleep(2000).await,
        _ => { break; }
      };

      // Update from and to intervals
      let from_time: String = self.format_call_times(call.from_time, true);
      let to_time: String = self.format_call_times(call.to_time, false);
      
      // Update url
      url = request_url.replace("{fromTime}", &from_time).to_string();
      url = url.replace("{toTime}", &to_time).to_string();

      // Make request
      let res_data: reqwest::Response = api_request(&url).await?;

      // Guard: Ensure status code
      if res_data.status() != 200 {
        let e: String = format!("Failed to extract data: {:?}", res_data.text().await);
        return Err(SmartError::APIResponseStatus(e));
      }

      // Decode and append response
      let (mut labels, mut prices) = self.deserialize_api_response_candles(res_data).await?;
      labels_full.append(&mut labels);
      prices_full.append(&mut prices);
    };
    
    // Remove duplicates (if any)
    self.remove_duplicates(&mut labels_full, &mut prices_full);

    // Return labels and prices
    let prices = HistoricalPrices {
      labels: labels_full,
      prices: prices_full
    };
    Ok(prices)
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashSet;

  // Test consistency of time intervals
  fn test_label_consistency(labels: &Vec<u64>) -> bool {
    let mut label_set: HashSet<u64> = HashSet::new();
    for i in 1..labels.len() {
      let distance: u64 = labels[i] - labels[i - 1];
      if distance > 3600 {
        // dbg!(i, labels[i], labels[i - 1], distance);
        if i < labels.len() - 20 {
          // dbg!(&labels[i - 1..i + 19]);
        }
      }
      label_set.insert(distance);
    }
    label_set.len() == 1
  }

  fn structure_candle_builder(exchange: Exchange, symbol: &str, api_key: Option<&str>) -> CandleBuilder {
    let symbol: String = symbol.to_string();
    let interval: u8 = 1;
    let interval_count: u32 = 200;
    let interval_period: IntervalPeriod = IntervalPeriod::Hour(interval, interval_count);
    CandleBuilder::new(symbol, interval_period, exchange, api_key)
  }

  #[tokio::test]
  async fn tests_calculate_call_count() {
    let price_builder: CandleBuilder = structure_candle_builder(Exchange::Dydx, "BTCUSDT", None);
    let (iterations, final_n) = price_builder.calculate_call_count();
    assert!(iterations > 0);
    assert!(final_n > 0);
  }

  #[tokio::test]
  async fn tests_calls_required() {
    let price_builder: CandleBuilder = structure_candle_builder(Exchange::Binance, "BTCUSDT", None);
    let calls_required: Vec<CallItem> = price_builder.calls_required().await.unwrap();
    assert!(calls_required.len() > 0);
  }

  #[tokio::test]
  async fn tests_fetch_prices_binance() {
    let price_builder: CandleBuilder = structure_candle_builder(Exchange::Binance, "BTCUSDT", None);
    let hist_prices: HistoricalPrices = price_builder.fetch_prices_candles().await.unwrap();
    assert!(hist_prices.labels.len() > 0 && hist_prices.prices.len() > 0);
    let consistency: bool = test_label_consistency(&hist_prices.labels);
    assert!(consistency);
  }

  #[tokio::test]
  async fn tests_fetch_prices_binance_us() {
    let price_builder: CandleBuilder = structure_candle_builder(Exchange::BinanceUs, "BTCUSDT", None);
    let hist_prices: HistoricalPrices = price_builder.fetch_prices_candles().await.unwrap();
    assert!(hist_prices.labels.len() > 0 && hist_prices.prices.len() > 0);
    let consistency: bool = test_label_consistency(&hist_prices.labels);
    assert!(consistency);
  }

  #[tokio::test]
  async fn tests_fetch_prices_bybit() {
    let price_builder: CandleBuilder = structure_candle_builder(Exchange::ByBit, "BTCUSDT", None);
    let hist_prices: HistoricalPrices = price_builder.fetch_prices_candles().await.unwrap();
    assert!(hist_prices.labels.len() > 0 && hist_prices.prices.len() > 0);
    let consistency: bool = test_label_consistency(&hist_prices.labels);
    assert!(consistency);
  }

  #[tokio::test]
  async fn tests_fetch_prices_coinbase() {
    let price_builder: CandleBuilder = structure_candle_builder(Exchange::Coinbase, "BTC-USD", None);
    let hist_prices: HistoricalPrices = price_builder.fetch_prices_candles().await.unwrap();
    assert!(hist_prices.labels.len() > 0 && hist_prices.prices.len() > 0);
    let consistency: bool = test_label_consistency(&hist_prices.labels);
    assert!(consistency);
  }

  #[tokio::test]
  async fn tests_fetch_prices_dydx() {
    let price_builder: CandleBuilder = structure_candle_builder(Exchange::Dydx, "BTC-USD", None);
    let hist_prices: HistoricalPrices = price_builder.fetch_prices_candles().await.unwrap();
    assert!(hist_prices.labels.len() > 0 && hist_prices.prices.len() > 0);
    let consistency: bool = test_label_consistency(&hist_prices.labels);
    assert!(consistency);
  }

  #[tokio::test]
  async fn tests_fetch_prices_twelve() {
    use dotenv::dotenv;
    use std::env;
    dotenv().ok();

    let api_key: String = match env::var("TWELVE_API_KEY") {
      Ok(val) => val,
      Err(_e) => panic!("Failed to read TWELVE_API_KEY"),
    };

    let price_builder: CandleBuilder = structure_candle_builder(Exchange::Twelve, "USD/GBP", Some(&api_key));
    let hist_prices: HistoricalPrices = price_builder.fetch_prices_candles().await.unwrap();
    assert!(hist_prices.labels.len() > 0 && hist_prices.prices.len() > 0);
  }
}