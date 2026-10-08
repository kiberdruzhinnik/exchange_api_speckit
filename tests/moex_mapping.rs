use exchange_api::moex::{
    board::{board_on_date, primary_board_by_date},
    mapping::map_history,
};
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

#[test]
fn chooses_primary_board_that_applied_on_each_trading_date() {
    let listings = fixture("board-change");
    let boards = primary_board_by_date(&listings).unwrap();
    assert_eq!(board_on_date(&boards, "2012-12-28"), Some("EQBR"));
    assert_eq!(board_on_date(&boards, "2013-01-03"), Some("TQBR"));
    let rows = serde_json::json!({
        "columns": ["BOARDID", "TRADEDATE", "CLOSE", "HIGH", "LOW", "VOLUME"],
        "data": [["EQBR", "2012-12-28", 10, 11, 9, 100], ["TQBR", "2013-01-03", 20, 21, 19, 200]]
    });
    let mut selected = Vec::new();
    for (board, _, _) in &boards {
        selected.extend(
            map_history(&rows, Some(1.0), Some(board))
                .unwrap()
                .into_iter()
                .filter(|record| {
                    let day = record.date.format("%Y-%m-%d").to_string();
                    board_on_date(&boards, &day) == Some(board.as_str())
                }),
        );
    }
    assert_eq!(selected.len(), 2);
}

#[test]
fn parses_quote_and_preserves_exact_history_shaped_fields() {
    let trade = exchange_api::moex::mapping::map_latest_trade(&fixture("trades-latest")).unwrap();
    assert_eq!(
        trade.date.unwrap().to_rfc3339(),
        "2026-10-08T08:41:57+00:00"
    );
    assert_eq!(trade.close, Some(280.34));
    assert_eq!(trade.volume, Some(1.0));
    assert_eq!(trade.high, None);
    assert_eq!(trade.low, None);
    assert_eq!(trade.facevalue, None);
}
