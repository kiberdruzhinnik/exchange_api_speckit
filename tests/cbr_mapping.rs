use chrono::{NaiveDate, TimeZone, Utc};
use exchange_api::cbr::{
    mapping::{map_history, map_latest},
    models::{DailyRate, DynamicRate, DynamicRates},
};

fn history() -> DynamicRates {
    serde_xml_fixture(include_str!("fixtures/cbr/history.xml"))
}

fn serde_xml_fixture(xml: &str) -> DynamicRates {
    quick_xml::de::from_str(xml).unwrap()
}

#[test]
fn maps_sorted_six_field_records_and_normalizes_non_unit_nominal() {
    let records = map_history(&history()).unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].date.to_rfc3339(), "2013-03-25T00:00:00+00:00");
    assert_eq!(records[0].close, Some(0.05));
    assert_eq!(records[0].facevalue, Some(100.0));
    assert_eq!(records[0].high, None);
    assert_eq!(records[0].low, None);
    assert_eq!(records[0].volume, None);
    assert_eq!(records[2].close, None);
}

#[test]
fn missing_value_or_nominal_maps_close_to_null() {
    let source = DynamicRates {
        records: vec![
            DynamicRate {
                date: Some("25.03.2013".into()),
                nominal: Some("100".into()),
                value: None,
            },
            DynamicRate {
                date: Some("26.03.2013".into()),
                nominal: None,
                value: Some("5".into()),
            },
        ],
    };
    let records = map_history(&source).unwrap();
    assert!(records.iter().all(|record| record.close.is_none()));
}

#[test]
fn rejects_missing_or_invalid_dates_and_duplicate_dates() {
    let missing = DynamicRates {
        records: vec![DynamicRate {
            date: None,
            nominal: Some("1".into()),
            value: Some("1".into()),
        }],
    };
    assert!(map_history(&missing).is_err());
    let invalid = DynamicRates {
        records: vec![DynamicRate {
            date: Some("not-a-date".into()),
            nominal: Some("1".into()),
            value: Some("1".into()),
        }],
    };
    assert!(map_history(&invalid).is_err());
    let duplicate = DynamicRates {
        records: vec![
            DynamicRate {
                date: Some("25.03.2013".into()),
                nominal: Some("1".into()),
                value: Some("1".into()),
            },
            DynamicRate {
                date: Some("25.03.2013".into()),
                nominal: Some("1".into()),
                value: Some("1".into()),
            },
        ],
    };
    assert!(map_history(&duplicate).is_err());
}

#[test]
fn rejects_invalid_provided_nominal_and_rate_values() {
    let invalid_nominal = DynamicRates {
        records: vec![DynamicRate {
            date: Some("25.03.2013".into()),
            nominal: Some("0".into()),
            value: Some("5".into()),
        }],
    };
    assert!(map_history(&invalid_nominal).is_err());
    let invalid_rate = DynamicRates {
        records: vec![DynamicRate {
            date: Some("25.03.2013".into()),
            nominal: Some("1".into()),
            value: Some("-5".into()),
        }],
    };
    assert!(map_history(&invalid_rate).is_err());
}

#[test]
fn maps_current_quote_and_empty_quote_to_one_shared_shape() {
    let quote = map_latest(Some((
        "09.10.2026".into(),
        DailyRate {
            char_code: Some("USD".into()),
            nominal: Some("1".into()),
            value: Some("92,5000".into()),
        },
    )))
    .unwrap();
    assert_eq!(
        quote.date,
        Some(
            Utc.from_utc_datetime(
                &NaiveDate::from_ymd_opt(2026, 10, 9)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            )
        )
    );
    assert_eq!(quote.close, Some(92.5));
    assert_eq!(quote.high, None);
    assert_eq!(quote.low, None);
    assert_eq!(quote.volume, None);
    assert_eq!(quote.facevalue, Some(1.0));
    assert_eq!(map_latest(None).unwrap().date, None);
}
