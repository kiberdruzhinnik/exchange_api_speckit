use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use exchange_api::{AppState, http::router, moex::client::MoexClient};
use serde_json::Value;
use std::time::Duration;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

async fn quote_app(trades: &'static str) -> (MockServer, axum::Router) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-description.json")),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/stock/markets/shares/securities/SBER/trades.json",
        ))
        .and(query_param("limit", "1"))
        .and(query_param("reversed", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(trades))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    (server, router(AppState::new(client)))
}

#[tokio::test]
async fn returns_latest_trade_as_one_history_shaped_record_and_fetches_each_time() {
    let (server, app) = quote_app(include_str!("fixtures/moex/trades-latest.json")).await;
    for route in [
        "/v2/quote/moex/SBER",
        "/v2/quote/moex/SBER",
        "/v1/moex/SBER/quote",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 1);
        assert_eq!(value[0]["date"], "2026-10-08T08:41:57Z");
        assert_eq!(value[0]["close"], 280.34);
        assert_eq!(value[0]["volume"].as_f64(), Some(1.0));
        assert!(value[0]["high"].is_null());
        assert!(value[0]["low"].is_null());
        assert!(value[0]["facevalue"].is_null());
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 6);
}

#[tokio::test]
async fn rejects_unknown_v2_quote_provider_without_fetching() {
    let server = MockServer::start().await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v2/quote/unknown/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"]["code"], "invalid_provider");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn returns_one_all_null_record_for_valid_empty_trade_block() {
    let (_, app) = quote_app(include_str!("fixtures/moex/trades-empty.json")).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        value,
        serde_json::json!([{"date":null,"close":null,"high":null,"low":null,"volume":null,"facevalue":null}])
    );
}

#[tokio::test]
async fn rejects_failed_trade_response_as_dependency_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-description.json")),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/stock/markets/shares/securities/SBER/trades.json",
        ))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let app = router(AppState::new(client));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn rejects_malformed_trade_rows_as_dependency_error() {
    let (server, _) = quote_app(
        r#"{"trades":{"columns":["TRADEDATE","TRADETIME","PRICE","QUANTITY"],"data":[["2026-10-08","11:41:57",null,1]]}}"#,
    )
    .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn rejects_unknown_symbols_without_requesting_trades() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/stock/markets/shares/securities/UNKNOWN/trades.json",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/UNKNOWN/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
