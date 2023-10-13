// use tokio::try_join;

use crate::SmartError;
use super::controller::PriceController;
use super::utils::extract_match_series;
use super::quotes::request_quote;
use super::models::{AssetType, Exchange, IntervalPeriod, DataCriteria, PairPrices};

/// Get Prices for Pair
/// Retrieves prices for items specified by user
/// Executes request simultaneously via two threads
pub async fn get_prices_pair(data_criteria: DataCriteria, twelve_api_key: Option<&str>) -> Result<PairPrices, SmartError> {

  // Initialize price controller - asset_1
  let controller_1: PriceController = PriceController::new(
    data_criteria.asset_0.clone(), 
    data_criteria.interval_period.clone(), 
    data_criteria.exchange.clone(),
    twelve_api_key
  );

  // Initialize price controller - asset_2
  let controller_2: PriceController = PriceController::new(
    data_criteria.asset_1.clone(), 
    data_criteria.interval_period.clone(), 
    data_criteria.exchange.clone(),
    twelve_api_key
  );

  let asset_1_future = controller_1.get_latest_prices();
  let asset_2_future = controller_2.get_latest_prices();
  let (asset_1_res, asset_2_res) = futures::join!(asset_1_future, asset_2_future);

  // // Get prices concurrently
  // let task1 = tokio::spawn(async move {
  //   controller_1.get_latest_prices().await
  // });

  // let task2 = tokio::spawn(async move {
  //   controller_2.get_latest_prices().await
  // });
  
  // let (asset_1_res, asset_2_res) = try_join!(task1, task2)
  //   .expect("Failed to join concurrent price processes");

  // Ensure time and length match
  let (series_0, series_1, labels) = match asset_1_res {
    Ok(asset_1) => match asset_2_res {
      Ok(asset_2) => {
        match extract_match_series(asset_1, asset_2) {
          Ok((series_1, series_2, labels)) => (series_1, series_2, labels),
          Err(_) => return Err(SmartError::RuntimeCheck("Could not match series".to_string()))
        }
      },
      Err(e) => return Err(SmartError::RuntimeCheck(e.to_string()))
    },
    Err(e) => return Err(SmartError::RuntimeCheck(e.to_string()))
  };

  Ok(PairPrices { series_0, series_1, labels })
}

/// Get Available Assets
/// Retrieves list of tradeable assets for a given exchange
pub async fn get_available_assets(exchange_str: &str, asset_type: Option<AssetType>) -> Result<String, SmartError> {
  let exchange: Exchange = Exchange::create_from_string(exchange_str);
  let symbols: Vec<String> = exchange.available_assets(asset_type).await?;
  let symbols_json = serde_json::to_string(&symbols)?;
  Ok(symbols_json)
}

/// Get Prices
/// Fetches prices
pub async fn fetch_prices(
  interval_period: &IntervalPeriod, 
  exchange: &Exchange, 
  asset_0: &str, 
  asset_1: &str,
  twelve_api_key: Option<&str>
) -> Result<PairPrices, SmartError> {

  let data_criteria: DataCriteria = DataCriteria {
    interval_period: interval_period.clone(),
    asset_0: asset_0.to_string(),
    asset_1: asset_1.to_string(),
    exchange: exchange.clone()
  };

  let prices: PairPrices = get_prices_pair(data_criteria, twelve_api_key).await?;

  Ok(prices)
}

/// Get Quote
/// Retrieve latest quote
pub async fn get_latest_quote(symbol: &str, exchange: &Exchange, twelve_api_key: Option<&str>) -> Result<f64, SmartError> {
  let quote: f64 = request_quote(&exchange, symbol, twelve_api_key).await?;
  Ok(quote)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn it_matches_period_request_twelve() {
    use dotenv::dotenv;
    use std::env;
    dotenv().ok();

    let twelve_api_key: String = match env::var("TWELVE_API_KEY") {
      Ok(val) => val,
      Err(_e) => panic!("Failed to read TWELVE_API_KEY"),
    };

    let period = 1000;
    let interval_period: IntervalPeriod = IntervalPeriod::Hour(1, period);
    let asset_0 = "USD/GBP".to_string();
    let asset_1 = "USD/EUR".to_string();
    let exchange: Exchange = Exchange::Twelve;

    let prices = fetch_prices(&interval_period, &exchange, &asset_0, &asset_1, Some(twelve_api_key.as_str())).await.unwrap();
    // assert_eq!(prices.labels.len(), period as usize);
  }
}
