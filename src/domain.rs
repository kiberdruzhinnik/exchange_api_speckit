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

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct LatestTradeRecord {
    #[serde(serialize_with = "serialize_optional_utc_z")]
    pub date: Option<DateTime<Utc>>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub facevalue: Option<f64>,
}

impl LatestTradeRecord {
    pub fn no_trade() -> Self {
        Self {
            date: None,
            close: None,
            high: None,
            low: None,
            volume: None,
            facevalue: None,
        }
    }
}

fn serialize_utc_z<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&date.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

fn serialize_optional_utc_z<S>(
    date: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match date {
        Some(date) => serializer.serialize_some(&date.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        None => serializer.serialize_none(),
    }
}
