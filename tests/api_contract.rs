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
                .uri("/v2/history/moex/SBER")
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
                .uri("/v2/history/moex/SBER")
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
async fn rejects_unknown_v2_provider_with_actionable_error_without_fetching() {
    let (server, app) = app_with_history(include_str!("fixtures/moex/history-page.json")).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v2/history/unknown/SBER")
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
async fn rejects_malformed_symbol_on_v2_history_route() {
    let (server, app) = app_with_history(include_str!("fixtures/moex/history-page.json")).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v2/history/moex/bad%20ticker")
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
    assert_eq!(value["error"]["code"], "invalid_symbol");
    assert!(server.received_requests().await.unwrap().is_empty());
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

#[tokio::test]
async fn durable_history_survives_restart_and_is_incrementally_refreshed() {
    use exchange_api::cache_store::CacheStore;

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
            "/engines/stock/markets/shares/boards/TQBR/securities/SBER.json",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-tqbr.json")),
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

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("history.sqlite3");
    let ttl = Duration::from_millis(150);
    let client = || MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let store = CacheStore::open(&db, 1024 * 1024).await.unwrap();
    let first_app = router(AppState::with_store(client(), ttl, 1024 * 1024, store));
    let first = first_app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);

    let reopened = CacheStore::open(&db, 1024 * 1024).await.unwrap();
    let restarted_app = router(AppState::with_store(client(), ttl, 1024 * 1024, reopened));
    let after_restart = restarted_app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_restart.status(), StatusCode::OK);
    assert_eq!(server.received_requests().await.unwrap().len(), 6);

    tokio::time::sleep(Duration::from_millis(175)).await;
    let expired_store = CacheStore::open(&db, 1024 * 1024).await.unwrap();
    let expired_app = router(AppState::with_store(
        client(),
        ttl,
        1024 * 1024,
        expired_store,
    ));
    let refreshed = expired_app
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.status(), StatusCode::OK);
    assert_eq!(server.received_requests().await.unwrap().len(), 9);
}

#[tokio::test]
async fn history_request_merges_only_newer_dates_and_returns_full_collection() {
    use exchange_api::cache_store::CacheStore;
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
            "/engines/stock/markets/shares/boards/TQBR/securities/SBER.json",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-tqbr.json")),
        )
        .mount(&server)
        .await;
    let history_path = "/history/engines/stock/markets/shares/securities/SBER.json";
    let first = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR","2026-10-06",1,1,1,1]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#;
    let incremental = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR","2026-10-07",2,2,2,2]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#;
    let duplicate = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR","2026-10-08",3,3,3,3],["TQBR","2026-10-08",3,3,3,3]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,2,100]]}}"#;
    Mock::given(method("GET"))
        .and(path(history_path))
        .and(wiremock::matchers::query_param_is_missing("from"))
        .respond_with(ResponseTemplate::new(200).set_body_string(first))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(history_path))
        .and(query_param("from", "2026-10-08"))
        .respond_with(ResponseTemplate::new(200).set_body_string(duplicate))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(history_path))
        .and(query_param("from", "2026-10-07"))
        .respond_with(ResponseTemplate::new(200).set_body_string(incremental))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 1024)
        .await
        .unwrap();
    let inspect_store = store.clone();
    let app = router(AppState::with_store(
        client_at(&server),
        Duration::from_secs(1),
        1024 * 1024,
        store,
    ));
    for expected_len in [1, 2] {
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
        let rows: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), expected_len);
        if expected_len == 2 {
            assert_eq!(rows[0]["date"], "2026-10-06T00:00:00Z");
            assert_eq!(rows[1]["date"], "2026-10-07T00:00:00Z");
        }
    }
    let rejected = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/moex/SBER")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::BAD_GATEWAY);
    let retained = inspect_store
        .collection("moex", "SBER")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retained.records.len(), 2);
    assert_eq!(retained.last_full_refresh_at, None);
    let history_requests = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|request| request.url.path() == history_path)
        .collect::<Vec<_>>();
    assert_eq!(history_requests.len(), 3);
    assert!(
        history_requests[1]
            .url
            .query_pairs()
            .any(|(key, value)| key == "from" && value == "2026-10-07")
    );
}

fn client_at(server: &MockServer) -> MoexClient {
    MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap()
}
