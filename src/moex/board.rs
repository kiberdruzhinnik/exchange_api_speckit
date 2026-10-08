use super::models::IssTable;
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
