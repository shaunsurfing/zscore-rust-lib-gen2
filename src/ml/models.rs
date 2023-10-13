use crate::SmartError;
use crate::pricing::models::{DataCriteria, AssetType, Exchange, IntervalPeriod};

use smartcore::ensemble::random_forest_classifier::RandomForestClassifier;
use smartcore::metrics::accuracy;
use smartcore::model_selection::train_test_split;
use smartcore::linalg::basic::matrix::DenseMatrix;
use wasm_bindgen::prelude::wasm_bindgen;

/*
  TODO: WORK IN PROGRESS
*/

struct ModelData {
  x: Vec<Vec<f64>>,
  y: Vec<i32>
}

struct ModelResults {
  metric_1: Option<String>,
  metric_2: Option<String>,
  metric_3: Option<String>
}

struct MLClassifier {
  data_criteria: DataCriteria,
  model_data: Option<ModelData>,
  model_results: Option<ModelResults>
}

impl MLClassifier {
  pub fn new(data_criteria: DataCriteria) -> Self {
    Self {
      data_criteria,
      model_data: None,
      model_results: None,
    }
  }

  /// Fetches price data, performs backtest, 
  pub fn construct_model_data(&mut self) {

  }
}


/// X: Vec<Vec<f64>>, y: Vec<i32> -> json string
// #[wasm_bindgen]
pub fn train_classifier(x_json: String, y_json: String) -> Result<String, String> {

  // Convert X Vec to Slice
  let vec_2d: Vec<Vec<f64>> = serde_json::from_str::<Vec<Vec<f64>>>(&x_json).map_err(|e| e.to_string())?;
  let temp_vec: Vec<&[f64]> = vec_2d.iter().map(AsRef::as_ref).collect();
  let slice_2d: &[&[f64]] = &temp_vec;

  // Initialize X and y
  let x: DenseMatrix<f64> = DenseMatrix::from_2d_array(slice_2d);
  let y: Vec<i32> = serde_json::from_str::<Vec<i32>>(&y_json).map_err(|e| e.to_string())?;

  // Train Test Split
  let (x_train, x_test, y_train, y_test) = train_test_split(
    &x, 
    &y,
    0.2, 
    false, 
    Some(12345)
  );

  // Create a random forest classifier
  let classifier = RandomForestClassifier::fit(&x, &y, Default::default()).unwrap();

  // Predict the classes for the test data
  let y_hat = classifier.predict(&x_test).unwrap();

  // Compute the accuracy of the model
  let accuracy: f64 = accuracy(&y_hat, &y_test);
  dbg!(y_hat, y_test, accuracy);

  Ok("".to_string())
}

// #[wasm_bindgen]
pub fn dummy_train_classifier() {

  let x_vec: &[&[f64]] = &[
    &[5.1, 3.5, 1.4, 0.2],
    &[4.9, 3.0, 1.4, 0.2],
    &[4.7, 3.2, 1.3, 0.2],
    &[4.6, 3.1, 1.5, 0.2],
    &[5.0, 3.6, 1.4, 0.2],
    &[5.4, 3.9, 1.7, 0.4],
    &[4.6, 3.4, 1.4, 0.3],
    &[5.0, 3.4, 1.5, 0.2],
    &[4.4, 2.9, 1.4, 0.2],
    &[4.9, 3.1, 1.5, 0.1],
    &[7.0, 3.2, 4.7, 1.4],
    &[6.4, 3.2, 4.5, 1.5],
    &[6.9, 3.1, 4.9, 1.5],
    &[5.5, 2.3, 4.0, 1.3],
    &[6.5, 2.8, 4.6, 1.5],
    &[5.7, 2.8, 4.5, 1.3],
    &[6.3, 3.3, 4.7, 1.6],
    &[4.9, 2.4, 3.3, 1.0],
    &[6.6, 2.9, 4.6, 1.3],
    &[5.2, 2.7, 3.9, 1.4],
  ];

  // Iris Features
  let x: DenseMatrix<f64> = DenseMatrix::from_2d_array(x_vec);

  // Prediction Features
  let y: Vec<i32> = vec![
    0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  ];

  let (x_train, x_test, y_train, y_test) = train_test_split(
    &x, 
    &y, 
    0.2, 
    false, 
    Some(12345)
  );

  // Create a random forest classifier
  let classifier = RandomForestClassifier::fit(&x, &y, Default::default()).unwrap();

  // Predict the classes for the test data
  let y_hat = classifier.predict(&x_test).unwrap();

  // Compute the accuracy of the model
  let accuracy: f64 = accuracy(&y_hat, &y_test);
  dbg!(y_hat, y_test, accuracy);
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::{PairAnalysis, full_pair_analysis};
  use crate::pricing::models::{DataCriteria, AssetType, Exchange, IntervalPeriod};
  use crate::pricing::symbols::request_symbols;
  use crate::pricing::volume::request_high_volume_tickers_all;

  #[tokio::test]
  async fn it_trains_model() {

    // // Get analysis
    // let exchange = Exchange::Binance;
    // let asset_type = AssetType::Crypto;
    // let interval_period = IntervalPeriod::Hour(1, 1000);
    // let data_criteria: DataCriteria = DataCriteria {
    //   exchange: exchange.clone(),
    //   asset_0: "BTCUSDT".to_string(),
    //   asset_1: "ETHUSDT".to_string(),
    //   interval_period: interval_period.clone()
    // };
    // let analysis: PairAnalysis = full_pair_analysis(data_criteria, None).await.unwrap();
    
    // Compile X data

    // dummy_train_classifier();
  }
}
