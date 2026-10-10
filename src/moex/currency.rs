use super::models::IssTable;
use crate::domain::{DailyMarketRecord, LatestQuoteRecord};
use chrono::{NaiveDateTime, TimeZone, Utc};
use chrono_tz::Europe;
use serde_json::Value;

pub fn map_history(value: &Value, board: &str) -> anyhow::Result<Vec<DailyMarketRecord>> {
    super::mapping::map_history(value, None, Some(board))
}

pub fn map_quote(value: &Value, lotsize: Option<f64>) -> anyhow::Result<LatestQuoteRecord> {
    let table = IssTable::parse(&value["marketdata"])?;
    let last = table.index("LAST")?;
    let time = table.index("TIME")?;
    let quantity = table.index("QTY")?;
    let Some(row) = table.data.first() else {
        return Ok(LatestQuoteRecord::no_trade());
    };
    let number = |index: usize| {
        row.get(index)
            .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
            .filter(|value: &f64| value.is_finite())
    };
    let Some(price) = number(last) else {
        return Ok(LatestQuoteRecord::no_trade());
    };
    let date = row.get(time).and_then(Value::as_str).and_then(|time| {
        let local_date = Utc::now().with_timezone(&Europe::Moscow).date_naive();
        NaiveDateTime::parse_from_str(&format!("{local_date} {time}"), "%Y-%m-%d %H:%M:%S")
            .ok()
            .and_then(|local| Europe::Moscow.from_local_datetime(&local).single())
            .map(|value| value.with_timezone(&Utc))
    });
    let volume = number(quantity).and_then(|lots| lotsize.map(|size| lots * size));
    Ok(LatestQuoteRecord {
        date,
        close: Some(price),
        high: None,
        low: None,
        volume,
        facevalue: None,
    })
}
