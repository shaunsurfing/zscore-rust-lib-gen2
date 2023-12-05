use std::time::Duration;
use crate::SmartError;
use super::models::HistoricalPrices;

/// Sleep
/// Basic sleep function
pub async fn sleep(millis: u64) {
  // tokio::time::sleep(Duration::from_millis(millis)).await;
  let sleep_count: u64 = millis * 1_000_000;
  for _ in 0..sleep_count {
    // Do nothing, just loop
    // Wasm hack
  }
}

/// Match Pair Series
/// Matches pair prices and labels to ensure time and lengh consistent
pub fn extract_match_series(asset_1: HistoricalPrices, asset_2: HistoricalPrices) 
-> Result<(Vec<f64>, Vec<f64>, Vec<u64>), String> 
{
  
  // Initialize
  let mut series_1: Vec<f64> = vec![];
  let mut series_2: Vec<f64> = vec![];
  let mut labels: Vec<u64> = vec![];

  // Ensure last label is the same
  let a1_last_label: &u64 = asset_1.labels.last().unwrap_or(&0);
  let a2_last_label: &u64 = asset_1.labels.last().unwrap_or(&0);
  if a1_last_label != a2_last_label {
    return Err("Error: Failed to match series (labels do not match)".to_string())
  }
  
  // Ensure series length is the same
  let a1_len: &usize = &asset_1.labels.len();
  let a2_len: &usize = &asset_2.labels.len();
  if a1_len == a2_len {
    series_1 = asset_1.prices;
    series_2 = asset_2.prices;
    labels = asset_1.labels;
  } else {
    let lowest: usize = if a1_len < a2_len { *a1_len } else { *a2_len };
    series_1.extend_from_slice(&asset_1.prices[lowest..]);
    series_2.extend_from_slice(&asset_2.prices[lowest..]);
    labels.extend_from_slice(&asset_1.labels[lowest..]);
  }

  // Return consolidated prices
  Ok((series_1, series_2, labels))
}

/// Send API Request
/// Sends GET request to given url and returns response
/// NON WASM VERSION
#[cfg(not(target_arch = "wasm32"))]
pub async fn api_request(url: &str) -> Result<reqwest::Response, SmartError> {
  let client: reqwest::Client = reqwest::Client::builder()
    .timeout(Duration::from_secs(10))
    .build()?;

  // Extract response
  let res: reqwest::Response = client
    .get(url)
    .header(reqwest::header::USER_AGENT, "CryptoWizardsApp/1.0.0")
    .send()
    .await?;
  
  // Guard: Ensure 200 status
  if res.status() != 200 {
    let err: String = format!("Failed to retrieve data for: {}", url);
    eprintln!("Error: {:?}", res.text().await);
    return Err(SmartError::APIResponseStatus(err))
  }
  
  Ok(res)
}


/// Send API Request
/// Sends GET request to given url and returns response
/// WASM VERSION
#[cfg(target_arch = "wasm32")]
pub async fn api_request(url: &str) -> Result<reqwest::Response, SmartError> {
  use async_std::future::timeout;

  // WASM VERSION
  let req_future = reqwest::Client::new()
    .get(url)
    .send();

  let duration = Duration::from_secs(10);
  let resonse_result = timeout(duration, req_future).await;
  let Ok(res_async) = resonse_result else { return Err(SmartError::RuntimeCheck("Failed to get async response".to_string())) };
  let Ok(res) = res_async else { return Err(SmartError::RuntimeCheck("Failed to get response".to_string())) };
  
  // Guard: Ensure 200 status
  if res.status() != 200 {
    let err: String = format!("Failed to retrieve data for: {}", url);
    eprintln!("Error: {:?}", res.text().await);
    return Err(SmartError::APIResponseStatus(err))
  }
  
  Ok(res)
}