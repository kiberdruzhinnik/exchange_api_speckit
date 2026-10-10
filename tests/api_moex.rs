use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use bytes::Bytes;
use exchange_api::{AppState, cache_store::CacheStore, http::router, moex::client::MoexClient};
use std::time::Duration;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const METADATA: &str = r#"{"boards":{"columns":["ENGINE","MARKET","BOARDID","IS_PRIMARY","HISTORY_FROM","HISTORY_TILL"],"data":[["stock","index","SNDX",1,"2000-01-01","9999-12-31"]]}}"#;
const HISTORY: &str = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["SNDX","2026-10-02",101,102,99,0]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#;

#[tokio::test]
async fn quote_with_no_primary_board_today_returns_no_quote_without_marketdata_fetch() {
    let server = MockServer::start().await;
    let today = chrono::Utc::now()
        .with_timezone(&chrono_tz::Europe::Moscow)
        .date_naive();
    let yesterday = today.pred_opt().unwrap();
    let metadata = format!(
        r#"{{"boards":{{"columns":["ENGINE","MARKET","BOARDID","IS_PRIMARY","HISTORY_FROM","HISTORY_TILL"],"data":[["stock","index","SNDX",1,"2000-01-01","{yesterday}"]]}}}}"#
    );
    Mock::given(method("GET"))
        .and(path("/securities/IMOEX.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(metadata))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let app = router(AppState::new(client));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/IMOEX/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!([{"date":null,"close":null,"high":null,"low":null,"volume":null,"facevalue":null}])
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/securities/IMOEX.json");
}

fn record(date: &str, close: f64) -> exchange_api::domain::DailyMarketRecord {
    exchange_api::domain::DailyMarketRecord {
        date: chrono::DateTime::parse_from_rfc3339(date)
            .unwrap()
            .with_timezone(&chrono::Utc),
        close: Some(close),
        high: None,
        low: None,
        volume: None,
        facevalue: None,
    }
}

async fn mount_symbol(server: &MockServer, metadata_status: u16, metadata: &'static str) {
    Mock::given(method("GET"))
        .and(path("/securities/IMOEX.json"))
        .respond_with(ResponseTemplate::new(metadata_status).set_body_string(metadata))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/index/boards/SNDX/securities/IMOEX.json",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(HISTORY))
        .mount(server)
        .await;
}

#[tokio::test]
async fn recognized_index_migrates_before_fetch_and_reuses_preflight_metadata() {
    let server = MockServer::start().await;
    mount_symbol(&server, 200, METADATA).await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 1024)
        .await
        .unwrap();
    let cached = serde_json::to_vec(&vec![record("2026-10-01T00:00:00Z", 100.0)]).unwrap();
    store
        .put("MOEX:IMOEX", &Bytes::from(cached), Duration::ZERO)
        .await
        .unwrap();
    let app = router(AppState::with_store(
        client,
        Duration::ZERO,
        1024,
        store.clone(),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/IMOEX")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let records: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(records.as_array().unwrap().len(), 2);
    assert_eq!(records[0]["close"], 100.0);
    assert_eq!(records[1]["close"], 101.0);
    assert!(store.get("MOEX:IMOEX").await.unwrap().is_none());
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.path() == "/securities/IMOEX.json")
            .count(),
        1
    );
    assert!(
        requests
            .iter()
            .position(|request| request.url.path() == "/securities/IMOEX.json")
            .unwrap()
            < requests
                .iter()
                .position(|request| request.url.path().starts_with("/history/"))
                .unwrap()
    );
}

#[tokio::test]
async fn unknown_symbol_does_not_migrate_legacy_history() {
    let server = MockServer::start().await;
    mount_symbol(
        &server,
        200,
        r#"{"boards":{"columns":["ENGINE","MARKET","BOARDID","IS_PRIMARY"],"data":[]}}"#,
    )
    .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 1024)
        .await
        .unwrap();
    let cached = serde_json::to_vec(&vec![record("2026-10-01T00:00:00Z", 100.0)]).unwrap();
    store
        .put("MOEX:IMOEX", &Bytes::from(cached), Duration::ZERO)
        .await
        .unwrap();
    let response = router(AppState::with_store(
        client,
        Duration::ZERO,
        1024,
        store.clone(),
    ))
    .oneshot(
        Request::builder()
            .uri("/v1/moex/IMOEX")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(store.get("MOEX:IMOEX").await.unwrap().is_some());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn metadata_failure_does_not_migrate_legacy_history() {
    let server = MockServer::start().await;
    mount_symbol(&server, 503, "{}").await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 1024)
        .await
        .unwrap();
    store
        .put("MOEX:IMOEX", &Bytes::from_static(b"[]"), Duration::ZERO)
        .await
        .unwrap();
    let response = router(AppState::with_store(
        client,
        Duration::ZERO,
        1024,
        store.clone(),
    ))
    .oneshot(
        Request::builder()
            .uri("/v1/moex/IMOEX")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(store.get("MOEX:IMOEX").await.unwrap().is_some());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn unusable_board_date_metadata_returns_dependency_error_before_history_fetch() {
    let cases = [
        include_str!("fixtures/moex/imoex-security-malformed-date.json"),
        include_str!("fixtures/moex/imoex-security-inverted-date.json"),
        include_str!("fixtures/moex/imoex-security-overlapping-dates.json"),
    ];
    for metadata in cases {
        let server = MockServer::start().await;
        mount_symbol(&server, 200, metadata).await;
        let client =
            MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
        let response = router(AppState::new(client))
            .oneshot(
                Request::builder()
                    .uri("/v1/moex/IMOEX")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/securities/IMOEX.json");
    }
}

#[tokio::test]
async fn migration_failure_returns_store_error_and_keeps_legacy_row() {
    let server = MockServer::start().await;
    mount_symbol(&server, 200, METADATA).await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite3");
    let store = CacheStore::open(&path, 1024).await.unwrap();
    let cached = serde_json::to_vec(&vec![record("2026-10-01T00:00:00Z", 100.0)]).unwrap();
    store
        .put("MOEX:IMOEX", &Bytes::from(cached), Duration::ZERO)
        .await
        .unwrap();
    let admin = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false),
    )
    .await
    .unwrap();
    sqlx::query("CREATE TRIGGER fail_moex_import BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END")
        .execute(&admin).await.unwrap();
    let response = router(AppState::with_store(
        client,
        Duration::ZERO,
        1024,
        store.clone(),
    ))
    .oneshot(
        Request::builder()
            .uri("/v1/moex/IMOEX")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(store.get("MOEX:IMOEX").await.unwrap().is_some());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    admin.close().await;
}

#[tokio::test]
async fn underscore_tickers_require_single_alphanumeric_separators() {
    let server = MockServer::start().await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let app = router(AppState::new(client));
    for ticker in ["_GLDRUB", "GLDRUB_", "GLD__RUB"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/moex/{ticker}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn existing_sber_history_keeps_its_shares_market_mapping() {
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
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/history-page.json")),
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
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let response = router(AppState::new(client))
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
    let rows: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rows[0]["close"], 282.22);
    assert_eq!(rows[0]["facevalue"], 1.0);
    assert_eq!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.url.path()
                == "/history/engines/stock/markets/shares/securities/SBER.json")
            .count(),
        1
    );
}
