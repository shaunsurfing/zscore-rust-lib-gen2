use std::str::FromStr;

use crate::SmartError;
use super::models::{Exchange, AssetType};
use super::utils::api_request;


/// Get symbols url
/// Retrieves symbols url for a given exchange
fn get_symbols_url(exchange: &Exchange, asset_type: Option<AssetType>) -> String {

  let binance_symbols: &str = "https://fapi.binance.com/fapi/v1/exchangeInfo";
  let binance_us_symbols: &str = "https://api.binance.us/api/v3/exchangeInfo";
  let bybit_symbols: &str = "https://api.bybit.com/v5/market/instruments-info?category=linear";
  let coinbase_symbols: &str = "https://api.exchange.coinbase.com/products";
  let dydx_symbols: &str = "https://api.dydx.exchange/v3/markets";

  let twelve_symbols: &str = match asset_type {
    Some(t) => match t {
      AssetType::Crypto => "https://api.twelvedata.com/cryptocurrencies",
      AssetType::Etf => "https://api.twelvedata.com/etf",
      AssetType::Forex => "https://api.twelvedata.com/forex_pairs",
      AssetType::Stock => "https://api.twelvedata.com/stocks",
      AssetType::Indices => "https://api.twelvedata.com/indices",
    },
    None => "https://api.twelvedata.com/forex_pairs"
  };

  match exchange {
    Exchange::Binance => binance_symbols.to_string(),
    Exchange::BinanceUs => binance_us_symbols.to_string(),
    Exchange::ByBit => bybit_symbols.to_string(),
    Exchange::Coinbase => coinbase_symbols.to_string(),
    Exchange::Dydx => dydx_symbols.to_string(),
    Exchange::Twelve => twelve_symbols.to_string(),
  }
}

/// Extract Symbols Binance
/// Takes Binance data and returns vector of api endpoints
fn extract_symbols_binance(json_text: String) -> Result<Vec<String>, SmartError> {
  let ticker_obj: serde_json::Value = serde_json::Value::from_str(&json_text)?;

  let mut tickers: Vec<String> = ticker_obj["symbols"]
    .as_array()
    .ok_or(SmartError::RuntimeCheck("Expected 'symbols' to be an array".to_string()))?
    .iter()
    .filter_map(|symbol_obj| {
      if symbol_obj["status"].as_str() == Some("TRADING") {
        symbol_obj["symbol"].as_str().map(|s| s.to_string())
      } else {
        None
      }
    })
    .collect();

  tickers.sort();
  Ok(tickers)
}

/// Extract Symbols ByBit
/// Retrieves tickers for ByBit
fn extract_symbols_bybit(json_text: String) -> Result<Vec<String>, SmartError> {
  let ticker_obj: serde_json::Value = serde_json::from_str(&json_text)?;

  // Navigating to the list within result.
  let list = ticker_obj.get("result")
    .and_then(|result| result.get("list"))
    .and_then(|list| list.as_array())
    .ok_or("Expected 'result.list' to be an array")
    .map_err(|e| SmartError::RuntimeCheck(e.to_string()))?;

  // Extracting symbols where status is "Trading".
  let tickers: Vec<String> = list.iter()
      .filter_map(|item| {
        item.get("status")
          .and_then(serde_json::Value::as_str)
          .and_then(|status| {
            if status == "Trading" {
              item.get("symbol").and_then(serde_json::Value::as_str).map(ToString::to_string)
            } else {
              None
            }
          })
      })
      .collect();

  Ok(tickers)
}

/// Extract Symbols Coinbase
/// Takes Coinbase data and returns vector of api endpoints
fn extract_symbols_coinbase(json_text: String) -> Result<Vec<String>, SmartError> {
  let ticker_obj: serde_json::Value = serde_json::Value::from_str(&json_text)?;
  let ticker_array: &Vec<serde_json::Value> = ticker_obj
    .as_array()
    .ok_or(SmartError::RuntimeCheck("Expected an array".to_string()))?;

  // Extract 'id' from each object in the array
  let tickers: Vec<String> = ticker_array
    .iter()
    .filter_map(|item| item["id"].as_str())
    .map(|s| s.to_string())
    .collect();

  Ok(tickers)
}

