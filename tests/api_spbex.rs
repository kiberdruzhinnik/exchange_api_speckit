use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use exchange_api::{
    AppState, cache_store::CacheStore, http::router, moex::client::MoexClient,
    spbex::client::SpbexClient,
};
use serde_json::Value;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::time::Duration;
use tempfile::tempdir;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const PATH: &str = "/api/reader/marketdata/charts/chistory";
const HISTORY: &str = r#"[{"bar_unixtime":1364169600,"close":73.35,"high":75.05,"low":73.21},{"bar_unixtime":1364256000,"close":74.15,"high":74.55,"low":73.90}]"#;

fn app(server: &MockServer, ttl: Duration) -> axum::Router {
    let moex = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let spbex = SpbexClient::new(
        &format!("{}/api/", server.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    router(AppState::with_services(moex, spbex, ttl, 1024 * 1024))
}

fn chart(body: &'static str) -> Mock {
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
}

#[tokio::test]
async fn history_returns_six_fields_sorted_and_uses_history_cache() {
    let server = MockServer::start().await;
    chart(HISTORY).expect(1).mount(&server).await;
    let app = app(&server, Duration::from_secs(30));
    for symbol in ["sber", "SBER"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/spbex/{symbol}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let records: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(records.as_array().unwrap().len(), 2);
        assert_eq!(records[0]["date"], "2013-03-25T00:00:00Z");
        assert_eq!(records[0]["volume"], Value::Null);
        assert_eq!(records[0]["facevalue"], 1.0);
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn history_excludes_current_date_candle_from_response_and_cache_but_quote_returns_it() {
    let server = MockServer::start().await;
    let today = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let feed = format!(
        r#"[{{"bar_unixtime":1364169600,"close":73.35,"high":75.05,"low":73.21}},{{"bar_unixtime":1364256000,"close":74.15,"high":74.55,"low":73.90}},{{"bar_unixtime":{today},"close":99.9,"high":100.0,"low":99.0}}]"#
    );
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string(feed))
        .expect(2)
        .mount(&server)
        .await;
    let app = app(&server, Duration::from_secs(30));

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/spbex/SBER")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let records: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(records.as_array().unwrap().len(), 2);
        assert!(records.as_array().unwrap().iter().all(|record| {
            record["date"].as_str().is_some_and(|date| {
                !date.starts_with(&chrono::Utc::now().format("%Y-%m-%d").to_string())
            })
        }));
    }

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/spbex/SBER/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let quote: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        quote[0]["date"],
        chrono::Utc::now().format("%Y-%m-%dT00:00:00Z").to_string()
    );
    assert_eq!(quote[0]["close"], 99.9);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn history_returns_empty_array_for_successful_empty_feed() {
    let server = MockServer::start().await;
    chart("[]").mount(&server).await;
    let response = app(&server, Duration::from_secs(1))
        .oneshot(
            Request::builder()
                .uri("/v1/spbex/SBER")
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
async fn quote_reaches_upstream_on_every_request_and_returns_latest_candle_shape() {
    let server = MockServer::start().await;
    chart(HISTORY).expect(2).mount(&server).await;
    let app = app(&server, Duration::from_secs(30));
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/spbex/SBER/quote")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 1);
        assert_eq!(value[0]["close"], 74.15);
        assert_eq!(value[0]["volume"], Value::Null);
        assert_eq!(value[0]["facevalue"], 1.0);
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn quote_expands_lookback_and_empty_full_range_returns_one_null_record() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(|request: &wiremock::Request| {
            let from: i64 = request
                .url
                .query_pairs()
                .find(|(key, _)| key == "from")
                .unwrap()
                .1
                .parse()
                .unwrap();
            if from < 1_400_000_000 {
                ResponseTemplate::new(200).set_body_string(
                    r#"[{"bar_unixtime":1364169600,"close":73.35,"high":75.05,"low":73.21}]"#,
                )
            } else {
                ResponseTemplate::new(200).set_body_string("[]")
            }
        })
        .mount(&server)
        .await;
    let router_app = app(&server, Duration::from_secs(30));
    let response = router_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/spbex/SBER/quote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value[0]["close"], 73.35);
    let requests = server.received_requests().await.unwrap();
    assert!(requests.len() > 2);
    let froms: Vec<i64> = requests
        .iter()
        .map(|request| {
            request
                .url
                .query_pairs()
                .find(|(key, _)| key == "from")
                .unwrap()
                .1
                .parse()
                .unwrap()
        })
        .collect();
    assert!(froms.windows(2).all(|window| window[1] < window[0]));

    let empty_server = MockServer::start().await;
    chart("[]").mount(&empty_server).await;
    let response = app(&empty_server, Duration::from_secs(30))
        .oneshot(
            Request::builder()
                .uri("/v1/spbex/SBER/quote")
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
    assert_eq!(
        value,
        serde_json::json!([{"date":null,"close":null,"high":null,"low":null,"volume":null,"facevalue":null}])
    );
}

#[tokio::test]
async fn persistent_history_is_reused_after_cache_state_restarts() {
    let server = MockServer::start().await;
    chart(HISTORY).expect(2).mount(&server).await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("history.sqlite3");
    {
        let moex = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
        let spbex = SpbexClient::new(
            &format!("{}/api/", server.uri()),
            Duration::from_secs(1),
            4096,
        )
        .unwrap();
        let store = CacheStore::open(&db, 1024 * 1024).await.unwrap();
        let app = router(AppState::with_services_and_store(
            moex,
            spbex,
            Duration::from_secs(30),
            1024 * 1024,
            store,
        ));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/spbex/SBER")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let moex = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let spbex = SpbexClient::new(
        &format!("{}/api/", server.uri()),
        Duration::from_secs(1),
        4096,
    )
    .unwrap();
    let store = CacheStore::open(&db, 1024 * 1024).await.unwrap();
    let app = router(AppState::with_services_and_store(
        moex,
        spbex,
        Duration::from_secs(30),
        1024 * 1024,
        store,
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/spbex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn history_store_read_and_write_failures_return_503() {
    for fail_write in [false, true] {
        let server = MockServer::start().await;
        chart(HISTORY).mount(&server).await;
        let dir = tempdir().unwrap();
        let db = dir.path().join("history.sqlite3");
        let store = CacheStore::open(&db, 1024 * 1024).await.unwrap();
        let admin = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(&db)
                .create_if_missing(false),
        )
        .await
        .unwrap();
        if fail_write {
            sqlx::query("CREATE TRIGGER reject_history_insert BEFORE INSERT ON history_collections BEGIN SELECT RAISE(ABORT, 'injected write failure'); END")
                .execute(&admin).await.unwrap();
        } else {
            sqlx::query("DROP TABLE history_collections")
                .execute(&admin)
                .await
                .unwrap();
        }
        let moex = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
        let spbex = SpbexClient::new(
            &format!("{}/api/", server.uri()),
            Duration::from_secs(1),
            4096,
        )
        .unwrap();
        let app = router(AppState::with_services_and_store(
            moex,
            spbex,
            Duration::from_secs(30),
            1024 * 1024,
            store,
        ));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/spbex/SBER")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["error"]["code"], "history_store_unavailable");
        admin.close().await;
    }
}

#[tokio::test]
async fn malformed_symbol_and_source_failures_have_distinct_statuses() {
    let server = MockServer::start().await;
    let app = app(&server, Duration::from_secs(1));
    let malformed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/spbex/bad%20symbol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    chart("broken").mount(&server).await;
    let failed = app
        .oneshot(
            Request::builder()
                .uri("/v1/spbex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(failed.status(), StatusCode::BAD_GATEWAY);
}
