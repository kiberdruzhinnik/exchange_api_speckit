use exchange_api::spbex::{
    mapping::{map_history, map_latest},
    models::SourceCandle,
};

fn fixture(name: &str) -> Vec<SourceCandle> {
    serde_json::from_str(
        &std::fs::read_to_string(format!("tests/fixtures/spbex/{name}.json")).unwrap(),
    )
    .unwrap()
}

#[test]
fn maps_and_sorts_six_field_history_records() {
    let records = map_history(&fixture("history")).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].date.to_rfc3339(), "2013-03-25T00:00:00+00:00");
    assert_eq!(records[0].close, Some(73.35));
    assert_eq!(records[0].volume, None);
    assert_eq!(records[0].facevalue, Some(1.0));
}

#[test]
fn rejects_bad_ohlc_duplicate_and_invalid_timestamps() {
    assert!(map_history(&fixture("malformed")).is_err());
    assert!(
        map_history(&[
            SourceCandle {
                time: 1_000_000,
                close: Some(2.0),
                high: Some(3.0),
                low: Some(1.0)
            },
            SourceCandle {
                time: 1_000_000,
                close: Some(2.0),
                high: Some(3.0),
                low: Some(1.0)
            },
        ])
        .is_err()
    );
    assert!(
        map_history(&[SourceCandle {
            time: 0,
            close: Some(1.0),
            high: Some(1.0),
            low: Some(1.0)
        }])
        .is_err()
    );
}

#[test]
fn maps_empty_quote_to_nullable_record_and_latest_candle_to_one_record() {
    let empty = map_latest(None).unwrap();
    assert_eq!(empty.date, None);
    let source = fixture("history");
    let quote = map_latest(source.last()).unwrap();
    assert_eq!(quote.close, Some(74.15));
    assert_eq!(quote.volume, None);
    assert_eq!(quote.facevalue, Some(1.0));
}
