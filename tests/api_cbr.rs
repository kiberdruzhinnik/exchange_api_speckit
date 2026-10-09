use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use exchange_api::{
    AppState, cache_store::CacheStore, cbr::client::CbrClient, http::router,
    moex::client::MoexClient, spbex::client::SpbexClient,
};
use serde_json::Value;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{path::Path, time::Duration};
use tempfile::tempdir;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

const DIRECTORY_PATH: &str = "/scripts/XML_valFull.asp";
const HISTORY_PATH: &str = "/scripts/XML_dynamic.asp";
const DAILY_PATH: &str = "/scripts/XML_daily.asp";

fn source(server: &MockServer, store: Option<CacheStore>, ttl: Duration) -> axum::Router {
    let moex = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let spbex = SpbexClient::new(
        &format!("{}/api/", server.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    let cbr = CbrClient::new(
        &format!("{}/", server.uri()),
        Duration::from_secs(1),
        1024 * 1024,
    )
    .unwrap();
    let state = match store {
        Some(store) => {
            AppState::with_all_services_and_store(moex, spbex, cbr, ttl, 1024 * 1024, store)
        }
        None => AppState::with_all_services(moex, spbex, cbr, ttl, 1024 * 1024),
    };
    router(state)
}

fn directory_mock() -> Mock {
    Mock::given(method("GET"))
        .and(path(DIRECTORY_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/currencies.xml")),
        )
}

fn currency_list_mock() -> Mock {
    Mock::given(method("GET"))
        .and(path(DAILY_PATH))
        .and(query_param("d", "0"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/cbr/current-currencies.xml")),
        )
}

fn history_mock() -> Mock {
    Mock::given(method("GET"))
        .and(path(HISTORY_PATH))
        .and(query_param("VAL_NM_RQ", "R01235"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/history.xml")),
        )
}

fn latest_mock(body: &'static str) -> Mock {
    Mock::given(method("GET"))
        .and(path(DAILY_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
}

#[tokio::test]
async fn history_returns_normalized_sorted_six_field_records_and_reuses_cache() {
    let server = MockServer::start().await;
    directory_mock().expect(1).mount(&server).await;
    currency_list_mock().mount(&server).await;
    history_mock().expect(1).mount(&server).await;
    let app = source(&server, None, Duration::from_secs(30));

    for (index, symbol) in ["usd", "USD"].into_iter().enumerate() {
        let route = if index == 0 {
            format!("/v2/history/cbr/{symbol}")
        } else {
            format!("/v1/cbr/{symbol}")
        };
        let response = app
            .clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(value.as_array().unwrap().len(), 3);
        assert_eq!(value[0]["date"], "2013-03-25T00:00:00Z");
        assert_eq!(value[0]["close"], 0.05);
        assert_eq!(value[0]["facevalue"], 100.0);
        for field in ["high", "low", "volume"] {
            assert_eq!(value[0][field], Value::Null);
        }
        assert_eq!(value[2]["close"], Value::Null);
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn unsupported_currency_returns_standard_400_error() {
    let server = MockServer::start().await;
    directory_mock().mount(&server).await;
    currency_list_mock().mount(&server).await;
    let response = source(&server, None, Duration::from_secs(1))
        .oneshot(
            Request::builder()
                .uri("/v1/cbr/XYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let value: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["error"]["code"], "invalid_symbol");
}

#[tokio::test]
async fn persists_history_across_store_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.sqlite3");
    let server = MockServer::start().await;
    directory_mock().expect(2).mount(&server).await;
    currency_list_mock().mount(&server).await;
    history_mock().expect(2).mount(&server).await;
    let store = CacheStore::open(&path, 1024 * 1024).await.unwrap();
    let first = source(&server, Some(store), Duration::from_secs(30));
    let first_response = first
        .oneshot(
            Request::builder()
                .uri("/v1/cbr/USD")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_response.status(), StatusCode::OK);

    let reopened = CacheStore::open(&path, 1024 * 1024).await.unwrap();
    let second = source(&server, Some(reopened), Duration::from_secs(30));
    let second_response = second
        .oneshot(
            Request::builder()
                .uri("/v1/cbr/USD")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_response.status(), StatusCode::OK);
    assert_eq!(server.received_requests().await.unwrap().len(), 6);
}

async fn store_with_insert_failure(path: &Path) -> CacheStore {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await.unwrap();
    sqlx::query("CREATE TABLE history_cache (symbol TEXT PRIMARY KEY NOT NULL, response_body BLOB NOT NULL, fetched_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, response_bytes INTEGER NOT NULL)").execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE history_collections (provider TEXT NOT NULL, symbol TEXT NOT NULL, latest_record_date TEXT, last_full_refresh_at INTEGER, consecutive_failures INTEGER NOT NULL DEFAULT 0, next_attempt_at INTEGER, updated_at INTEGER NOT NULL, PRIMARY KEY(provider, symbol))").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_history_insert BEFORE INSERT ON history_collections BEGIN SELECT RAISE(ABORT, 'test store failure'); END").execute(&pool).await.unwrap();
    pool.close().await;
    CacheStore::open(path, 1024 * 1024).await.unwrap()
}

#[tokio::test]
async fn history_store_failure_returns_503() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.sqlite3");
    let store = store_with_insert_failure(&path).await;
    let server = MockServer::start().await;
    directory_mock().mount(&server).await;
    currency_list_mock().mount(&server).await;
    history_mock().mount(&server).await;
    let response = source(&server, Some(store), Duration::from_secs(30))
        .oneshot(
            Request::builder()
                .uri("/v1/cbr/USD")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn quote_fetches_latest_rate_on_every_request_and_returns_shared_shape() {
    let server = MockServer::start().await;
    directory_mock().expect(1).mount(&server).await;
    latest_mock(include_str!("fixtures/cbr/latest.xml"))
        .expect(3)
        .mount(&server)
        .await;
    let app = source(&server, None, Duration::from_secs(30));
    for route in ["/v2/quote/cbr/USD", "/v1/cbr/USD/quote"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(value.as_array().unwrap().len(), 1);
        assert_eq!(value[0]["date"], "2026-10-09T00:00:00Z");
        assert_eq!(value[0]["close"], 92.5);
        assert_eq!(value[0]["facevalue"], 1.0);
        for field in ["high", "low", "volume"] {
            assert_eq!(value[0][field], Value::Null);
        }
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

#[tokio::test]
async fn quote_without_source_rate_returns_one_all_null_record() {
    let server = MockServer::start().await;
    directory_mock().mount(&server).await;
    currency_list_mock().mount(&server).await;
    latest_mock(include_str!("fixtures/cbr/quote-empty.xml"))
        .mount(&server)
        .await;
    let response = source(&server, None, Duration::from_secs(1))
        .oneshot(
            Request::builder()
                .uri("/v1/cbr/USD/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value.as_array().unwrap().len(), 1);
    for field in ["date", "close", "high", "low", "volume", "facevalue"] {
        assert_eq!(value[0][field], Value::Null);
    }
}

#[tokio::test]
async fn successful_empty_history_returns_empty_array() {
    let server = MockServer::start().await;
    directory_mock().mount(&server).await;
    currency_list_mock().mount(&server).await;
    Mock::given(method("GET"))
        .and(path(HISTORY_PATH))
        .and(query_param("VAL_NM_RQ", "R01235"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/empty.xml")),
        )
        .mount(&server)
        .await;
    let response = source(&server, None, Duration::from_secs(1))
        .oneshot(
            Request::builder()
                .uri("/v1/cbr/USD")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap(),
        "[]"
    );
}

#[tokio::test]
async fn malformed_upstream_history_returns_standard_502_error() {
    let server = MockServer::start().await;
    directory_mock().mount(&server).await;
    currency_list_mock().mount(&server).await;
    Mock::given(method("GET"))
        .and(path(HISTORY_PATH))
        .and(query_param("VAL_NM_RQ", "R01235"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/malformed.xml")),
        )
        .mount(&server)
        .await;
    let response = source(&server, None, Duration::from_secs(1))
        .oneshot(
            Request::builder()
                .uri("/v1/cbr/USD")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let value: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["error"]["code"], "cbr_unavailable");
}
