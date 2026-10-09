use serde_yaml::Value;
use std::path::{Path, PathBuf};

const FEATURE_CONTRACTS: &[(&str, &[(&str, &str)])] = &[
    (
        "specs/001-moex-ticker-update/contracts/openapi.yaml",
        &[
            ("/v1/moex/{SYMBOL}", "moex_unavailable"),
            ("/v1/moex/{SYMBOL}/quote", "moex_unavailable"),
        ],
    ),
    (
        "specs/002-spbex-ticker-update/contracts/openapi.yaml",
        &[
            ("/v1/spbex/{SYMBOL}", "spbex_unavailable"),
            ("/v1/spbex/{SYMBOL}/quote", "spbex_unavailable"),
        ],
    ),
    (
        "specs/003-cbr-currency-rates/contracts/openapi.yaml",
        &[
            ("/v1/cbr/{SYMBOL}", "cbr_unavailable"),
            ("/v1/cbr/{SYMBOL}/quote", "cbr_unavailable"),
        ],
    ),
];

fn external_path_item<'a>(
    feature_file: &Path,
    feature: &'a Value,
    path: &str,
    canonical: &'a Value,
) -> &'a Value {
    let reference = feature["paths"][path]["$ref"]
        .as_str()
        .expect("feature path must reference canonical contract");
    let (file, pointer) = reference
        .split_once('#')
        .expect("reference has JSON pointer");
    let resolved = feature_file.parent().unwrap().join(file);
    assert!(
        resolved.exists(),
        "external reference target does not exist: {}",
        resolved.display()
    );
    assert_eq!(
        resolved.file_name().and_then(|s| s.to_str()),
        Some("openapi.yaml")
    );
    let mut value = canonical;
    for segment in pointer.trim_start_matches('/').split('/') {
        let key = segment.replace("~1", "/").replace("~0", "~");
        value = &value[&key];
    }
    value
}

#[test]
fn each_feature_contract_resolves_its_two_paths_in_the_canonical_contract() {
    let canonical: Value =
        serde_yaml::from_str(include_str!("../specs/contracts/openapi.yaml")).unwrap();
    let fields = ["date", "close", "high", "low", "volume", "facevalue"];
    for (relative, expected_paths) in FEATURE_CONTRACTS {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
        let text = std::fs::read_to_string(&file).unwrap();
        let feature: Value = serde_yaml::from_str(&text).unwrap();
        for (path, upstream_code) in *expected_paths {
            let operation = external_path_item(&file, &feature, path, &canonical);
            let get = &operation["get"];
            assert!(
                get["responses"]["400"].is_mapping(),
                "{path} missing 400 response"
            );
            assert!(
                get["responses"]["502"].is_mapping(),
                "{path} missing 502 response"
            );
            assert_eq!(
                get["responses"]["502"]["$ref"].as_str(),
                Some(match *upstream_code {
                    "moex_unavailable" => "#/components/responses/MoexUpstreamUnavailable",
                    "spbex_unavailable" => "#/components/responses/SpbexUpstreamUnavailable",
                    _ => "#/components/responses/CbrUpstreamUnavailable",
                })
            );
            if !path.ends_with("/quote") {
                assert!(
                    get["responses"]["503"].is_mapping(),
                    "{path} missing history-store failure response"
                );
            }
            let schema = &get["responses"]["200"]["content"]["application/json"]["schema"];
            assert_eq!(schema["type"].as_str(), Some("array"));
            let record = if path.ends_with("/quote") {
                "LatestQuoteRecord"
            } else {
                "DailyMarketRecord"
            };
            let required = canonical["components"]["schemas"][record]["required"]
                .as_sequence()
                .unwrap();
            for field in fields {
                assert!(
                    required.iter().any(|item| item.as_str() == Some(field)),
                    "{record} missing {field}"
                );
            }
        }
    }
}

#[test]
fn canonical_contract_has_all_six_routes_and_shared_envelope() {
    let contract: Value =
        serde_yaml::from_str(include_str!("../specs/contracts/openapi.yaml")).unwrap();
    for path in [
        "/v1/moex/{SYMBOL}",
        "/v1/moex/{SYMBOL}/quote",
        "/v1/spbex/{SYMBOL}",
        "/v1/spbex/{SYMBOL}/quote",
        "/v1/cbr/{SYMBOL}",
        "/v1/cbr/{SYMBOL}/quote",
        "/v2/history/{PROVIDER}/{SYMBOL}",
        "/v2/quote/{PROVIDER}/{SYMBOL}",
    ] {
        assert!(
            contract["paths"][path]["get"].is_mapping(),
            "missing {path}"
        );
    }
    for response in [
        "InvalidSymbol",
        "MoexUpstreamUnavailable",
        "SpbexUpstreamUnavailable",
        "CbrUpstreamUnavailable",
        "HistoryStoreUnavailable",
        "InvalidV2PathParameter",
        "ProviderUnavailable",
    ] {
        assert!(contract["components"]["responses"][response].is_mapping());
    }
    assert!(contract["components"]["schemas"]["ErrorResponse"]["properties"]["error"]["properties"]["code"].is_mapping());
    assert!(contract["components"]["schemas"]["ErrorResponse"]["properties"]["error"]["properties"]["message"].is_mapping());
    assert_eq!(
        contract["paths"]["/v2/history/{PROVIDER}/{SYMBOL}"]["get"]["parameters"][0]["$ref"],
        "#/components/parameters/Provider"
    );
    assert_eq!(
        contract["components"]["parameters"]["Provider"]["schema"]["enum"],
        serde_yaml::to_value(["moex", "spbex", "cbr"]).unwrap()
    );
    assert_eq!(
        contract["components"]["responses"]["InvalidV2PathParameter"]["content"]["application/json"]
            ["examples"]["invalidProvider"]["value"]["error"]["code"],
        "invalid_provider"
    );

    let feature: Value = serde_yaml::from_str(include_str!(
        "../specs/005-v2-history-quote-api/contracts/openapi-v2.yaml"
    ))
    .unwrap();
    for path in [
        "/v2/history/{PROVIDER}/{SYMBOL}",
        "/v2/quote/{PROVIDER}/{SYMBOL}",
    ] {
        assert!(feature["paths"][path]["get"].is_mapping(), "missing {path}");
    }
}
