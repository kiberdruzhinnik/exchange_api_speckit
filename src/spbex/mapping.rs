use crate::{
    domain::{DailyMarketRecord, LatestTradeRecord},
    spbex::models::SourceCandle,
};
use chrono::{DateTime, Utc};

pub fn map_history(candles: &[SourceCandle]) -> anyhow::Result<Vec<DailyMarketRecord>> {
    let mut records = candles
        .iter()
        .map(map_candle)
        .collect::<anyhow::Result<Vec<_>>>()?;
    records.sort_by_key(|record| record.date);
    for pair in records.windows(2) {
        anyhow::ensure!(
            pair[0].date != pair[1].date,
            "duplicate SPBEX candle timestamp"
        );
    }
    Ok(records)
}

pub fn map_latest(candle: Option<&SourceCandle>) -> anyhow::Result<LatestTradeRecord> {
    let Some(candle) = candle else {
        return Ok(LatestTradeRecord::no_trade());
    };
    let record = map_candle(candle)?;
    Ok(LatestTradeRecord {
        date: Some(record.date),
        close: record.close,
        high: record.high,
        low: record.low,
        volume: None,
        facevalue: record.facevalue,
    })
}

fn map_candle(candle: &SourceCandle) -> anyhow::Result<DailyMarketRecord> {
    anyhow::ensure!(candle.time > 0, "SPBEX candle timestamp must be positive");
    let close = valid_price(candle.close, "close")?;
    let high = valid_price(candle.high, "high")?;
    let low = valid_price(candle.low, "low")?;
    anyhow::ensure!(
        low <= close && close <= high,
        "SPBEX candle has inconsistent OHLC values"
    );
    let date = DateTime::from_timestamp(candle.time, 0)
        .ok_or_else(|| anyhow::anyhow!("invalid SPBEX candle timestamp"))?;
    Ok(DailyMarketRecord {
        date: date.with_timezone(&Utc),
        close: Some(close),
        high: Some(high),
        low: Some(low),
        volume: None,
        facevalue: Some(1.0),
    })
}

fn valid_price(value: Option<f64>, field: &str) -> anyhow::Result<f64> {
    let value = value.ok_or_else(|| anyhow::anyhow!("SPBEX candle missing {field}"))?;
    anyhow::ensure!(
        value.is_finite() && value > 0.0,
        "SPBEX candle has invalid {field}"
    );
    Ok(value)
}
