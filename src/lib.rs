pub mod backtest;
pub mod ml;
pub mod prelude;
pub mod pricing;
pub mod stats;

#[derive(thiserror::Error, Debug)]
pub enum SmartError {
  #[error("Failed to retrieve data")]
  APIResponseStatus(String),
  #[error("Runtime error check failed")]
  RuntimeCheck(String),
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  ParseFloat(#[from] std::num::ParseFloatError),
  #[error(transparent)]
  Reqwest(#[from] reqwest::Error),
  #[error(transparent)]
  SerdeJson(#[from] serde_json::Error)
}
