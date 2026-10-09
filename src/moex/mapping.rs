use super::models::IssTable;
use crate::domain::{DailyMarketRecord, LatestQuoteRecord};
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Europe;
use serde_json::Value;

pub fn map_history(
    value: &Value,
    facevalue: Option<f64>,
    board: Option<&str>,
) -> anyhow::Result<Vec<DailyMarketRecord>> {
    let table = IssTable::parse(value)?;
    let date = table.index("TRADEDATE")?;
    let board_idx = table.columns.iter().position(|c| c == "BOARDID");
    if board.is_some() && board_idx.is_none() {
        anyhow::bail!("MOEX history missing BOARDID");
    }
    let close = table.columns.iter().position(|c| c == "CLOSE");
    let high = table.columns.iter().position(|c| c == "HIGH");
    let low = table.columns.iter().position(|c| c == "LOW");
    let volume = table.columns.iter().position(|c| c == "VOLUME");
    let mut records = Vec::new();
    for row in table.data {
        let date_text = row
            .get(date)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("history row missing trading date"))?;
        let day = NaiveDate::parse_from_str(date_text, "%Y-%m-%d")
            .map_err(|e| anyhow::anyhow!("invalid trading date: {e}"))?;
        if let Some(expected) = board {
            let index = board_idx.expect("board index checked above");
            let actual = row
                .get(index)
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("history row missing board id"))?;
            if actual != expected {
                continue;
            }
        }
        let timestamp = Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0).unwrap());
        let number = |index: Option<usize>| index.and_then(|i| row.get(i)?.as_f64());
        records.push(DailyMarketRecord {
            date: timestamp,
            close: number(close),
            high: number(high),
            low: number(low),
            volume: number(volume),
            facevalue,
        });
    }
    records.sort_by_key(|record| record.date);
    Ok(records)
}

pub fn lotsize(value: &Value) -> anyhow::Result<Option<f64>> {
    let table = IssTable::parse(&value["securities"])?;
    let lotsize = table.index("LOTSIZE")?;
    Ok(table
        .data
        .first()
        .and_then(|row| row.get(lotsize))
        .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok())))
}

pub fn map_latest_trade(value: &Value) -> anyhow::Result<LatestQuoteRecord> {
    let Some(trade) = super::models::parse_latest_trade(value)? else {
        return Ok(LatestQuoteRecord::no_trade());
    };
    let local = NaiveDateTime::parse_from_str(
        &format!("{} {}", trade.trade_date, trade.trade_time),
        "%Y-%m-%d %H:%M:%S",
    )?;
    let utc = Europe::Moscow
        .from_local_datetime(&local)
        .single()
        .ok_or_else(|| anyhow::anyhow!("latest trade timestamp is ambiguous or invalid"))?
        .with_timezone(&Utc);
    Ok(LatestQuoteRecord {
        date: Some(utc),
        close: Some(trade.price),
        high: None,
        low: None,
        volume: Some(trade.quantity),
        facevalue: None,
    })
}
