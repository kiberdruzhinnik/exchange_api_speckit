use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use exchange_api::{AppState, http::router, moex::client::MoexClient};
use std::time::Duration;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::test]
async fn maps_history_store_failure_to_service_unavailable() {
    use axum::response::IntoResponse;
    let response =
        exchange_api::http::errors::ApiError::Store("database unavailable".into()).into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn keeps_spbex_and_history_store_errors_distinct_in_json_envelope() {
    use axum::response::IntoResponse;
    let spbex =
        exchange_api::http::errors::ApiError::Spbex(anyhow::anyhow!("upstream unavailable"))
            .into_response();
    assert_eq!(spbex.status(), StatusCode::BAD_GATEWAY);
    let spbex_body = axum::body::to_bytes(spbex.into_body(), 4096).await.unwrap();
    let spbex_json: serde_json::Value = serde_json::from_slice(&spbex_body).unwrap();
    assert_eq!(spbex_json["error"]["code"], "spbex_unavailable");

    let store =
        exchange_api::http::errors::ApiError::Store("sqlite unavailable".into()).into_response();
    assert_eq!(store.status(), StatusCode::SERVICE_UNAVAILABLE);
    let store_body = axum::body::to_bytes(store.into_body(), 4096).await.unwrap();
    let store_json: serde_json::Value = serde_json::from_slice(&store_body).unwrap();
    assert_eq!(store_json["error"]["code"], "history_store_unavailable");
}

#[tokio::test]
async fn rejects_malformed_symbols() {
    let server = MockServer::start().await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/bad%20ticker")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "invalid_symbol");
    assert!(json["error"]["message"].is_string());
}

#[tokio::test]
async fn maps_upstream_failure_to_bad_gateway() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

fn security_fixture() -> &'static str {
    include_str!("fixtures/moex/security-description.json")
}

#[tokio::test]
async fn maps_malformed_upstream_json_to_bad_gateway() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn maps_upstream_timeout_to_bad_gateway() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(security_fixture())
                .set_delay(Duration::from_millis(100)),
        )
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_millis(10)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn maps_upstream_server_error_to_bad_gateway() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn maps_history_row_without_date_to_bad_gateway() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(security_fixture()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/history/engines/stock/markets/shares/securities/SBER.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR",null,1,1,1,1]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn maps_unknown_symbol_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/ZZZZUNKNOWN.json"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let response = router(AppState::new(client))
        .oneshot(
            Request::builder()
                .uri("/v1/moex/ZZZZUNKNOWN")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn shared_routes_preserve_each_providers_upstream_error_code() {
    use exchange_api::{cbr::client::CbrClient, spbex::client::SpbexClient};
    use wiremock::matchers::any;

    let moex_server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(503))
        .mount(&moex_server)
        .await;
    let spbex_server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(503))
        .mount(&spbex_server)
        .await;
    let cbr_server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(503))
        .mount(&cbr_server)
        .await;
    let state = AppState::with_all_services(
        MoexClient::new(&format!("{}/", moex_server.uri()), Duration::from_secs(1)).unwrap(),
        SpbexClient::new(
            &format!("{}/", spbex_server.uri()),
            Duration::from_secs(1),
            4096,
        )
        .unwrap(),
        CbrClient::new(
            &format!("{}/", cbr_server.uri()),
            Duration::from_secs(1),
            4096,
        )
        .unwrap(),
        Duration::from_secs(60),
        1024 * 1024,
    );
    for (path, expected_code) in [
        ("/v1/moex/SBER", "moex_unavailable"),
        ("/v1/spbex/SBER", "spbex_unavailable"),
        ("/v1/cbr/USD", "cbr_unavailable"),
    ] {
        let response = router(state.clone())
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["error"]["code"], expected_code,
            "wrong code for {path}"
        );
        assert!(json["error"]["message"].is_string());
    }
}
