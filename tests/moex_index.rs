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
        "security" => include_str!("fixtures/moex/imoex-security.json"),
        "history" => include_str!("fixtures/moex/imoex-history.json"),
        "marketdata" => include_str!("fixtures/moex/imoex-marketdata.json"),
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
        .and(path("/securities/IMOEX.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("security")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/index/boards/SNDX/securities/IMOEX.json",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(history))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/stock/markets/index/boards/SNDX/securities/IMOEX.json",
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
async fn serves_imoex_history_on_v1_and_v2_with_index_values() {
    let (_, app) = app().await;
    for path in ["/v1/moex/IMOEX", "/v2/history/moex/IMOEX"] {
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
        assert_eq!(rows.as_array().unwrap().len(), 2);
        assert_eq!(rows[0]["date"], "2026-10-01T00:00:00Z");
        assert_eq!(rows[0]["close"].as_f64(), Some(3200.0));
        assert_eq!(rows[0]["high"].as_f64(), Some(3210.0));
        assert_eq!(rows[0]["low"].as_f64(), Some(3190.0));
        assert_eq!(rows[0]["volume"].as_f64(), Some(0.0));
        assert!(rows[0]["facevalue"].is_null());
    }
}

#[tokio::test]
async fn serves_imoex_published_current_value_without_a_trade() {
    let (_, app) = app().await;
    for path in ["/v1/moex/IMOEX/quote", "/v2/quote/moex/IMOEX"] {
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
        assert_eq!(rows[0]["close"].as_f64(), Some(3215.5));
        assert!(rows[0]["date"].is_string());
        assert!(rows[0]["volume"].is_null());
        assert!(rows[0]["high"].is_null());
        assert!(rows[0]["low"].is_null());
        assert!(rows[0]["facevalue"].is_null());
    }
}

#[tokio::test]
async fn returns_empty_history_and_all_null_quote_when_index_has_no_published_value() {
    let (_, app) = app_with(
        r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,0,100]]}}"#,
        200,
        r#"{"marketdata":{"columns":["CURRENTVALUE","LASTVALUE","SYSTIME"],"data":[[null,null,null]]}}"#,
    ).await;
    let history = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/moex/IMOEX")
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
                .uri("/v1/moex/IMOEX/quote")
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
async fn maps_index_current_marketdata_failure_to_dependency_error() {
    let (_, app) = app_with(fixture("history"), 503, "{}").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/IMOEX/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn index_quote_with_malformed_present_timestamp_returns_dependency_error() {
    let (_, app) = app_with(
        fixture("history"),
        200,
        r#"{"marketdata":{"columns":["CURRENTVALUE","LASTVALUE","SYSTIME"],"data":[[3215.5,3210.0,"not-a-time"]]}}"#,
    )
    .await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/IMOEX/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn malformed_non_null_index_values_return_dependency_errors() {
    let cases = [
        r#"{"marketdata":{"columns":["CURRENTVALUE","LASTVALUE"],"data":[["bad",3210.0]]}}"#,
        r#"{"marketdata":{"columns":["CURRENTVALUE","LASTVALUE"],"data":[[null,"bad"]]}}"#,
    ];
    for marketdata in cases {
        let (_, app) = app_with(fixture("history"), 200, marketdata).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/moex/IMOEX/quote")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }
}

#[tokio::test]
async fn malformed_non_null_index_history_values_return_dependency_errors() {
    let cases = [
        r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["SNDX","2026-10-01","bad",3210,3190,0]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#,
        r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["SNDX","2026-10-01",3200,3210,3190,"bad"]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#,
    ];
    for history in cases {
        let (_, app) = app_with(
            history,
            200,
            r#"{"marketdata":{"columns":["CURRENTVALUE"],"data":[]}}"#,
        )
        .await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/moex/IMOEX")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }
}
