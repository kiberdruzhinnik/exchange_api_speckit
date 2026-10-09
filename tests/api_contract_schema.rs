use serde_yaml::Value;

#[test]
fn openapi_describes_history_and_quote_array_contracts() {
    let contract: Value = serde_yaml::from_str(include_str!(
        "../specs/001-moex-ticker-update/contracts/openapi.yaml"
    ))
    .unwrap();
    let paths = contract.get("paths").unwrap();
    for path in ["/v1/moex/{SYMBOL}", "/v1/moex/{SYMBOL}/quote"] {
        let schema =
            &paths[path]["get"]["responses"]["200"]["content"]["application/json"]["schema"];
        assert_eq!(schema["type"], "array");
    }
    let fields = ["date", "close", "high", "low", "volume", "facevalue"];
    for schema_name in ["DailyMarketRecord", "LatestTradeQuoteRecord"] {
        let required = contract["components"]["schemas"][schema_name]["required"]
            .as_sequence()
            .unwrap();
        for field in fields {
            assert!(
                required.iter().any(|v| v.as_str() == Some(field)),
                "{schema_name} missing {field}"
            );
        }
    }
}

#[test]
fn spbex_contract_documents_history_quote_and_distinct_errors() {
    let contract: Value = serde_yaml::from_str(include_str!(
        "../specs/002-spbex-ticker-update/contracts/openapi.yaml"
    ))
    .unwrap();
    let paths = contract.get("paths").unwrap();
    for path in ["/v1/spbex/{SYMBOL}", "/v1/spbex/{SYMBOL}/quote"] {
        let get = &paths[path]["get"];
        let schema = &get["responses"]["200"]["content"]["application/json"]["schema"];
        assert_eq!(schema["type"], "array");
        assert!(get["responses"].get("400").is_some());
        assert!(get["responses"].get("502").is_some());
    }
    assert!(
        paths["/v1/spbex/{SYMBOL}"]["get"]["responses"]
            .get("503")
            .is_some()
    );
    let history = &paths["/v1/spbex/{SYMBOL}"]["get"];
    let quote = &paths["/v1/spbex/{SYMBOL}/quote"]["get"];
    assert!(
        history["description"]
            .as_str()
            .unwrap()
            .contains("current UTC calendar date")
    );
    assert!(
        history["responses"]["200"]["description"]
            .as_str()
            .unwrap()
            .contains("Candles dated today are excluded")
    );
    assert!(
        quote["description"]
            .as_str()
            .unwrap()
            .contains("including a candle dated on the current UTC calendar date")
    );
    assert!(
        contract["components"]["schemas"]["DailyMarketRecord"]["properties"]["volume"]["type"]
            .as_str()
            == Some("null")
    );
}

#[test]
fn cbr_contract_documents_history_quote_nullability_and_errors() {
    let contract: Value = serde_yaml::from_str(include_str!(
        "../specs/003-cbr-currency-rates/contracts/openapi.yaml"
    ))
    .unwrap();
    let paths = contract.get("paths").unwrap();
    for path in ["/v1/cbr/{SYMBOL}", "/v1/cbr/{SYMBOL}/quote"] {
        let get = &paths[path]["get"];
        let schema = &get["responses"]["200"]["content"]["application/json"]["schema"];
        assert_eq!(schema["type"], "array");
        assert!(get["responses"].get("400").is_some());
        assert!(get["responses"].get("502").is_some());
    }
    assert!(
        paths["/v1/cbr/{SYMBOL}"]["get"]["responses"]
            .get("503")
            .is_some()
    );
    let fields = ["date", "close", "high", "low", "volume", "facevalue"];
    for schema_name in ["DailyCurrencyRate", "LatestCurrencyQuote"] {
        let required = contract["components"]["schemas"][schema_name]["required"]
            .as_sequence()
            .unwrap();
        for field in fields {
            assert!(required.iter().any(|value| value.as_str() == Some(field)));
        }
    }
    let close_type = contract["components"]["schemas"]["DailyCurrencyRate"]["properties"]["close"]
        ["type"]
        .as_sequence()
        .unwrap();
    assert!(
        close_type
            .iter()
            .any(|value| value.as_str() == Some("number"))
    );
    assert!(
        close_type
            .iter()
            .any(|value| value.as_str() == Some("null"))
    );
}
