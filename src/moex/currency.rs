use super::models::IssTable;
use crate::domain::{DailyMarketRecord, LatestQuoteRecord};
use chrono::{NaiveDateTime, TimeZone, Utc};
use chrono_tz::Europe;
use serde_json::Value;

pub fn map_history(value: &Value, board: &str) -> anyhow::Result<Vec<DailyMarketRecord>> {
    super::mapping::map_history_with_numeric_validation(value, None, Some(board), true)
}

pub fn map_quote(value: &Value) -> anyhow::Result<LatestQuoteRecord> {
    let table = IssTable::parse(&value["marketdata"])?;
    let last = table.index("LAST")?;
    let time = table.index("TIME")?;
    let Some(row) = table.data.first() else {
        return Ok(LatestQuoteRecord::no_trade());
    };
    let number = |index: usize, field: &str| -> anyhow::Result<Option<f64>> {
        let Some(value) = row.get(index) else {
            return Ok(None);
        };
        if value.is_null() {
            return Ok(None);
        }
        value
            .as_f64()
            .or_else(|| value.as_str()?.parse().ok())
            .filter(|value: &f64| value.is_finite())
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("MOEX currency quote has invalid {field}"))
    };
    let Some(price) = number(last, "LAST")? else {
        return Ok(LatestQuoteRecord::no_trade());
    };
    let time = row
        .get(time)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("MOEX currency quote has invalid TIME"))?;
    let local_date = Utc::now().with_timezone(&Europe::Moscow).date_naive();
    let local = NaiveDateTime::parse_from_str(&format!("{local_date} {time}"), "%Y-%m-%d %H:%M:%S")
        .map_err(|_| anyhow::anyhow!("MOEX currency quote has invalid TIME"))?;
    let date = Europe::Moscow
        .from_local_datetime(&local)
        .single()
        .map(|value| value.with_timezone(&Utc))
        .ok_or_else(|| anyhow::anyhow!("MOEX currency quote has invalid TIME"))?;
    let volume = super::mapping::current_trade_count(value)?;
    Ok(LatestQuoteRecord {
        date: Some(date),
        close: Some(price),
        high: None,
        low: None,
        volume,
        facevalue: None,
    })
}
