use reqwest::{Client, StatusCode};
use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

struct Service {
    child: Child,
    _data: tempfile::TempDir,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn start_service_without_refresh_settings(
    moex_base_url: &str,
    spbex_base_url: &str,
    cbr_base_url: &str,
) -> (Service, String) {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);

    let data = tempfile::tempdir().unwrap();
    let database = data.path().join("history.sqlite3");
    let child = Command::new(env!("CARGO_BIN_EXE_exchange-api"))
        .env_clear()
        .env("EXCHANGE_API_LISTEN_ADDR", address.to_string())
        .env("EXCHANGE_API_HISTORY_CACHE_DB_PATH", &database)
        .env("EXCHANGE_API_MOEX_ISS_BASE_URL", moex_base_url)
        .env("EXCHANGE_API_SPBEX_API_BASE_URL", spbex_base_url)
        .env("EXCHANGE_API_CBR_API_BASE_URL", cbr_base_url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let service = Service { child, _data: data };
    let base_url = format!("http://{address}");
    let client = Client::new();
    for _ in 0..100 {
        if client
            .get(format!("{base_url}/health/live"))
            .send()
            .await
            .is_ok_and(|response| response.status() == StatusCode::OK)
        {
            return (service, base_url);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("service did not become live with refresh settings unset");
}

#[tokio::test]
async fn v2_routes_are_registered_when_refresh_settings_are_unset() {
    let moex = MockServer::start().await;
    let spbex = MockServer::start().await;
    let cbr = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/history/engines/stock/markets/shares/securities/SBER.json",
        ))
        .and(query_param("start", "0"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/history-page.json")),
        )
        .mount(&moex)
        .await;
    Mock::given(method("GET"))
        .and(path("/securities/SBER.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-description.json")),
        )
        .mount(&moex)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/stock/markets/shares/boards/TQBR/securities/SBER.json",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/security-tqbr.json")),
        )
        .mount(&moex)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/engines/stock/markets/shares/securities/SBER/trades.json",
        ))
        .and(query_param("limit", "1"))
        .and(query_param("reversed", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/moex/trades-latest.json")),
        )
        .mount(&moex)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/reader/marketdata/charts/chistory"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/spbex/history.json")),
        )
        .mount(&spbex)
        .await;

    Mock::given(method("GET"))
        .and(path("/scripts/XML_valFull.asp"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/currencies.xml")),
        )
        .mount(&cbr)
        .await;
    Mock::given(method("GET"))
        .and(path("/scripts/XML_dynamic.asp"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/history.xml")),
        )
        .mount(&cbr)
        .await;
    Mock::given(method("GET"))
        .and(path("/scripts/XML_daily.asp"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/latest.xml")),
        )
        .mount(&cbr)
        .await;

    let (_service, base_url) = start_service_without_refresh_settings(
        &format!("{}/", moex.uri()),
        &format!("{}/api/", spbex.uri()),
        &format!("{}/", cbr.uri()),
    )
    .await;
    let client = Client::new();
    for path in [
        "/v2/history/moex/SBER",
        "/v2/quote/moex/SBER",
        "/v2/history/spbex/SBER",
        "/v2/quote/spbex/SBER",
        "/v2/history/cbr/USD",
        "/v2/quote/cbr/USD",
    ] {
        let response = client
            .get(format!("{base_url}{path}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
}
