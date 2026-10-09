use exchange_api::moex::client::MoexClient;
use std::time::Duration;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

#[tokio::test]
async fn fetches_every_history_page() {
    let server = MockServer::start().await;
    let first = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR","2026-10-06",1,1,1,1],["TQBR","2026-10-05",1,1,1,1]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,3,2]]}}"#;
    let second = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR","2026-10-04",1,1,1,1]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[2,3,2]]}}"#;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(first))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .and(query_param("start", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_string(second))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(2)).unwrap();
    let history = client.history("SBER").await.unwrap();
    assert_eq!(history["data"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn rejects_upstream_json_larger_than_response_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"description":[]}"#))
        .mount(&server)
        .await;
    let client = MoexClient::with_limits(
        &format!("{}/", server.uri()),
        Duration::from_secs(2),
        8,
        1024,
    )
    .unwrap();

    let error = client.security("SBER").await.unwrap_err();
    assert!(error.to_string().contains("byte limit"));
}

#[tokio::test]
async fn rejects_history_that_exceeds_aggregate_limit() {
    let server = MockServer::start().await;
    let first = r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR","2026-10-06",1,1,1,1]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,2,1]]}}"#;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(first))
        .mount(&server)
        .await;
    let client = MoexClient::with_limits(
        &format!("{}/", server.uri()),
        Duration::from_secs(2),
        4096,
        80,
    )
    .unwrap();

    let error = client.history("SBER").await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("history exceeds configured byte limit")
    );
}

#[tokio::test]
async fn requests_history_after_latest_record_date() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/history/engines/stock/markets/shares/securities/SBER.json"))
        .and(query_param("from", "2026-10-07"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"history":{"columns":["BOARDID","TRADEDATE","CLOSE","HIGH","LOW","VOLUME"],"data":[["TQBR","2026-10-07",1,1,1,1]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,1,100]]}}"#))
        .mount(&server).await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    let rows = client
        .history_since(
            "SBER",
            Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 6).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(rows["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn rejects_failed_or_incomplete_history_pages() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/history/engines/stock/markets/shares/securities/SBER.json")).and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"history":{"columns":["TRADEDATE"],"data":[["2024-01-01"]]},"history.cursor":{"columns":["INDEX","TOTAL","PAGESIZE"],"data":[[0,2,1]]}}"#)).mount(&server).await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .and(query_param("start", "1"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;
    let client = MoexClient::new(&format!("{}/", server.uri()), Duration::from_secs(1)).unwrap();
    assert!(client.history("SBER").await.is_err());
}
