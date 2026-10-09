use exchange_api::{
    Providers,
    cache_store::CacheStore,
    domain::{DailyMarketRecord, LatestQuoteRecord},
    history_refresh,
    provider::{ExchangeProvider, ProviderError},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone)]
struct Stub {
    calls: Arc<AtomicUsize>,
    result: Result<Vec<DailyMarketRecord>, ProviderError>,
}

#[async_trait::async_trait]
impl ExchangeProvider for Stub {
    fn normalize_symbol(&self, value: &str) -> Option<String> {
        Some(value.to_owned())
    }
    fn upstream_code(&self) -> &'static str {
        "stub_unavailable"
    }
    async fn history(&self, _: &str) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }
    async fn quote(&self, _: &str) -> Result<LatestQuoteRecord, ProviderError> {
        Ok(LatestQuoteRecord::no_trade())
    }
}

fn record(close: f64) -> DailyMarketRecord {
    DailyMarketRecord {
        date: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        close: Some(close),
        high: None,
        low: None,
        volume: None,
        facevalue: None,
    }
}

fn providers(stub: Stub) -> Providers {
    let stub: Arc<dyn ExchangeProvider> = Arc::new(stub);
    Providers {
        moex: stub.clone(),
        spbex: stub.clone(),
        cbr: stub,
    }
}

#[tokio::test]
async fn worker_refreshes_due_collection_and_resets_retry_state() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("cache.sqlite"), 1024)
        .await
        .unwrap();
    store
        .merge_records("moex", "SBER", &[record(1.0)], true, 0)
        .await
        .unwrap();
    store
        .record_failure("moex", "SBER", true, 10, 900, 1)
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let stub = Stub {
        calls: calls.clone(),
        result: Ok(vec![record(2.0)]),
    };
    let (tx, rx) = tokio::sync::watch::channel(false);
    let task = tokio::spawn(history_refresh::run(
        store.clone(),
        providers(stub),
        Duration::from_secs(1209600),
        Duration::from_secs(3),
        rx,
    ));
    tokio::time::timeout(Duration::from_secs(2), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    tx.send(true).unwrap();
    task.await.unwrap();
    let collection = store.collection("moex", "SBER").await.unwrap().unwrap();
    assert_eq!(collection.records[0].close, Some(2.0));
    assert_eq!(collection.consecutive_failures, 0);
    assert!(collection.last_full_refresh_at.unwrap() > 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn retry_state_uses_capped_exponential_delay_and_persists() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.sqlite");
    let store = CacheStore::open(&path, 1024).await.unwrap();
    store
        .merge_records("moex", "SBER", &[record(1.0)], true, 10)
        .await
        .unwrap();
    store
        .record_failure("moex", "SBER", true, 604800, 3, 100)
        .await
        .unwrap();
    assert_eq!(
        store
            .collection("moex", "SBER")
            .await
            .unwrap()
            .unwrap()
            .next_attempt_at,
        Some(101)
    );
    store
        .record_failure("moex", "SBER", true, 604800, 3, 200)
        .await
        .unwrap();
    assert_eq!(
        store
            .collection("moex", "SBER")
            .await
            .unwrap()
            .unwrap()
            .next_attempt_at,
        Some(202)
    );
    store
        .record_failure("moex", "SBER", true, 604800, 3, 300)
        .await
        .unwrap();
    assert_eq!(
        store
            .collection("moex", "SBER")
            .await
            .unwrap()
            .unwrap()
            .next_attempt_at,
        Some(303)
    );
    let reopened = CacheStore::open(&path, 1024).await.unwrap();
    assert_eq!(
        reopened
            .collection("moex", "SBER")
            .await
            .unwrap()
            .unwrap()
            .consecutive_failures,
        3
    );
}

#[tokio::test]
async fn permanent_status_defers_until_one_full_interval_after_failure() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("cache.sqlite"), 1024)
        .await
        .unwrap();
    store
        .merge_records("moex", "SBER", &[record(1.0)], true, 10)
        .await
        .unwrap();
    store
        .record_failure("moex", "SBER", false, 1209600, 900, 500)
        .await
        .unwrap();
    let state = store.collection("moex", "SBER").await.unwrap().unwrap();
    assert_eq!(state.next_attempt_at, Some(1_210_100));
    assert_eq!(state.last_full_refresh_at, Some(10));
}

#[tokio::test]
async fn failed_refresh_keeps_last_good_data_and_success_timestamp() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("cache.sqlite"), 1024)
        .await
        .unwrap();
    store
        .merge_records("moex", "SBER", &[record(1.0)], true, 1)
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let stub = Stub {
        calls: calls.clone(),
        result: Err(ProviderError::InvalidData(
            "incomplete full response".into(),
        )),
    };
    let (tx, rx) = tokio::sync::watch::channel(false);
    let task = tokio::spawn(history_refresh::run(
        store.clone(),
        providers(stub),
        Duration::from_secs(1),
        Duration::from_secs(3),
        rx,
    ));
    tokio::time::timeout(Duration::from_secs(2), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    tx.send(true).unwrap();
    task.await.unwrap();
    let state = store.collection("moex", "SBER").await.unwrap().unwrap();
    assert_eq!(state.records, vec![record(1.0)]);
    assert_eq!(state.last_full_refresh_at, Some(1));
    assert_eq!(state.consecutive_failures, 1);
    assert!(state.next_attempt_at.unwrap() > chrono::Utc::now().timestamp());
}

#[test]
fn classifies_transient_statuses_as_retryable() {
    for status in [408, 425, 429, 500, 502, 599] {
        assert!(
            ProviderError::HttpStatus {
                status,
                message: String::new()
            }
            .retryable()
        );
    }
    assert!(
        !ProviderError::HttpStatus {
            status: 404,
            message: String::new()
        }
        .retryable()
    );
    assert!(ProviderError::Transport("timeout".into()).retryable());
    assert!(ProviderError::InvalidData("invalid page".into()).retryable());
}
