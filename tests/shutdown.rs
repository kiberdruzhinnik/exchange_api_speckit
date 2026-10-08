use std::{
    future::{IntoFuture, pending},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
#[test]
fn sigint_stops_the_service_process() {
    let database = tempfile::tempdir().unwrap();
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);

    let child = Command::new(env!("CARGO_BIN_EXE_exchange-api"))
        .env("LISTEN_ADDR", address.to_string())
        .env(
            "MOEX_HISTORY_CACHE_DB_PATH",
            database.path().join("history.sqlite3"),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);

    wait_for_http(&address);
    let started = Instant::now();
    let signal = Command::new("kill")
        .args(["-INT", &child.0.id().to_string()])
        .status()
        .unwrap();
    assert!(signal.success(), "failed to send SIGINT to service process");

    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "service exited unsuccessfully: {status}");
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(30));
        thread::sleep(Duration::from_millis(20));
    }

    assert!(TcpStream::connect(address).is_err());
}

#[tokio::test]
async fn shutdown_waits_for_in_flight_server_work_to_finish() {
    use axum::{Router, routing::get};
    use tokio::sync::{Notify, oneshot};

    let started = Arc::new(Notify::new());
    let route_started = Arc::clone(&started);
    let app = Router::new().route(
        "/slow",
        get(move || {
            let started = Arc::clone(&route_started);
            async move {
                started.notify_one();
                tokio::time::sleep(Duration::from_millis(100)).await;
                "finished"
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (server_shutdown_tx, server_shutdown_rx) = oneshot::channel();
    let (signal_tx, signal_rx) = oneshot::channel();
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = server_shutdown_rx.await;
        })
        .into_future();
    let signal = async move {
        signal_rx.await.map_err(std::io::Error::other)?;
        let _ = server_shutdown_tx.send(());
        Ok::<(), std::io::Error>(())
    };
    let server_task = tokio::spawn(exchange_api::shutdown::run_until_shutdown(
        server,
        signal,
        Duration::from_secs(1),
    ));

    let started_request = started.notified();
    tokio::pin!(started_request);
    let client_request = tokio::spawn(async move {
        reqwest::get(format!("http://{address}/slow"))
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });
    started_request.await;
    signal_tx.send(()).unwrap();

    assert_eq!(client_request.await.unwrap(), "finished");
    assert!(server_task.await.unwrap().is_ok());
}

#[test]
fn shutdown_cancels_work_when_the_grace_deadline_expires() {
    use axum::{Router, routing::get};
    use tokio::sync::{Notify, oneshot};

    let cancelled = Arc::new(AtomicBool::new(false));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .unwrap();

    runtime.block_on(async {
        let started = Arc::new(Notify::new());
        let route_cancelled = Arc::clone(&cancelled);
        let route_started = Arc::clone(&started);
        let app = Router::new().route(
            "/hang",
            get(move || {
                let cancelled = Arc::clone(&route_cancelled);
                let started = Arc::clone(&route_started);
                async move {
                    let _drop_flag = DropFlag(cancelled);
                    started.notify_one();
                    pending::<&'static str>().await
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (server_shutdown_tx, server_shutdown_rx) = oneshot::channel();
        let (signal_tx, signal_rx) = oneshot::channel();
        let server = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = server_shutdown_rx.await;
            })
            .into_future();
        let signal = async move {
            signal_rx.await.map_err(std::io::Error::other)?;
            let _ = server_shutdown_tx.send(());
            Ok::<(), std::io::Error>(())
        };
        let server_task = tokio::spawn(exchange_api::shutdown::run_until_shutdown(
            server,
            signal,
            Duration::from_millis(20),
        ));

        let started_request = started.notified();
        tokio::pin!(started_request);
        let _client_request =
            tokio::spawn(async move { reqwest::get(format!("http://{address}/hang")).await });
        started_request.await;
        signal_tx.send(()).unwrap();

        assert!(server_task.await.unwrap().is_ok());
        // The binary returns from main after the deadline, which drops its Tokio
        // runtime and cancels Axum's detached connection task before process exit.
        assert!(!cancelled.load(Ordering::SeqCst));
    });

    drop(runtime);
    assert!(cancelled.load(Ordering::SeqCst));
}

fn wait_for_http(address: &SocketAddr) {
    let started = Instant::now();
    loop {
        if let Ok(mut stream) = TcpStream::connect(address) {
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            stream
                .write_all(
                    b"GET /health/live HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(
                response.starts_with("HTTP/1.1 200"),
                "unexpected response: {response}"
            );
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "service did not start"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
