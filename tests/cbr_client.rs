use exchange_api::cbr::client::{CbrClient, CbrError};
use std::time::Duration;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const DIRECTORY: &str = "/scripts/XML_valFull.asp";
const DAILY: &str = "/scripts/XML_daily.asp";
const HISTORY: &str = "/scripts/XML_dynamic.asp";

fn client(server: &MockServer, limit: usize) -> CbrClient {
    CbrClient::new(&format!("{}/", server.uri()), Duration::from_secs(1), limit).unwrap()
}

#[tokio::test]
async fn rejects_unsupported_symbol_using_current_directory() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DIRECTORY))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/currencies.xml")),
        )
        .mount(&server)
        .await;
    daily_directory_mock().mount(&server).await;
    assert!(matches!(
        client(&server, 4096).history("XXX").await,
        Err(CbrError::InvalidSymbol)
    ));
}

#[tokio::test]
async fn requests_history_after_latest_record_date() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DIRECTORY))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/currencies.xml")),
        )
        .mount(&server)
        .await;
    daily_directory_mock().mount(&server).await;
    Mock::given(method("GET"))
        .and(path(HISTORY))
        .and(wiremock::matchers::query_param("date_req1", "03/01/2024"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/history.xml")),
        )
        .mount(&server)
        .await;
    let after = chrono::DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    client(&server, 4096)
        .history_since("USD", Some(after))
        .await
        .unwrap();
    let request = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.url.path() == HISTORY)
        .unwrap();
    assert!(
        request
            .url
            .query_pairs()
            .any(|(key, value)| key == "date_req1" && value == "03/01/2024")
    );
}

#[tokio::test]
async fn rejects_upstream_status_and_malformed_xml() {
    let status_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DIRECTORY))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/currencies.xml")),
        )
        .mount(&status_server)
        .await;
    daily_directory_mock().mount(&status_server).await;
    Mock::given(method("GET"))
        .and(path(HISTORY))
        .respond_with(ResponseTemplate::new(503))
        .mount(&status_server)
        .await;
    assert!(matches!(
        client(&status_server, 4096).history("USD").await,
        Err(CbrError::HttpStatus(503))
    ));

    let malformed_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DIRECTORY))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/currencies.xml")),
        )
        .mount(&malformed_server)
        .await;
    daily_directory_mock().mount(&malformed_server).await;
    Mock::given(method("GET"))
        .and(path(HISTORY))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/malformed.xml")),
        )
        .mount(&malformed_server)
        .await;
    assert!(matches!(
        client(&malformed_server, 4096).history("USD").await,
        Err(CbrError::InvalidData(_))
    ));
}

#[tokio::test]
async fn enforces_response_size_limit_for_history_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DIRECTORY))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/currencies.xml")),
        )
        .mount(&server)
        .await;
    daily_directory_mock().mount(&server).await;
    Mock::given(method("GET"))
        .and(path(HISTORY))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/cbr/history.xml")),
        )
        .mount(&server)
        .await;
    assert!(
        matches!(client(&server, 300).history("USD").await, Err(CbrError::Upstream(message)) if message.contains("byte limit"))
    );
}

#[tokio::test]
async fn enforces_streamed_response_limit_without_content_length() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let count = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]);
            let first_line = request.lines().next().unwrap_or_default();
            if first_line.starts_with("GET /scripts/XML_valFull.asp ") {
                write_fixed_xml(
                    &mut stream,
                    include_str!("fixtures/cbr/currencies.xml").as_bytes(),
                );
            } else if first_line.starts_with("GET /scripts/XML_daily.asp?d=0 ") {
                write_fixed_xml(
                    &mut stream,
                    include_str!("fixtures/cbr/current-currencies.xml").as_bytes(),
                );
            } else {
                let mut body = include_str!("fixtures/cbr/history.xml").as_bytes().to_vec();
                body.extend(std::iter::repeat_n(b' ', 1024));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                for chunk in body.chunks(48) {
                    write!(stream, "{:X}\r\n", chunk.len()).unwrap();
                    stream.write_all(chunk).unwrap();
                    stream.write_all(b"\r\n").unwrap();
                }
                stream.write_all(b"0\r\n\r\n").unwrap();
            }
        }
    });

    let result = CbrClient::new(&format!("http://{address}/"), Duration::from_secs(2), 300)
        .unwrap()
        .history("USD")
        .await;
    assert!(matches!(result, Err(CbrError::Upstream(message)) if message.contains("byte limit")));
    server.join().unwrap();
}

fn write_fixed_xml(stream: &mut std::net::TcpStream, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(body).unwrap();
}

fn daily_directory_mock() -> Mock {
    Mock::given(method("GET"))
        .and(path(DAILY))
        .and(wiremock::matchers::query_param("d", "0"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/cbr/current-currencies.xml")),
        )
}
