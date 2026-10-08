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
