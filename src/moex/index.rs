use super::models::IssTable;
use crate::domain::{DailyMarketRecord, LatestQuoteRecord};
use chrono::{NaiveDateTime, TimeZone, Utc};
use chrono_tz::Europe;
use serde_json::Value;

pub fn map_history(value: &Value, board: &str) -> anyhow::Result<Vec<DailyMarketRecord>> {
    super::mapping::map_history(value, None, Some(board))
}

pub fn map_quote(value: &Value) -> anyhow::Result<LatestQuoteRecord> {
    let table = IssTable::parse(&value["marketdata"])?;
    let current = table.index("CURRENTVALUE")?;
    let last = table
        .columns
        .iter()
        .position(|column| column.eq_ignore_ascii_case("LASTVALUE"));
    let time = table.columns.iter().position(|column| {
        column.eq_ignore_ascii_case("SYSTIME") || column.eq_ignore_ascii_case("TIME")
    });
    let Some(row) = table.data.first() else {
        return Ok(LatestQuoteRecord::no_trade());
    };
    let number = |index: usize| {
        row.get(index)
            .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
            .filter(|value: &f64| value.is_finite())
    };
    let close = number(current).or_else(|| last.and_then(number));
    let date = time
        .and_then(|index| row.get(index))
        .and_then(Value::as_str)
        .and_then(|time| {
            let date = chrono::Utc::now()
                .with_timezone(&Europe::Moscow)
                .date_naive();
            NaiveDateTime::parse_from_str(&format!("{date} {time}"), "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|local| Europe::Moscow.from_local_datetime(&local).single())
                .map(|value| value.with_timezone(&Utc))
        });
    if close.is_none() {
        return Ok(LatestQuoteRecord::no_trade());
    }
    Ok(LatestQuoteRecord {
        date,
        close,
        high: None,
        low: None,
        volume: None,
        facevalue: None,
    })
}
