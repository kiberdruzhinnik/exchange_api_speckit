use bytes::Bytes;
use exchange_api::cache_store::CacheStore;
use std::time::Duration;

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

#[tokio::test]
async fn reopened_store_returns_unexpired_complete_response() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite3");
    let response = Bytes::from_static(b"[{\"date\":\"2026-10-06T00:00:00Z\"}]");
    {
        let store = CacheStore::open(&path, 1024).await.unwrap();
        store
            .put("SBER", &response, Duration::from_secs(60))
            .await
            .unwrap();
    }
    let reopened = CacheStore::open(path, 1024).await.unwrap();
    assert_eq!(reopened.get("SBER").await.unwrap().unwrap().body, response);
}

#[tokio::test]
async fn replaces_whole_body_without_pruning_durable_entries_to_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 6)
        .await
        .unwrap();
    store
        .put("AAA", &Bytes::from_static(b"old!"), Duration::from_secs(60))
        .await
        .unwrap();
    store
        .put("AAA", &Bytes::from_static(b"new!"), Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(
        store.get("AAA").await.unwrap().unwrap().body,
        Bytes::from_static(b"new!")
    );
    store
        .put(
            "BBB",
            &Bytes::from_static(b"other"),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    assert_eq!(
        store.get("AAA").await.unwrap().unwrap().body,
        Bytes::from_static(b"new!")
    );
    assert_eq!(
        store.get("BBB").await.unwrap().unwrap().body,
        Bytes::from_static(b"other")
    );
}

#[tokio::test]
async fn returns_entries_even_when_legacy_ttl_is_zero() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 1024)
        .await
        .unwrap();
    store
        .put("SBER", &Bytes::from_static(b"[]"), Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(
        store.get("SBER").await.unwrap().unwrap().body,
        Bytes::from_static(b"[]")
    );
}

#[tokio::test]
async fn migrates_legacy_keys_and_consolidates_spbex_date_suffixes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let pool = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true),
    )
    .await
    .unwrap();
    sqlx::query("CREATE TABLE history_cache (symbol TEXT PRIMARY KEY NOT NULL, response_body BLOB NOT NULL, fetched_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, response_bytes INTEGER NOT NULL)").execute(&pool).await.unwrap();
    let old = serde_json::to_vec(&vec![record("2024-01-01T00:00:00Z", 1.0)]).unwrap();
    let new = serde_json::to_vec(&vec![record("2024-01-01T00:00:00Z", 2.0)]).unwrap();
    for (key, body) in [
        ("MOEX:SBER", old.clone()),
        ("CBR:USD", old.clone()),
        ("SPBEX:SIBN:2024-01-01", old),
        ("SPBEX:SIBN:2024-01-02", new),
    ] {
        sqlx::query("INSERT INTO history_cache VALUES(?,?,0,0,?)")
            .bind(key)
            .bind(&body)
            .bind(body.len() as i64)
            .execute(&pool)
            .await
            .unwrap();
    }
    pool.close().await;
    let store = CacheStore::open(&path, 1).await.unwrap();
    assert_eq!(
        store
            .collection("moex", "SBER")
            .await
            .unwrap()
            .unwrap()
            .records
            .len(),
        1
    );
    assert_eq!(
        store
            .collection("cbr", "USD")
            .await
            .unwrap()
            .unwrap()
            .records
            .len(),
        1
    );
    let spbex = store.collection("spbex", "SIBN").await.unwrap().unwrap();
    assert_eq!(spbex.records.len(), 1);
    assert_eq!(spbex.records[0].close, Some(2.0));
    assert_eq!(store.collections().await.unwrap().len(), 3);
}

#[tokio::test]
async fn rejects_duplicate_dates_without_mutating_committed_records() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 1)
        .await
        .unwrap();
    let initial = record("2024-01-01T00:00:00Z", 1.0);
    store
        .merge_records("moex", "SBER", &[initial.clone()], true, 10)
        .await
        .unwrap();
    let duplicate = record("2024-01-02T00:00:00Z", 2.0);
    assert!(
        store
            .merge_records("moex", "SBER", &[duplicate.clone(), duplicate], true, 20)
            .await
            .is_err()
    );
    let collection = store.collection("moex", "SBER").await.unwrap().unwrap();
    assert_eq!(collection.records, vec![initial]);
    assert_eq!(collection.last_full_refresh_at, Some(10));
}

#[tokio::test]
async fn failed_transaction_preserves_existing_rows_and_refresh_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite3");
    let store = CacheStore::open(&path, 1).await.unwrap();
    let initial = record("2024-01-01T00:00:00Z", 1.0);
    store
        .merge_records("moex", "SBER", &[initial.clone()], true, 10)
        .await
        .unwrap();
    let admin = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false),
    )
    .await
    .unwrap();
    sqlx::query("CREATE TRIGGER fail_history_record BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END").execute(&admin).await.unwrap();
    assert!(
        store
            .merge_records(
                "moex",
                "SBER",
                &[record("2024-01-02T00:00:00Z", 2.0)],
                true,
                20
            )
            .await
            .is_err()
    );
    let collection = store.collection("moex", "SBER").await.unwrap().unwrap();
    assert_eq!(collection.records, vec![initial]);
    assert_eq!(collection.last_full_refresh_at, Some(10));
    admin.close().await;
}
