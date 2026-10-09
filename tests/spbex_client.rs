use exchange_api::spbex::{
    client::{SpbexClient, SpbexError},
    validation::normalize_symbol,
};
use std::time::Duration;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

const PATH: &str = "/api/reader/marketdata/charts/chistory";

#[tokio::test]
async fn bounds_history_from_latest_record_timestamp() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .and(query_param("from", "1704153601"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&server)
        .await;
    let client = SpbexClient::new(
        &format!("{}/api/", server.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    let after = chrono::DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert!(
        client
            .history_since("SBER", Some(after))
            .await
            .unwrap()
            .is_empty()
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests[0]
            .url
            .query_pairs()
            .any(|(key, value)| key == "from" && value == "1704153601")
    );
}

#[tokio::test]
async fn requests_daily_chart_history_with_normalized_symbol_and_full_range() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .and(query_param("symbol", "SBER"))
        .and(query_param("resolution", "1440"))
        .and(query_param("from", "0"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/spbex/history.json")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = SpbexClient::new(
        &format!("{}/api", server.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    assert_eq!(client.history(" sber ").await.unwrap().len(), 2);
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].url.query().unwrap().contains("to="));
}

#[tokio::test]
async fn accepts_successful_empty_feed_and_rejects_explicit_symbol_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .expect(1)
        .mount(&server)
        .await;
    let client = SpbexClient::new(
        &format!("{}/api/", server.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    assert!(client.history("SBER").await.unwrap().is_empty());

    let rejected = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(404))
        .mount(&rejected)
        .await;
    let client = SpbexClient::new(
        &format!("{}/api/", rejected.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    assert!(matches!(
        client.history("NOPE").await,
        Err(SpbexError::InvalidSymbol)
    ));
}

#[tokio::test]
async fn rejects_failed_malformed_and_oversized_responses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;
    let client = SpbexClient::new(
        &format!("{}/api/", server.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    assert!(matches!(
        client.history("SBER").await,
        Err(SpbexError::HttpStatus(502))
    ));

    let malformed = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&malformed)
        .await;
    let client = SpbexClient::new(
        &format!("{}/api/", malformed.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    assert!(matches!(
        client.history("SBER").await,
        Err(SpbexError::InvalidData(_))
    ));

    let large = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string("[1234567890]"))
        .mount(&large)
        .await;
    let client =
        SpbexClient::new(&format!("{}/api/", large.uri()), Duration::from_secs(1), 5).unwrap();
    assert!(
        matches!(client.history("SBER").await, Err(SpbexError::Upstream(message)) if message.contains("byte limit"))
    );
}

#[test]
fn normalizes_and_rejects_symbols() {
    assert_eq!(normalize_symbol(" sber ").as_deref(), Some("SBER"));
    assert_eq!(normalize_symbol("bad ticker"), None);
}