/// Extract Symbols Dydx
/// Takes Dydx data and returns vector of api endpoints
fn extract_symbols_dydx(json_text: String) -> Result<Vec<String>, SmartError> {
  let ticker_obj: serde_json::Value = serde_json::Value::from_str(&json_text)?;
  let markets_obj = ticker_obj["markets"]
    .as_object()
    .ok_or(SmartError::RuntimeCheck("Expected 'markets' to be an object".to_string()))?;

  // Extract 'market' from each object in the 'markets' object
  let tickers: Vec<String> = markets_obj
    .values()
    .filter_map(|item| item["market"].as_str())
    .map(|s| s.to_string())
    .collect();

  Ok(tickers)
}

/// Extract Symbols Twelve
/// Takes Twelve data and returns vector of api endpoints
fn extract_symbols_twelve(json_text: String) -> Result<Vec<String>, SmartError> {
  let ticker_obj: serde_json::Value = serde_json::Value::from_str(&json_text)?;
  
  // Access the 'data' array from the parsed JSON
  let data_arr = ticker_obj["data"]
    .as_array()
    .ok_or(SmartError::RuntimeCheck("Expected 'data' to be an array".to_string()))?;
  
  // Extract 'symbol' from each object in the 'data' array
  let tickers: Vec<String> = data_arr
    .iter()
    .filter_map(|item| item["symbol"].as_str())
    .map(|s| s.to_string())
    .collect();

  Ok(tickers)
}

/// Request tickers
/// Requests list of available tickers for a given exchange
pub async fn request_symbols(exchange: &Exchange, asset_type: Option<AssetType>) -> Result<Vec<String>, SmartError> {

  // Initialize url
  let request_url: String = get_symbols_url(&exchange, asset_type);

  // Make request
  let res_data: reqwest::Response = api_request(&request_url).await?;

  // Guard: Ensure status code
  if res_data.status() != 200 {
    let e: String = format!("Failed to extract data: {:?}", res_data.text().await);
    return Err(SmartError::APIResponseStatus(e));
  }

  // Send JSON
  let json_text: String = res_data.text().await?;
  let tickers: Vec<String> = match exchange {
    Exchange::Binance => extract_symbols_binance(json_text)?,
    Exchange::BinanceUs => extract_symbols_binance(json_text)?,
    Exchange::ByBit => extract_symbols_bybit(json_text)?,
    Exchange::Coinbase => extract_symbols_coinbase(json_text)?,
    Exchange::Dydx => extract_symbols_dydx(json_text)?,
    Exchange::Twelve => extract_symbols_twelve(json_text)?,
  };

  Ok(tickers)
}

#[cfg(test)]
mod tests {
  use crate::pricing::models::{Exchange, AssetType};
  use super::request_symbols;

  #[tokio::test]
  async fn tests_get_available_symbols_binance_main() {
    let exchange: Exchange = Exchange::Binance;
    let tickers: Vec<String> = request_symbols(&exchange, None).await.unwrap();
    assert!(tickers.len() > 0);
  }

  #[tokio::test]
  async fn tests_get_available_symbols_binance_us() {
    let exchange: Exchange = Exchange::BinanceUs;
    let tickers: Vec<String> = request_symbols(&exchange, None).await.unwrap();
    assert!(tickers.len() > 0);
  }

  #[tokio::test]
  async fn tests_get_available_symbols_bybit() {
    let exchange: Exchange = Exchange::ByBit;
    let tickers: Vec<String> = request_symbols(&exchange, None).await.unwrap();
    assert!(tickers.len() > 0);
  }

  #[tokio::test]
  async fn tests_get_available_symbols_coinbase() {
    let exchange: Exchange = Exchange::Coinbase;
    let tickers: Vec<String> = request_symbols(&exchange, None).await.unwrap();
    assert!(tickers.len() > 0);
  }

  #[tokio::test]
  async fn tests_get_available_symbols_dydx() {
    let exchange: Exchange = Exchange::Dydx;
    let tickers: Vec<String> = request_symbols(&exchange, None).await.unwrap();
    assert!(tickers.len() > 0);
  }

  #[tokio::test]
  async fn tests_get_available_symbols_twelve() {
    let exchange: Exchange = Exchange::Twelve;
    let tickers: Vec<String> = request_symbols(&exchange, Some(AssetType::Forex)).await.unwrap();
    assert!(tickers.len() > 0);
  }
}