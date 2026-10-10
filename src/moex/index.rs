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
    let number = |index: Option<usize>, field: &str| -> anyhow::Result<Option<f64>> {
        let Some(value) = index.and_then(|index| row.get(index)) else {
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
            .ok_or_else(|| anyhow::anyhow!("MOEX index quote has invalid {field}"))
    };
    let close = match number(Some(current), "CURRENTVALUE")? {
        Some(value) => Some(value),
        None => number(last, "LASTVALUE")?,
    };
    if close.is_none() {
        return Ok(LatestQuoteRecord::no_trade());
    }
    let date = match time
        .and_then(|index| row.get(index))
        .filter(|value| !value.is_null())
    {
        None => None,
        Some(value) => {
            let time = value
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("MOEX index quote has invalid update time"))?;
            let local_date = Utc::now().with_timezone(&Europe::Moscow).date_naive();
            let local =
                NaiveDateTime::parse_from_str(&format!("{local_date} {time}"), "%Y-%m-%d %H:%M:%S")
                    .map_err(|_| anyhow::anyhow!("MOEX index quote has invalid update time"))?;
            Some(
                Europe::Moscow
                    .from_local_datetime(&local)
                    .single()
                    .map(|value| value.with_timezone(&Utc))
                    .ok_or_else(|| anyhow::anyhow!("MOEX index quote has invalid update time"))?,
            )
        }
    };
    Ok(LatestQuoteRecord {
        date,
        close,
        high: None,
        low: None,
        volume: None,
        facevalue: None,
    })
}
