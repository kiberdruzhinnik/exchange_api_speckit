use crate::{
    cbr::models::{DailyRate, DynamicRate, DynamicRates},
    domain::{DailyMarketRecord, LatestTradeRecord},
};
use anyhow::{Context, bail};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use std::collections::HashSet;

pub fn map_history(source: &DynamicRates) -> anyhow::Result<Vec<DailyMarketRecord>> {
    let mut records = Vec::with_capacity(source.records.len());
    let mut dates = HashSet::with_capacity(source.records.len());
    for row in &source.records {
        let record = map_dynamic_rate(row)?;
        if !dates.insert(record.date) {
            bail!("duplicate CBR history date");
        }
        records.push(record);
    }
    records.sort_by_key(|record| record.date);
    Ok(records)
}

pub fn map_latest(source: Option<(String, DailyRate)>) -> anyhow::Result<LatestTradeRecord> {
    let Some((date, rate)) = source else {
        return Ok(LatestTradeRecord::no_trade());
    };
    let (date, close, facevalue) =
        map_values(&date, rate.value.as_deref(), rate.nominal.as_deref())?;
    Ok(LatestTradeRecord {
        date: Some(date),
        close,
        high: None,
        low: None,
        volume: None,
        facevalue,
    })
}

fn map_dynamic_rate(row: &DynamicRate) -> anyhow::Result<DailyMarketRecord> {
    let date_text = row
        .date
        .as_deref()
        .context("CBR history row has no effective date")?;
    let (date, close, facevalue) =
        map_values(date_text, row.value.as_deref(), row.nominal.as_deref())?;
    Ok(DailyMarketRecord {
        date,
        close,
        high: None,
        low: None,
        volume: None,
        facevalue,
    })
}

fn map_values(
    date_text: &str,
    value_text: Option<&str>,
    nominal_text: Option<&str>,
) -> anyhow::Result<(DateTime<Utc>, Option<f64>, Option<f64>)> {
    let date = NaiveDate::parse_from_str(date_text, "%d.%m.%Y")
        .with_context(|| format!("invalid CBR effective date {date_text:?}"))?;
    let date = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).context("invalid CBR date")?);
    let value = parse_decimal(value_text, "value")?;
    let nominal = parse_decimal(nominal_text, "nominal")?;
    if nominal.is_some_and(|value| value <= 0.0) {
        bail!("CBR nominal must be positive");
    }
    let close = match (value, nominal) {
        (Some(value), Some(nominal)) => {
            let normalized = value / nominal;
            if !normalized.is_finite() || normalized <= 0.0 {
                bail!("CBR normalized rate must be finite and positive");
            }
            Some(normalized)
        }
        _ => None,
    };
    Ok((date, close, nominal))
}

fn parse_decimal(value: Option<&str>, field: &str) -> anyhow::Result<Option<f64>> {
    value
        .map(|value| {
            let normalized = value.trim().replace(',', ".");
            let parsed: f64 = normalized
                .parse()
                .with_context(|| format!("invalid CBR {field} value {value:?}"))?;
            if !parsed.is_finite() {
                bail!("CBR {field} must be finite");
            }
            Ok(parsed)
        })
        .transpose()
}
