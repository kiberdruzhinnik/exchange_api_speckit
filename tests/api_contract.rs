use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use exchange_api::{AppState, http::router, moex::client::MoexClient};
use serde_json::Value;
use std::time::{Duration, Instant};
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

async fn app_with_history(body: &'static str) -> (MockServer, axum::Router) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
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
            "/engines/stock/markets/shares/boards/TQBR/securities/SBER.json",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-tqbr.json")),
        )
        .mount(&server)
        .await;
    // Security reference and listing metadata share one ISS document in production.
    let client = MoexClient::new(
        &format!("{}/", server.uri()),
        std::time::Duration::from_secs(2),
    )
    .unwrap();
    (server, router(AppState::new(client)))
}

#[tokio::test]
async fn returns_six_field_array() {
    let body = include_str!("fixtures/moex/history-page.json");
    let (server, app) = app_with_history(body).await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value[0]["date"], "2026-10-06T00:00:00Z");
    assert_eq!(value[0]["close"], 282.22);
    assert_eq!(value[0]["high"], 284.0);
    assert_eq!(value[0]["low"], 280.12);
    assert_eq!(value[0]["volume"], 22099060.0);
    assert_eq!(value[0]["facevalue"], 1.0);
    for key in ["date", "close", "high", "low", "volume", "facevalue"] {
        assert!(value[0].get(key).is_some());
    }

    let cached_response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cached_response.status(), StatusCode::OK);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn returns_empty_array_when_no_history() {
    let body = include_str!("fixtures/moex/history-empty.json");
    let (_, app) = app_with_history(body).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        serde_json::json!([])
    );
}

#[tokio::test]
async fn warm_response_cache_meets_one_second_local_p95() {
    let (server, app) = app_with_history(include_str!("fixtures/moex/history-page.json")).await;
    let mut samples = Vec::new();
    for _ in 0..101 {
        let started = Instant::now();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/moex/SBER")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(!body.is_empty());
        samples.push(started.elapsed());
    }
    samples.sort();
    let p95 = samples[95];
    assert!(p95 < Duration::from_secs(1), "local p95 was {p95:?}");
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}
