use exchange_api::{
    cbr::{client::CbrClient, provider::CbrProvider},
    domain::{DailyMarketRecord, LatestQuoteRecord},
    moex::{client::MoexClient, provider::MoexProvider},
    provider::{ExchangeProvider, ProviderError},
    spbex::{client::SpbexClient, provider::SpbexProvider},
};
use std::time::Duration;

fn assert_shared_contract(_: &dyn ExchangeProvider) {}

#[tokio::test]
async fn each_provider_implements_normalization_and_error_code_contract() {
    let moex =
        MoexProvider(MoexClient::new("https://example.test/", Duration::from_secs(1)).unwrap());
    let spbex = SpbexProvider(
        SpbexClient::new("https://example.test/api/", Duration::from_secs(1), 1024).unwrap(),
    );
    let cbr =
        CbrProvider(CbrClient::new("https://example.test/", Duration::from_secs(1), 1024).unwrap());
    for provider in [&moex as &dyn ExchangeProvider, &spbex, &cbr] {
        assert_shared_contract(provider);
    }
    assert_eq!(moex.normalize_symbol(" sber ").as_deref(), Some("SBER"));
    assert_eq!(spbex.normalize_symbol(" sber ").as_deref(), Some("SBER"));
    assert_eq!(cbr.normalize_symbol(" usd ").as_deref(), Some("USD"));
    assert_eq!(
        (
            moex.upstream_code(),
            spbex.upstream_code(),
            cbr.upstream_code()
        ),
        ("moex_unavailable", "spbex_unavailable", "cbr_unavailable")
    );
    assert!(moex.normalize_symbol("bad ticker").is_none());
    assert!(cbr.normalize_symbol("USDD").is_none());
}

#[test]
fn shared_records_keep_the_six_nullable_public_fields() {
    let daily = DailyMarketRecord {
        date: chrono::Utc::now(),
        close: None,
        high: None,
        low: None,
        volume: None,
        facevalue: None,
    };
    let quote = LatestQuoteRecord::no_trade();
    for value in [
        serde_json::to_value(daily).unwrap(),
        serde_json::to_value(quote).unwrap(),
    ] {
        for field in ["date", "close", "high", "low", "volume", "facevalue"] {
            assert!(value.get(field).is_some(), "missing field {field}");
        }
    }
}

#[test]
fn provider_errors_have_neutral_categories() {
    let invalid = ProviderError::InvalidSymbol;
    let upstream = ProviderError::Upstream("timeout".to_owned());
    assert!(matches!(invalid, ProviderError::InvalidSymbol));
    assert!(matches!(upstream, ProviderError::Upstream(_)));
}
