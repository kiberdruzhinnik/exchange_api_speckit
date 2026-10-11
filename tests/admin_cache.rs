use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use bytes::Bytes;
use exchange_api::{
    AppState, cache_store::CacheStore, domain::DailyMarketRecord, http::router,
    moex::client::MoexClient,
};
use serde_json::Value;
use std::time::Duration;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn record(symbol_value: f64) -> DailyMarketRecord {
    DailyMarketRecord {
        date: chrono::DateTime::parse_from_rfc3339("2026-10-06T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        close: Some(symbol_value),
        high: None,
        low: None,
        volume: None,
        facevalue: None,
    }
}

async fn app(key: Option<&str>) -> (tempfile::TempDir, AppState, axum::Router) {
    let directory = tempfile::tempdir().unwrap();
    let store = CacheStore::open(directory.path().join("history.sqlite3"), 1024 * 1024)
        .await
        .unwrap();
    let client = MoexClient::new("http://127.0.0.1:1/", Duration::from_secs(1)).unwrap();
    let state = AppState::with_store(client, Duration::from_secs(60), 1024 * 1024, store)
        .with_admin_api_key(key.map(str::to_owned));
    let router = router(state.clone());
    (directory, state, router)
}

fn clear_request(path: &str, key: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().method("DELETE").uri(path);
    if let Some(key) = key {
        request = request.header("X-API-Key", key);
    }
    request.body(Body::empty()).unwrap()
}

async fn error_code(response: axum::response::Response) -> String {
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    value["error"]["code"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn requires_key_before_validating_target_and_fails_closed_when_unconfigured() {
    let (_directory, _state, router) = app(Some("secret")).await;
    let missing = router
        .clone()
        .oneshot(clear_request("/v2/admin/cache/clear/unknown/%20", None))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(missing).await, "unauthorized");

    let incorrect = router
        .oneshot(clear_request(
            "/v2/admin/cache/clear/moex/SBER",
            Some("wrong"),
        ))
        .await
        .unwrap();
    assert_eq!(incorrect.status(), StatusCode::UNAUTHORIZED);

    let (_directory, _state, unconfigured) = app(None).await;
    let unavailable = unconfigured
        .oneshot(clear_request(
            "/v2/admin/cache/clear/moex/SBER",
            Some("secret"),
        ))
        .await
        .unwrap();
    assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(error_code(unavailable).await, "admin_api_key_unconfigured");
}

#[tokio::test]
async fn clears_only_targeted_provider_symbol_across_all_cache_layers() {
    let (_directory, state, router) = app(Some("secret")).await;
    let store = state.cache_store.as_ref().unwrap();
    store
        .merge_records("moex", "SBER", &[record(10.0)], true, 10)
        .await
        .unwrap();
    store
        .merge_records("moex", "GAZP", &[record(20.0)], true, 10)
        .await
        .unwrap();
    store
        .merge_records("cbr", "SBER", &[record(30.0)], true, 10)
        .await
        .unwrap();
    store
        .put("MOEX:SBER", &Bytes::from_static(b"legacy"), Duration::ZERO)
        .await
        .unwrap();
    state
        .history_cache
        .get_or_fetch("MOEX:SBER".into(), || async {
            Ok(Bytes::from_static(b"memory-sber"))
        })
        .await
        .unwrap();
    state
        .history_cache
        .get_or_fetch("MOEX:GAZP".into(), || async {
            Ok(Bytes::from_static(b"memory-gazp"))
        })
        .await
        .unwrap();

    let response = router
        .clone()
        .oneshot(clear_request(
            "/v2/admin/cache/clear/moex/sber",
            Some("secret"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(store.collection("moex", "SBER").await.unwrap().is_none());
    assert!(store.get("MOEX:SBER").await.unwrap().is_none());
    assert!(state.history_cache.get("MOEX:SBER").await.is_none());
    assert!(store.collection("moex", "GAZP").await.unwrap().is_some());
    assert!(store.collection("cbr", "SBER").await.unwrap().is_some());
    assert_eq!(
        state.history_cache.get("MOEX:GAZP").await.unwrap(),
        Bytes::from_static(b"memory-gazp")
    );

    let repeat = router
        .oneshot(clear_request(
            "/v2/admin/cache/clear/moex/SBER",
            Some("secret"),
        ))
        .await
        .unwrap();
    assert_eq!(repeat.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn authorized_requests_validate_provider_and_symbol_without_mutation() {
    let (_directory, state, router) = app(Some("secret")).await;
    state
        .cache_store
        .as_ref()
        .unwrap()
        .merge_records("moex", "SBER", &[record(10.0)], true, 10)
        .await
        .unwrap();

    let unknown = router
        .clone()
        .oneshot(clear_request(
            "/v2/admin/cache/clear/unknown/SBER",
            Some("secret"),
        ))
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(unknown).await, "invalid_provider");

    let invalid_symbol = router
        .oneshot(clear_request(
            "/v2/admin/cache/clear/moex/bad%20symbol",
            Some("secret"),
        ))
        .await
        .unwrap();
    assert_eq!(invalid_symbol.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(invalid_symbol).await, "invalid_symbol");
    assert!(
        state
            .cache_store
            .as_ref()
            .unwrap()
            .collection("moex", "SBER")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn cleared_history_is_fetched_and_retained_again_on_next_history_request() {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-description.json")),
        )
        .mount(&upstream)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/stock/markets/shares/boards/TQBR/securities/SBER.json",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-tqbr.json")),
        )
        .mount(&upstream)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .and(query_param("start", "0"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/history-page.json")),
        )
        .mount(&upstream)
        .await;

    let directory = tempfile::tempdir().unwrap();
    let store = CacheStore::open(directory.path().join("history.sqlite3"), 1024 * 1024)
        .await
        .unwrap();
    store
        .merge_records("moex", "SBER", &[record(1.0)], true, 10)
        .await
        .unwrap();
    let client = MoexClient::new(&format!("{}/", upstream.uri()), Duration::from_secs(2)).unwrap();
    let state = AppState::with_store(client, Duration::from_secs(60), 1024 * 1024, store)
        .with_admin_api_key(Some("secret".to_owned()));
    let app = router(state.clone());

    let cleared = app
        .clone()
        .oneshot(clear_request(
            "/v2/admin/cache/clear/moex/SBER",
            Some("secret"),
        ))
        .await
        .unwrap();
    assert_eq!(cleared.status(), StatusCode::NO_CONTENT);
    assert!(
        state
            .cache_store
            .as_ref()
            .unwrap()
            .collection("moex", "SBER")
            .await
            .unwrap()
            .is_none()
    );

    let fetched = app
        .oneshot(
            Request::builder()
                .uri("/v2/history/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    let collection = state
        .cache_store
        .as_ref()
        .unwrap()
        .collection("moex", "SBER")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(collection.records[0].close, Some(282.22));
    assert_eq!(upstream.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn persistent_delete_failure_returns_503_and_keeps_the_collection() {
    let (directory, state, router) = app(Some("secret")).await;
    let store = state.cache_store.as_ref().unwrap();
    store
        .merge_records("moex", "SBER", &[record(10.0)], true, 10)
        .await
        .unwrap();
    let admin = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(directory.path().join("history.sqlite3"))
            .create_if_missing(false),
    )
    .await
    .unwrap();
    sqlx::query("CREATE TRIGGER prevent_admin_delete BEFORE DELETE ON history_collections BEGIN SELECT RAISE(ABORT, 'injected delete failure'); END")
        .execute(&admin)
        .await
        .unwrap();

    let response = router
        .oneshot(clear_request(
            "/v2/admin/cache/clear/moex/SBER",
            Some("secret"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(error_code(response).await, "history_store_unavailable");
    assert!(store.collection("moex", "SBER").await.unwrap().is_some());
    admin.close().await;
}
