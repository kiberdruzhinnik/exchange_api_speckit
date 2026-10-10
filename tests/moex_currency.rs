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
    matchers::{method, path},
};

fn fixture(name: &str) -> &'static str {
    match name {
        "security" => include_str!("fixtures/moex/gldrub_tom-security.json"),
        "history" => include_str!("fixtures/moex/gldrub_tom-history.json"),
        "marketdata" => include_str!("fixtures/moex/gldrub_tom-marketdata.json"),
        _ => unreachable!(),
    }
}

async fn app_with(
    history: &'static str,
    marketdata_status: u16,
    marketdata: &'static str,
) -> (MockServer, axum::Router) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/GLDRUB_TOM.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("security")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/currency/markets/selt/boards/CETS/securities/GLDRUB_TOM.json",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(history))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/currency/markets/selt/boards/CETS/securities/GLDRUB_TOM.json",
        ))
        .respond_with(ResponseTemplate::new(marketdata_status).set_body_string(marketdata))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    (server, router(AppState::new(client)))
}

async fn app() -> (MockServer, axum::Router) {
    app_with(fixture("history"), 200, fixture("marketdata")).await
}

#[tokio::test]
async fn serves_gldrub_tom_history_from_its_primary_currency_board() {
    let (server, app) = app().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let rows: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["close"].as_f64(), Some(80.0));
    assert_eq!(rows[0]["high"].as_f64(), Some(81.0));
    assert_eq!(rows[0]["low"].as_f64(), Some(79.0));
    assert!(rows[0]["volume"].is_null());
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .any(|request| request.url.path().contains("/boards/CETS/"))
    );
}

#[tokio::test]
async fn serves_currency_quote_with_fetch_time_trade_count() {
    let (_, app) = app().await;
    for path in ["/v1/moex/GLDRUB_TOM/quote", "/v2/quote/moex/GLDRUB_TOM"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rows: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(rows[0]["close"].as_f64(), Some(81.25));
        assert_eq!(rows[0]["volume"].as_f64(), Some(7.0));
        assert!(rows[0]["date"].is_string());
        assert!(rows[0]["high"].is_null());
        assert!(rows[0]["low"].is_null());
        assert!(rows[0]["facevalue"].is_null());
    }
}

#[tokio::test]
async fn returns_empty_currency_history_and_all_null_quote_when_no_trade_exists() {
    let (_, app) = app_with(
        r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW"],"data":[]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,0,100]]}}"#,
        200,
        r#"{"securities":{"columns":["LOTSIZE"],"data":[[1000]]},"marketdata":{"columns":["LAST","TIME","QTY"],"data":[]}}"#,
    ).await;
    let history = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(history.status(), StatusCode::OK);
    let history_body = axum::body::to_bytes(history.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&history_body).unwrap(),
        serde_json::json!([])
    );
    let quote = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(quote.status(), StatusCode::OK);
    let quote_body = axum::body::to_bytes(quote.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&quote_body).unwrap(),
        serde_json::json!([{"date":null,"close":null,"high":null,"low":null,"volume":null,"facevalue":null}])
    );
}

#[tokio::test]
async fn currency_quote_leaves_volume_null_when_numtrades_is_unavailable() {
    let (_, app) = app_with(
        fixture("history"),
        200,
        r#"{"securities":{"columns":["LOTSIZE"],"data":[[1000]]},"marketdata":{"columns":["LAST","TIME","QTY"],"data":[[81.25,"12:34:56",3]]}}"#,
    )
    .await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let rows: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rows[0]["close"].as_f64(), Some(81.25));
    assert!(rows[0]["volume"].is_null());
}

#[tokio::test]
async fn malformed_currency_quote_time_returns_dependency_error() {
    let (_, app) = app_with(
        fixture("history"),
        200,
        r#"{"securities":{"columns":["LOTSIZE"],"data":[[1000]]},"marketdata":{"columns":["LAST","TIME","NUMTRADES"],"data":[[81.25,"not-a-time",7]]}}"#,
    )
    .await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn malformed_non_null_currency_last_returns_dependency_error() {
    let (_, app) = app_with(
        fixture("history"),
        200,
        r#"{"securities":{"columns":["LOTSIZE"],"data":[[1000]]},"marketdata":{"columns":["LAST","TIME","NUMTRADES"],"data":[["bad","12:34:56",7]]}}"#,
    )
    .await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn malformed_non_null_currency_history_value_returns_dependency_error() {
    let history = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW"],"data":[["CETS","2026-10-01","bad",81,79]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#;
    let (_, app) = app_with(
        history,
        200,
        r#"{"marketdata":{"columns":["LAST","TIME","NUMTRADES"],"data":[]}}"#,
    )
    .await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn maps_currency_current_marketdata_failure_to_dependency_error() {
    let (_, app) = app_with(fixture("history"), 503, "{}").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/GLDRUB_TOM/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}
