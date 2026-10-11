use exchange_api::cache_store::CacheStore;

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
async fn durable_collections_survive_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite3");
    let expected = record("2026-10-06T00:00:00Z", 42.0);
    {
        let store = CacheStore::open(&path).await.unwrap();
        store
            .merge_records("moex", "SBER", &[expected.clone()], true, 10)
            .await
            .unwrap();
    }
    let reopened = CacheStore::open(&path).await.unwrap();
    assert_eq!(
        reopened
            .collection("moex", "SBER")
            .await
            .unwrap()
            .unwrap()
            .records,
        vec![expected]
    );
}

#[tokio::test]
async fn rejects_duplicate_dates_without_mutating_committed_records() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"))
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
    let store = CacheStore::open(&path).await.unwrap();
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

#[tokio::test]
async fn clearing_a_collection_preserves_other_providers_and_symbols() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"))
        .await
        .unwrap();
    store
        .merge_records(
            "moex",
            "SBER",
            &[record("2026-10-01T00:00:00Z", 10.0)],
            true,
            10,
        )
        .await
        .unwrap();
    store
        .merge_records(
            "moex",
            "GAZP",
            &[record("2026-10-01T00:00:00Z", 20.0)],
            true,
            10,
        )
        .await
        .unwrap();
    store
        .merge_records(
            "cbr",
            "SBER",
            &[record("2026-10-01T00:00:00Z", 30.0)],
            true,
            10,
        )
        .await
        .unwrap();
    store.clear_history("moex", "SBER").await.unwrap();
    assert!(store.collection("moex", "SBER").await.unwrap().is_none());
    assert!(store.collection("moex", "GAZP").await.unwrap().is_some());
    assert!(store.collection("cbr", "SBER").await.unwrap().is_some());
}

#[tokio::test]
async fn upgrade_drops_legacy_response_cache_and_migration_tables_but_keeps_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let pool = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true),
    )
    .await
    .unwrap();
    sqlx::query("CREATE TABLE history_collections (provider TEXT NOT NULL, symbol TEXT NOT NULL, latest_record_date TEXT, last_full_refresh_at INTEGER, consecutive_failures INTEGER NOT NULL DEFAULT 0, next_attempt_at INTEGER, updated_at INTEGER NOT NULL, PRIMARY KEY(provider, symbol))").execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE history_records (provider TEXT NOT NULL, symbol TEXT NOT NULL, record_date TEXT NOT NULL, record_json TEXT NOT NULL, PRIMARY KEY(provider, symbol, record_date))").execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE history_cache (symbol TEXT PRIMARY KEY NOT NULL, response_body BLOB NOT NULL, fetched_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, response_bytes INTEGER NOT NULL)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO history_cache VALUES('MOEX:SBER',X'5B5D',0,0,2)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE history_schema_migrations (version INTEGER PRIMARY KEY NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO history_schema_migrations VALUES(1)")
        .execute(&pool)
        .await
        .unwrap();
    let expected = record("2026-10-06T00:00:00Z", 42.0);
    sqlx::query("INSERT INTO history_collections(provider,symbol,latest_record_date,last_full_refresh_at,consecutive_failures,next_attempt_at,updated_at) VALUES('moex','SBER',?,10,0,NULL,10)").bind(expected.date.to_rfc3339()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO history_records VALUES('moex','SBER',?,?)")
        .bind(expected.date.to_rfc3339())
        .bind(serde_json::to_string(&expected).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let store = CacheStore::open(&path).await.unwrap();
    assert_eq!(
        store
            .collection("moex", "SBER")
            .await
            .unwrap()
            .unwrap()
            .records,
        vec![expected]
    );
    let pool = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false),
    )
    .await
    .unwrap();
    for table in ["history_cache", "history_schema_migrations"] {
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?")
                .bind(table)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(exists, 0, "{table} should be removed");
    }
}
