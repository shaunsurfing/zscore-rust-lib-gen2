use crate::SmartError;
use super::candles::CandleBuilder;
use super::models::{Exchange, IntervalPeriod, HistoricalPrices};

#[derive(Debug)]
pub struct PriceController {
  candle_builder: CandleBuilder
}

impl PriceController {
  pub fn new(symbol: String, interval: IntervalPeriod, exchange: Exchange, twelve_api_key: Option<&str>) 
  -> Self {
    let candle_builder: CandleBuilder = CandleBuilder::new(symbol, interval, exchange, twelve_api_key);
    Self { candle_builder }
  }

  /// Get latest prices
  /// Retrieve latest close prices and labels including current price
  pub async fn get_latest_prices(&self) -> Result<HistoricalPrices, SmartError> {
    let hist_data_res: HistoricalPrices = self.candle_builder.fetch_prices_candles().await?;
    Ok(hist_data_res)
  }
}
