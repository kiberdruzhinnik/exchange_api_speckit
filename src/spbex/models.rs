use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct SourceCandle {
    #[serde(rename = "bar_unixtime")]
    pub time: i64,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
}
