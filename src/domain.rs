use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct DailyMarketRecord {
    #[serde(serialize_with = "serialize_utc_z")]
    pub date: DateTime<Utc>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub facevalue: Option<f64>,
}

fn serialize_utc_z<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&date.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}
