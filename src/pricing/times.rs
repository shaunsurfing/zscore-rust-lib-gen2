use chrono::{DateTime, Utc, NaiveDateTime, Timelike, Duration};
use super::models::IntervalPeriod;
use crate::SmartError;

/// Gets Current World Time in UTC
/// Retrieves world time from external API and tries another if fails
pub fn get_world_time_utc() -> Result<i64, SmartError> {
  let now: DateTime<Utc> = Utc::now();
  Ok(now.timestamp())
}

/// Convert unix timestamp to ISO format
/// Required for exchanges like DYDX
pub fn convert_timestamp_to_iso(timestamp: i64) -> String {
  let naive_dt: NaiveDateTime = NaiveDateTime::from_timestamp_opt(timestamp, 0)
    .expect("Failed to create naive datetime from timestamp");
  let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_dt, Utc);
  datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Convert ISO format to unix timestamp
/// Required for exchanges like DYDX
pub fn convert_iso_to_timestamp(mut iso_string: String, from_format: &str) -> u64 {
  let mut format_string = from_format.trim().to_string();
    
  // iso_string contains only date, append a default time and timezone.
  if !iso_string.contains("T") && !iso_string.contains(" ") {
    iso_string.push_str("T00:00:00+00:00");
  
  // iso_string has a space between date and time.
  } else if iso_string.contains(" ") {
    format_string = format_string.replace("T", " ");
    iso_string.push_str("+00:00"); // Append the '+00:00' timezone offset

  // iso_string cointains 'z' remove it
  } else if iso_string.ends_with('Z') {
    iso_string.truncate(iso_string.len() - 1); // Remove the 'Z'
    iso_string.push_str("+00:00"); // Add the '+00:00' timezone offset
  }

  let dt_naive: NaiveDateTime = NaiveDateTime::parse_from_str(&iso_string.trim(), format_string.trim())
    .expect("Failed to parse datetime from iso_string");
  
  let dt: DateTime<Utc> = DateTime::<Utc>::from_naive_utc_and_offset(dt_naive, Utc);

  dt.timestamp() as u64
}

/// Convert unix timestamp to DateTime
/// Takes in timestamp and converts into datetime
fn convert_timestamp_to_dt(timestamp: i64) -> DateTime<Utc> {
  let naive_dt: Option<NaiveDateTime> = NaiveDateTime::from_timestamp_opt(timestamp, 0);
  match naive_dt {
      Some(dt) => DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc),
      None => panic!("Invalid timestamp"),
  }
}

/// Get datetime as at interval start
/// Get unix datetime as at the last interval start
fn get_unix_datetime_at_interval(datetime: DateTime<Utc>, interval: &IntervalPeriod) -> DateTime<Utc> {
  let seconds_since_midnight = datetime.num_seconds_from_midnight();
  let remainder: i64 = match interval {
    IntervalPeriod::Min(n, _) => (seconds_since_midnight % (*n as u32 * 60)) as i64,
    IntervalPeriod::Hour(n, _) => (seconds_since_midnight % (*n as u32 * 60 * 60)) as i64,
    IntervalPeriod::Day(_, _) => datetime.timestamp() % (24 * 60 * 60 * 2), // day always starts at day 0 yesterday
  };
  datetime - chrono::Duration::seconds(remainder as i64)
}

/// Subtract Time
/// Gets timestamp after subtracting time
pub fn subtract_time(timestamp: i64, interval: &IntervalPeriod, limit: &i64) -> i64 {
  let unix_dt: DateTime<Utc> = convert_timestamp_to_dt(timestamp);
  let dt_end: DateTime<Utc> = get_unix_datetime_at_interval(unix_dt, &interval);
  let dt_start: DateTime<Utc> = match interval {
    IntervalPeriod::Min(n, _) => dt_end - Duration::minutes(*limit * (*n as i64)),
    IntervalPeriod::Hour(n, _) => dt_end - Duration::hours(*limit * (*n as i64)),
    IntervalPeriod::Day(n, _) => dt_end - Duration::days(*limit * (*n as i64)),
  };
  dt_start.timestamp()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn tests_retrieves_world_time_utc() {
    let unix_res: Result<i64, SmartError> = get_world_time_utc();
    match unix_res {
      Ok(utc) => assert!(utc > 0),
      Err(e) => panic!("Failed to retrieve utc {}", e)
    };
  }

  #[tokio::test]
  async fn tests_datetime_at_interval() {
    let unix_ts: i64 = 1688214200;
    let unix_dt: DateTime<Utc> = convert_timestamp_to_dt(unix_ts);
    let interval: IntervalPeriod = IntervalPeriod::Min(15, 0);
    let unix_start: DateTime<Utc> = get_unix_datetime_at_interval(unix_dt, &interval);
    assert_eq!(unix_start.timestamp(), 1688213700);
  }

  #[tokio::test]
  async fn tests_datetime_subtract() {
    let unix_ts: i64 = 1688214200;
    let interval: IntervalPeriod = IntervalPeriod::Day(1, 0);
    let unix_start: i64 = subtract_time(unix_ts, &interval, &0);
    assert_eq!(unix_start, 1688083200);
  }
}
