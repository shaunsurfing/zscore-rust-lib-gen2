use crate::SmartError;
use super::models::{Exchange, QuoteExch};
use super::utils::api_request;

/// Get quote url
/// Retrieves quote url for a given exchange
fn get_quote_url(exchange: &Exchange, twelve_api_key: Option<&str>) -> String {
  match exchange {
    Exchange::Binance => "https://fapi.binance.com/fapi/v1/ticker/price?symbol={symbol}".to_string(),
    Exchange::BinanceUs => "https://api.binance.us/api/v3/ticker/price?symbol={symbol}".to_string(),
    Exchange::ByBit => "https://api.bybit.com/v5/market/tickers?category=linear&symbol={symbol}".to_string(),
    Exchange::Coinbase => "https://api.exchange.coinbase.com/products/{symbol}/book?level=0".to_string(),
    Exchange::Dydx => "https://api.dydx.exchange/v3/markets?market={symbol}".to_string(),
    Exchange::Twelve => {
      match twelve_api_key {
        Some(api_key) => {
          let base_url: &str = "https://api.twelvedata.com/price?symbol={symbol}";
          format!("{}&apikey={}", base_url, api_key)
        },
        None => panic!("Must provide an API key for Twelve provider")
      }
    }
  }
}

/// Request quote
/// Requests a quote from a given exchange
pub async fn request_quote(exchange: &Exchange, symbol: &str, twelve_api_key: Option<&str>) -> Result<f64, SmartError> {

  // Initialize url
  let mut request_url: String = get_quote_url(&exchange, twelve_api_key);
  request_url = request_url.replace("{symbol}", symbol);

  // Make request
  let res_data: reqwest::Response = api_request(&request_url).await?;

  // Guard: Ensure status code
  if res_data.status() != 200 {
    let e: String = format!("Failed to extract data: {:?}", res_data.text().await);
    return Err(SmartError::APIResponseStatus(e));
  }

  // Extract result
  let data_obj: serde_json::Value = res_data.json().await?;
  let price: f64 = match exchange {
    Exchange::Binance | Exchange::BinanceUs => {
      let price = data_obj.get("price")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
      price
    },
    Exchange::ByBit => {
      let price = data_obj.get("result")
        .and_then(|v| v.get("list"))
        .and_then(|list| list.get(0))
        .and_then(|obj| obj.get("lastPrice"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
      price
    },
    Exchange::Coinbase => {
      let price = data_obj.get("asks")
        .and_then(serde_json::Value::as_array)
        .and_then(|asks| asks.first())
        .and_then(serde_json::Value::as_array)
        .and_then(|ask| ask.get(0))
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
      price
    },
    Exchange::Dydx => {
      let price: f64 = if let Some(markets) = data_obj.get("markets").and_then(serde_json::Value::as_object) {
        let mut price_detail: f64 = 0.0;
        for (_, details) in markets {
          price_detail = details.get("indexPrice")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
          break;
        }
        price_detail
      } else {
        0.0
      };
      price
    },
    Exchange::Twelve => {
      let price: f64 = data_obj.get("price")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
      price
    }
  };
  
  Ok(price)
}

/// Get Quotes All Exchanges
/// Retrieve quotes for all exchanges
pub async fn get_quotes_all_exchanges(twelve_api_key: Option<&str>) -> Result<QuoteExch, SmartError> {
  let exchanges: [Exchange; 6] = [Exchange::Binance, Exchange::BinanceUs, Exchange::ByBit, Exchange::Coinbase, Exchange::Dydx, Exchange::Twelve];
  let mut quote_exch: QuoteExch = QuoteExch { binance: 0.0, binance_us: 0.0, bybit: 0.0, coinbase: 0.0, dydx: 0.0, twelve: 0.0 };

  for exchange in exchanges {

    let symbol: &str = match exchange {
      Exchange::Binance | Exchange::BinanceUs | Exchange::ByBit => "BTCUSDT",
      Exchange::Coinbase | Exchange::Dydx  => "BTC-USD",
      Exchange::Twelve => "BTCUSD"
    };

    let quote_res: Result<f64, SmartError> = request_quote(&exchange, symbol, twelve_api_key).await;

    if let Ok(quote) = quote_res {
      match exchange {
        Exchange::Binance => quote_exch.binance = quote,
        Exchange::BinanceUs => quote_exch.binance_us = quote,
        Exchange::ByBit => quote_exch.bybit = quote,
        Exchange::Coinbase => quote_exch.coinbase = quote,
        Exchange::Dydx => quote_exch.dydx = quote,
        Exchange::Twelve => quote_exch.twelve = quote
      }
    }
  }

  Ok(quote_exch)
}


#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn tests_retrieve_quote_binance() {
    let price = request_quote(&Exchange::Binance, "BTCUSDT", None).await;
    assert!(price.unwrap() > 0.0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_binance_us() {
    let price = request_quote(&Exchange::BinanceUs, "BTCUSDT", None).await;
    assert!(price.unwrap() > 0.0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_bybit() {
    let price = request_quote(&Exchange::ByBit, "BTCUSDT", None).await;
    assert!(price.unwrap() > 0.0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_coinbase() {
    let price = request_quote(&Exchange::Coinbase, "BTC-USD", None).await;
    assert!(price.unwrap() > 0.0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_dydx() {
    let price = request_quote(&Exchange::Dydx, "BTC-USD", None).await;
    assert!(price.unwrap() > 0.0);
  }

  #[tokio::test]
  async fn tests_retrieve_quote_twelve() {
    use dotenv::dotenv;
    use std::env;
    dotenv().ok();

    let api_key: String = match env::var("TWELVE_API_KEY") {
      Ok(val) => val,
      Err(_e) => panic!("Failed to read TWELVE_API_KEY"),
    };

    let price = request_quote(&Exchange::Twelve, "USD/GBP", Some(&api_key)).await;
    assert!(price.unwrap() > 0.0);
  }

  #[tokio::test]
  async fn tests_get_quotes_all_exchanges() {
    use dotenv::dotenv;
    use std::env;
    dotenv().ok();

    let api_key: String = match env::var("TWELVE_API_KEY") {
      Ok(val) => val,
      Err(_e) => panic!("Failed to read TWELVE_API_KEY"),
    };

    let quotes: QuoteExch = get_quotes_all_exchanges(Some(&api_key)).await.unwrap();
    assert!(quotes.coinbase > 0.0);
  }
}