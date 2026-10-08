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
