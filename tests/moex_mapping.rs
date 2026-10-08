use exchange_api::moex::mapping::map_history;
use serde_json::Value;

fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(format!("tests/fixtures/moex/{name}.json")).unwrap(),
    )
    .unwrap()
}

#[test]
fn maps_named_columns_and_normalizes_trading_date() {
    let value = fixture("history-page");
    let records = map_history(&value["history"], Some(1.0), None).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].date.to_rfc3339(), "2026-10-06T00:00:00+00:00");
    assert_eq!(records[0].close, Some(282.22));
    assert_eq!(records[0].facevalue, Some(1.0));
}

#[test]
fn rejects_invalid_date_before_filtering_non_primary_board_rows() {
    let value = serde_json::json!({
        "columns": ["BOARDID", "TRADEDATE", "CLOSE", "HIGH", "LOW", "VOLUME"],
        "data": [["EQBR", null, 1.0, 1.0, 1.0, 1.0]]
    });
    let error = map_history(&value, Some(1.0), Some("TQBR")).unwrap_err();
    assert!(error.to_string().contains("trading date"));
}

#[test]
fn preserves_rows_with_missing_trailing_market_values() {
    let value = serde_json::json!({
        "columns": ["BOARDID", "TRADEDATE", "CLOSE", "HIGH", "LOW", "VOLUME"],
        "data": [["TQBR", "2026-10-06", 282.22]]
    });
    let records = map_history(&value, Some(1.0), Some("TQBR")).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].close, Some(282.22));
    assert_eq!(records[0].high, None);
    assert_eq!(records[0].low, None);
    assert_eq!(records[0].volume, None);
}

#[test]
fn preserves_null_values_and_sorts_records() {
    let value = fixture("history-multi-page");
    let records = map_history(&value["history"], None, None).unwrap();
    assert_eq!(records[0].date.to_rfc3339(), "2026-10-05T00:00:00+00:00");
    assert_eq!(records[0].close, None);
    assert_eq!(records[0].volume, None);
}
