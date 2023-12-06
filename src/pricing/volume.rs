use crate::SmartError;
use super::models::Exchange;
use super::utils::api_request;

use std::collections::{HashMap, HashSet};

/// Get tickers url
/// Retrieves symbols url for a given exchange
fn get_tickers_url(exchange: &Exchange) -> Option<String> {

  let binance_tickers: &str = "https://fapi.binance.com/fapi/v1/ticker/24hr";
  let binance_us_tickers: &str = "https://api.binance.us/api/v3/ticker/24hr";
  let bybit_tickers: &str = "https://api.bybit.com/v5/market/tickers?category=linear";

  let url: &str = match exchange {
    Exchange::Binance => binance_tickers,
    Exchange::BinanceUs => binance_us_tickers,
    Exchange::ByBit => bybit_tickers,
    _ => return None
  };

  Some(url.to_string())
}

/// Extract High Volume Tickers Binance
/// Ranks tickers in order of volume traded
fn extract_high_volume_tickers_binance(json_text: String) -> Result<HashMap<i32, String>, SmartError> {
  let ticker_array: Vec<serde_json::Value> = serde_json::from_str(&json_text)?;
  let mut volume_map: HashMap<i32, String> = HashMap::new();
  for item in ticker_array {
    if let (Some(symbol), Some(quote_volume)) = (
      item.get("symbol").and_then(|s| s.as_str()),
      item.get("quoteVolume").and_then(|v| v.as_str()),
    ) {
      let total_vol: i32 = quote_volume.parse::<f32>().unwrap_or(0.0) as i32;
      volume_map.insert(total_vol, symbol.to_string());
    }
  }
  Ok(volume_map)
}

/// Extract High Volume Tickers ByBit
/// Ranks tickers in order of volume traded
fn extract_high_volume_tickers_bybit(json_text: String) -> Result<HashMap<i32, String>, SmartError> {
  let ticker_obj: serde_json::Value = serde_json::from_str(&json_text)?;
  let mut volume_map: HashMap<i32, String> = HashMap::new();
  if let Some(list) = ticker_obj.get("result").and_then(|r| r.get("list")) {
    if let Some(array) = list.as_array() {
      for item in array {
        if let (Some(symbol), Some(volume_24h), Some(last_price)) = (
            item.get("symbol").and_then(|s| s.as_str()), 
            item.get("volume24h").and_then(|v| v.as_str()),
            item.get("lastPrice").and_then(|l| l.as_str()),
          ) {
          
          let total_vol: f32 = (volume_24h.parse::<f32>().unwrap_or(1.0) * last_price.parse::<f32>().unwrap_or(1.0)) / 1000.0;
          volume_map.insert(total_vol as i32, symbol.to_string());
        }
      }
    }
  }
  Ok(volume_map)
}

/// Request High Volume Tickers
/// Requests list of available tickers for a given exchange
pub async fn request_high_volume_tickers(exchange: &Exchange) -> Result<Vec<String>, SmartError> {

  // Handle if exchange is Twelve and thus assumed to want forex for high volume tickers
  if exchange == &Exchange::Twelve { // AUDCHF GBPCAD GBPCAD 
    let currencies = vec![
      "USD/JPY", "USD/EUR", "USD/AUD", "USD/GBP", "USD/CHF", "USD/CAD", "EUR/GBP", "EUR/CHF", "EUR/JPY", 
      "AUD/CAD", "EUR/AUD", "GBP/AUD", "CAD/CHF", "AUD/CHF", "GBP/CAD", "EUR/NZD"
    ];

    return Ok(currencies.iter().map(|c| c.to_string()).collect())
  }

  // Initialize url
  let request_url: String = get_tickers_url(&exchange).expect("exchange volume information not available");

  // Make request
  let res_data: reqwest::Response = api_request(&request_url).await?;

  // Guard: Ensure status code
  if res_data.status() != 200 {
    let e: String = format!("Failed to extract data: {:?}", res_data.text().await);
    return Err(SmartError::APIResponseStatus(e));
  }

  // Send JSON
  let json_text: String = res_data.text().await?;

  let tickers_hm: HashMap<i32, String> = match exchange {
    Exchange::Binance => extract_high_volume_tickers_binance(json_text)?,
    Exchange::BinanceUs => extract_high_volume_tickers_binance(json_text)?,
    Exchange::ByBit => extract_high_volume_tickers_bybit(json_text)?,
    _ => panic!("should only include Binance, BinanceUs and ByBit")
  };

  let mut sorted: Vec<_> = tickers_hm.iter().collect();
  sorted.sort_by_key(|a| a.0);
  sorted.reverse();
  
  // Condense into standalone symbol only for top 25% volume traded
  let mut symbols: Vec<String> = vec![];
  let breakpoint: usize = (sorted.len() as f32 * 0.25) as usize;
  for (i, symbol) in sorted.iter().enumerate() {
    if i >= breakpoint { break; }
    let standalone = symbol.1.replace("USDT", "");
    let standalone = standalone.replace("USDC", "");
    symbols.push(standalone);
  }

  Ok(symbols)
}

/// Request All High Volume Tickers
/// Requests list of available tickers for a given exchange
pub async fn request_high_volume_tickers_all() -> Result<Vec<String>, SmartError> {
  let mut all_tickers: Vec<String> = vec![];
  let binance_res = request_high_volume_tickers(&Exchange::Binance).await;
  let binance_us_res = request_high_volume_tickers(&Exchange::BinanceUs).await;
  let bybit_res = request_high_volume_tickers(&Exchange::ByBit).await;
  let twelve_res = request_high_volume_tickers(&Exchange::Twelve).await;
  
  if let Ok(binance) = binance_res { all_tickers.extend(binance); }
  if let Ok(binance_us) = binance_us_res { all_tickers.extend(binance_us); }
  if let Ok(bybit) = bybit_res { all_tickers.extend(bybit); }
  if let Ok(twelve) = twelve_res { all_tickers.extend(twelve); }

  let unique_values: HashSet<String> = all_tickers.into_iter().collect();
  let tickers_without_duplicates: Vec<String> = unique_values.into_iter().collect();
  Ok(tickers_without_duplicates)
}


#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn it_combines_all_known_high_volume_symbols() {
    let tickers: Vec<String> = request_high_volume_tickers_all().await.unwrap();
    dbg!(&tickers);
    assert!(tickers.len() > 0);
  }
}