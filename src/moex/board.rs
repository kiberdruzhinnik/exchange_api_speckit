use super::models::{BoardAssignment, InstrumentCategory, IssTable, ResolvedInstrument};
use chrono::NaiveDate;
use serde_json::Value;

pub fn primary_board_by_date(value: &Value) -> anyhow::Result<Vec<(String, String, String)>> {
    let table = IssTable::parse(&value["boards"])?;
    let board = table.index("BOARDID")?;
    let primary = table
        .columns
        .iter()
        .position(|column| {
            column.eq_ignore_ascii_case("is_primary") || column.eq_ignore_ascii_case("isprimary")
        })
        .ok_or_else(|| anyhow::anyhow!("MOEX board table missing primary marker"))?;
    let from = table
        .columns
        .iter()
        .position(|column| {
            column.eq_ignore_ascii_case("history_from") || column.eq_ignore_ascii_case("from")
        })
        .ok_or_else(|| anyhow::anyhow!("MOEX board table missing history start"))?;
    let till = table.columns.iter().position(|column| {
        column.eq_ignore_ascii_case("history_till") || column.eq_ignore_ascii_case("till")
    });
    let mut result = Vec::new();
    for row in table.data {
        if row
            .get(primary)
            .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
            .unwrap_or_default()
            != 1
        {
            continue;
        }
        let id = row
            .get(board)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("primary board missing id"))?
            .to_owned();
        let start = row
            .get(from)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("primary board missing start date"))?
            .to_owned();
        let end = till
            .and_then(|i| row.get(i))
            .and_then(Value::as_str)
            .unwrap_or("9999-12-31")
            .to_owned();
        result.push((id, start, end));
    }
    Ok(result)
}

pub fn board_on_date<'a>(boards: &'a [(String, String, String)], date: &str) -> Option<&'a str> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    boards
        .iter()
        .find(|(_, from, till)| {
            NaiveDate::parse_from_str(from, "%Y-%m-%d").is_ok_and(|start| start <= date)
                && NaiveDate::parse_from_str(till, "%Y-%m-%d").is_ok_and(|end| date <= end)
        })
        .map(|(id, _, _)| id.as_str())
}

pub fn resolve_instrument(value: &serde_json::Value) -> anyhow::Result<Option<ResolvedInstrument>> {
    let table = IssTable::parse(&value["boards"])?;
    let engine = table
        .columns
        .iter()
        .position(|column| column.eq_ignore_ascii_case("engine"));
    let market = table
        .columns
        .iter()
        .position(|column| column.eq_ignore_ascii_case("market"));
    let board = table.index("boardid")?;
    let primary = table
        .columns
        .iter()
        .position(|column| {
            column.eq_ignore_ascii_case("is_primary") || column.eq_ignore_ascii_case("isprimary")
        })
        .ok_or_else(|| anyhow::anyhow!("MOEX board table missing primary marker"))?;
    let from = table.columns.iter().position(|column| {
        column.eq_ignore_ascii_case("history_from") || column.eq_ignore_ascii_case("from")
    });
    let till = table.columns.iter().position(|column| {
        column.eq_ignore_ascii_case("history_till") || column.eq_ignore_ascii_case("till")
    });

    let supported = table.data.iter().find_map(|row| {
        let engine_name = engine
            .and_then(|index| row.get(index))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("stock");
        let market_name = market
            .and_then(|index| row.get(index))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("shares");
        let category = match (engine_name, market_name) {
            ("stock", "shares") => InstrumentCategory::Shares,
            ("stock", "index") => InstrumentCategory::Index,
            ("currency", "selt") => InstrumentCategory::Currency,
            _ => return None,
        };
        Some((engine_name.to_owned(), market_name.to_owned(), category))
    });
    let Some((engine_name, market_name, category)) = supported else {
        return Ok(None);
    };

    let mut assignments = Vec::new();
    for row in &table.data {
        if engine
            .and_then(|index| row.get(index))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("stock")
            != engine_name
            || market
                .and_then(|index| row.get(index))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("shares")
                != market_name
        {
            continue;
        }
        let board_name = row
            .get(board)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("MOEX board assignment missing board id"))?;
        let is_primary = row
            .get(primary)
            .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
            .unwrap_or_default()
            == 1;
        if !is_primary {
            continue;
        }
        let history_from =
            effective_date(from.and_then(|index| row.get(index)), None, "history_from")?;
        let history_till = effective_date(
            till.and_then(|index| row.get(index)),
            Some("9999-12-31"),
            "history_till",
        )?;
        let start = NaiveDate::parse_from_str(&history_from, "%Y-%m-%d")
            .map_err(|error| anyhow::anyhow!("invalid MOEX history_from date: {error}"))?;
        let end = NaiveDate::parse_from_str(&history_till, "%Y-%m-%d")
            .map_err(|error| anyhow::anyhow!("invalid MOEX history_till date: {error}"))?;
        anyhow::ensure!(
            start <= end,
            "MOEX primary board has an inverted effective-date interval"
        );
        assignments.push(BoardAssignment {
            engine: engine_name.clone(),
            market: market_name.clone(),
            board: board_name.to_owned(),
            primary: true,
            history_from: history_from.to_owned(),
            history_till: history_till.to_owned(),
        });
    }
    if assignments.is_empty() {
        return Ok(None);
    }
    assignments.sort_by(|a, b| a.history_from.cmp(&b.history_from));
    for pair in assignments.windows(2) {
        let previous_end = NaiveDate::parse_from_str(&pair[0].history_till, "%Y-%m-%d")
            .map_err(|error| anyhow::anyhow!("invalid MOEX history_till date: {error}"))?;
        let next_start = NaiveDate::parse_from_str(&pair[1].history_from, "%Y-%m-%d")
            .map_err(|error| anyhow::anyhow!("invalid MOEX history_from date: {error}"))?;
        anyhow::ensure!(
            previous_end < next_start,
            "MOEX primary-board effective-date intervals overlap"
        );
    }
    Ok(Some(ResolvedInstrument {
        category,
        engine: engine_name,
        market: market_name,
        boards: assignments,
        lotsizes: Default::default(),
    }))
}

fn effective_date(
    value: Option<&serde_json::Value>,
    default: Option<&str>,
    field: &str,
) -> anyhow::Result<String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return default
            .map(str::to_owned)
            .ok_or_else(|| anyhow::anyhow!("MOEX {field} is required"));
    };
    let date = value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("MOEX {field} must be a date string"))?;
    let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|error| anyhow::anyhow!("invalid MOEX {field} date: {error}"))?;
    anyhow::ensure!(
        parsed.format("%Y-%m-%d").to_string() == date,
        "invalid MOEX {field} date format"
    );
    Ok(date.to_owned())
}
