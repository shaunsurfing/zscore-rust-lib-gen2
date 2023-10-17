use crate::SmartError;
use super::quotes::request_quote;
use super::models::{Exchange, QuotePrice};
use super::utils::{api_request, sleep};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct BinanceRawQuote {
  price: String,
  symbol: String,
  _time: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PriceWrapper {
  price: String,
}

/// Get multi quote url
/// Retrieves quote url for a given exchange
fn get_multi_quote_url(exchange: &Exchange, twelve_api_key: Option<&str>) -> String {
  match exchange {
    Exchange::Binance => "https://fapi.binance.com/fapi/v1/ticker/price".to_string(),
    Exchange::BinanceUs => "https://api.binance.us/api/v3/ticker/price".to_string(),
    Exchange::ByBit => "https://api.bybit.com/v5/market/tickers?category=linear".to_string(),
    Exchange::Coinbase => "https://api.exchange.coinbase.com/products/".to_string(),
    Exchange::Dydx => "https://api.dydx.exchange/v3/markets".to_string(),
    Exchange::Twelve => {
      match twelve_api_key {
        Some(api_key) => {
          let base_url: &str = "https://api.twelvedata.com/price?symbol={symbolstring}";
          format!("{}&apikey={}", base_url, api_key)
        },
        None => panic!("Must provide an API key for Twelve provider")
      }
    }
  }
}

/// Decode Binance Quote Data
/// Structures received data into the required price struct
fn decode_binance_quote_data(data_str: String, symbols: Vec<&str>) -> Result<Vec<QuotePrice>, SmartError> {
  let data: Vec<BinanceRawQuote> = serde_json::from_str::<Vec<BinanceRawQuote>>(data_str.as_str())?;
  let mut prices: Vec<QuotePrice> = Vec::new();
  for quote in data {
    if symbols.contains(&quote.symbol.as_str()) {
      if let Ok(price) = quote.price.parse::<f64>() {
        prices.push(QuotePrice {
          symbol: quote.symbol,
          price,
        });
      }
    }
  }
  Ok(prices)
}

/// Decode ByBit Quote Data
/// Structures received data into the required price struct
fn decode_bybit_quote_data(data_str: String, symbols: Vec<&str>) -> Result<Vec<QuotePrice>, SmartError> {
  let v: serde_json::Value = serde_json::from_str(&data_str)?;
  let list = v["result"]["list"].as_array().ok_or("Failed to get the list")
    .map_err(|e| SmartError::RuntimeCheck(e.to_string()))?;
  let mut prices: Vec<QuotePrice> = Vec::new();
  for item in list {
    if let Some(symbol) = item["symbol"].as_str() {
      if symbols.contains(&symbol) {
        if let Some(price_str) = item["lastPrice"].as_str() {
          if let Ok(price) = price_str.parse::<f64>() {
            prices.push(QuotePrice {
              symbol: symbol.to_string(),
              price,
            });
          }
        }
      }
    }
  }
  Ok(prices)
}

/// Decode Coinbase Quote Data
/// Structures received data into the required price struct
async fn decode_coinbase_quote_data(data_str: String, symbols: Vec<&str>) -> Result<Vec<QuotePrice>, SmartError> {
  let data: serde_json::Value = serde_json::from_str(&data_str)?;
  let mut prices: Vec<QuotePrice> = Vec::new();
  let mut counts = 0;
  if let Some(array) = data.as_array() {
    for obj in array {
      if let Some(base_currency) = obj["base_currency"].as_str() {
        if let Some(quote_currency) = obj["quote_currency"].as_str() {
          let symbol = format!("{}-{}", base_currency, quote_currency);

          if symbols.contains(&symbol.as_str()) {
            counts += 1;
            if counts > 1 { sleep(100).await; }

            // Call price from api call
            // This is because there is no mass price list found for coinbase
            let price: f64 = request_quote(&Exchange::Coinbase, symbol.as_str(), None).await?;
            prices.push(QuotePrice {
                symbol,
                price,
            });
          }
        }
      }
    }
  }

  Ok(prices)
}

/// Decode Dydx Quote Data
/// Structures received data into the required price struct
fn decode_dydx_quote_data(data_str: String, symbols: Vec<&str>) -> Result<Vec<QuotePrice>, SmartError> {
  let data: serde_json::Value = serde_json::from_str(&data_str)?;
  let markets = data["markets"].as_object().ok_or("Failed to parse markets")
    .map_err(|e| SmartError::RuntimeCheck(e.to_string()))?;
  let mut prices: Vec<QuotePrice> = Vec::new();
  for symbol in &symbols {
    if let Some(market) = markets.get(*symbol) {
      if let Some(price_str) = market["indexPrice"].as_str() {
        if let Ok(price) = price_str.parse::<f64>() {
          prices.push(QuotePrice {
            symbol: symbol.to_string(),
            price,
          });
        }
      }
    }
  }
  Ok(prices)
}

/// Decode Twelve Quote Data
/// Structures received data into the required price struct
fn decode_twelve_quote_data(data_str: String, symbols: Vec<&str>) -> Result<Vec<QuotePrice>, SmartError> {
  let data: HashMap<String, PriceWrapper> = serde_json::from_str(&data_str)?;
  let mut prices: Vec<QuotePrice> = Vec::new();
  for (symbol, quote) in data.iter() {
    if symbols.contains(&symbol.as_str()) {
      if let Ok(price) = quote.price.parse::<f64>() {
        prices.push(QuotePrice {
          symbol: symbol.clone(),
          price,
        });
      }
    }
  }
  Ok(prices)
}

/// Request Multi Quote
/// Requests a Quotes from a given exchange
pub async fn request_multi_quote(exchange: &Exchange, symbols: Vec<&str>, twelve_api_key: Option<&str>) -> Result<Vec<QuotePrice>, SmartError> {

  // Initialize url
  let mut request_url: String = get_multi_quote_url(&exchange, twelve_api_key);
  if exchange == &Exchange::Twelve {
    let symbolstring: String = symbols.iter().map(|&s| format!("{},",s)).collect();
    request_url = request_url.replace("{symbolstring}", symbolstring.as_str());
  }

  // Make request
  let res_data: reqwest::Response = api_request(&request_url).await?;

  // Guard: Ensure status code
  if res_data.status() != 200 {
    let e: String = format!("Failed to extract data: {:?}", res_data.text().await);
    return Err(SmartError::APIResponseStatus(e));
  }

  // Extract result
  let data_str: String = res_data.text().await?;
  match exchange {
    Exchange::Binance | Exchange::BinanceUs => Ok(decode_binance_quote_data(data_str, symbols)?),
    Exchange::ByBit => Ok(decode_bybit_quote_data(data_str, symbols)?),
    Exchange::Coinbase => Ok(decode_coinbase_quote_data(data_str, symbols).await?),
    Exchange::Dydx => Ok(decode_dydx_quote_data(data_str, symbols)?),
    Exchange::Twelve => Ok(decode_twelve_quote_data(data_str, symbols)?)
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn tests_retrieve_quotes_multi_binance_only() {
    let symbols = vec!["BTCUSDT", "ETHUSDT"];
    let prices = request_multi_quote(&Exchange::Binance, symbols, None).await.unwrap();
    // dbg!(&prices);
    assert!(prices.len() > 0);
  }

  #[tokio::test]
  async fn tests_retrieve_quotes_multi_binance_us() {
    let symbols = vec!["BTCUSDT", "ETHUSDT"];
    let prices = request_multi_quote(&Exchange::BinanceUs, symbols, None).await.unwrap();
    // dbg!(&prices);
    assert!(prices.len() > 0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_multi_bybit() {
    let symbols = vec!["BTCUSDT", "ETHUSDT"];
    let prices = request_multi_quote(&Exchange::ByBit, symbols, None).await.unwrap();
    // dbg!(&prices);
    assert!(prices.len() > 0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_multi_coinbase() {
    let symbols = vec!["BTC-USD", "ETH-USD"];
    let prices = request_multi_quote(&Exchange::Coinbase, symbols, None).await.unwrap();
    // dbg!(&prices);
    assert!(prices.len() > 0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_multi_dydx() {
    let symbols = vec!["BTC-USD", "ETH-USD"];
    let prices = request_multi_quote(&Exchange::Dydx, symbols, None).await.unwrap();
    // dbg!(&prices);
    assert!(prices.len() > 0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_multi_twelve() {
    use dotenv::dotenv;
    use std::env;
    dotenv().ok();

    let api_key: String = match env::var("TWELVE_API_KEY") {
      Ok(val) => val,
      Err(_e) => panic!("Failed to read TWELVE_API_KEY"),
    };
    
    let symbols = vec!["BTC/USD", "ETH/USD"];
    let prices = request_multi_quote(&Exchange::Twelve, symbols, Some(&api_key)).await.unwrap();
    // dbg!(&prices);
    assert!(prices.len() > 0);
  }
}