use crate::domain::DailyMarketRecord;
use bytes::Bytes;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::collections::HashSet;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::Notify;

#[derive(Clone, Debug, PartialEq)]
pub struct StoredResponse {
    pub body: Bytes,
    pub fresh_for: Duration,
}

#[derive(Clone, Debug)]
pub struct CollectionState {
    pub provider: String,
    pub symbol: String,
    pub records: Vec<DailyMarketRecord>,
    pub last_full_refresh_at: Option<i64>,
    pub consecutive_failures: u32,
    pub next_attempt_at: Option<i64>,
}

#[derive(Clone)]
pub struct CacheStore {
    pool: SqlitePool,
    changed: Arc<Notify>,
}

impl CacheStore {
    pub async fn open(path: impl AsRef<Path>, _max_bytes: u64) -> Result<Self, sqlx::Error> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS history_cache (
                symbol TEXT PRIMARY KEY NOT NULL,
                response_body BLOB NOT NULL,
                fetched_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL DEFAULT 9223372036854775807,
                response_bytes INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;
        // Old rows remain available when upgrading from the expiring response cache.
        sqlx::query("UPDATE history_cache SET expires_at = 9223372036854775807")
            .execute(&pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS history_cache_expiry_idx ON history_cache(expires_at)",
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS history_collections (provider TEXT NOT NULL, symbol TEXT NOT NULL, latest_record_date TEXT, last_full_refresh_at INTEGER, consecutive_failures INTEGER NOT NULL DEFAULT 0, next_attempt_at INTEGER, updated_at INTEGER NOT NULL, PRIMARY KEY(provider, symbol))")
            .execute(&pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS history_records (provider TEXT NOT NULL, symbol TEXT NOT NULL, record_date TEXT NOT NULL, record_json TEXT NOT NULL, PRIMARY KEY(provider, symbol, record_date), FOREIGN KEY(provider, symbol) REFERENCES history_collections(provider, symbol) ON DELETE CASCADE)")
            .execute(&pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS history_schema_migrations (version INTEGER PRIMARY KEY NOT NULL)").execute(&pool).await?;
        let store = Self {
            pool,
            changed: Arc::new(Notify::new()),
        };
        store.migrate_legacy_rows().await?;
        Ok(store)
    }

    async fn migrate_legacy_rows(&self) -> Result<(), sqlx::Error> {
        let migrated: Option<i64> =
            sqlx::query_scalar("SELECT version FROM history_schema_migrations WHERE version=1")
                .fetch_optional(&self.pool)
                .await?;
        if migrated.is_some() {
            return Ok(());
        }
        let rows = sqlx::query_as::<_, (String, Vec<u8>)>(
            "SELECT symbol, response_body FROM history_cache",
        )
        .fetch_all(&self.pool)
        .await?;
        for (key, body) in rows {
            // MOEX symbols must be validated against ISS before their cached history
            // is moved into the durable collection. The first history request does
            // this through `import_moex_legacy_history`.
            if key.starts_with("MOEX:") {
                continue;
            }
            let Some((provider, symbol)) = legacy_identity(&key) else {
                continue;
            };
            let Ok(records) = serde_json::from_slice::<Vec<DailyMarketRecord>>(&body) else {
                continue;
            };
            let _ = self
                .merge_records(&provider, &symbol, &records, false, unix_millis())
                .await?;
        }
        sqlx::query("INSERT OR IGNORE INTO history_schema_migrations(version) VALUES(1)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Import one recognized MOEX symbol's legacy response into its durable
    /// collection. The source row is deleted in the same transaction as the
    /// collection merge, so failed imports remain available for a retry.
    pub async fn import_moex_legacy_history(&self, symbol: &str) -> Result<(), sqlx::Error> {
        let key = format!("MOEX:{symbol}");
        let Some(body) = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT response_body FROM history_cache WHERE symbol = ?",
        )
        .bind(&key)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(());
        };

        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM history_collections WHERE provider='moex' AND symbol=?",
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;
        if exists.is_some() {
            return Ok(());
        }

        let records: Vec<DailyMarketRecord> =
            serde_json::from_slice(&body).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        let mut identities = HashSet::with_capacity(records.len());
        for record in &records {
            if !identities.insert(record.date.to_rfc3339()) {
                return Err(sqlx::Error::Protocol(
                    "duplicate history record date".into(),
                ));
            }
        }

        let mut tx = self.pool.begin().await?;
        let now = unix_millis();
        let latest = records
            .iter()
            .map(|record| record.date)
            .max()
            .map(|date| date.to_rfc3339());
        sqlx::query("INSERT INTO history_collections(provider,symbol,latest_record_date,last_full_refresh_at,consecutive_failures,next_attempt_at,updated_at) VALUES('moex',?,?,NULL,0,NULL,?)")
            .bind(symbol).bind(latest).bind(now).execute(&mut *tx).await?;
        for record in &records {
            sqlx::query("INSERT INTO history_records(provider,symbol,record_date,record_json) VALUES('moex',?,?,?)")
                .bind(symbol)
                .bind(record.date.to_rfc3339())
                .bind(serde_json::to_string(record).map_err(|error| sqlx::Error::Encode(Box::new(error)))?)
                .execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM history_cache WHERE symbol=?")
            .bind(&key)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.changed.notify_one();
        Ok(())
    }

    pub async fn collection(
        &self,
        provider: &str,
        symbol: &str,
    ) -> Result<Option<CollectionState>, sqlx::Error> {
        let meta = sqlx::query_as::<_, (Option<i64>, i64, Option<i64>)>("SELECT last_full_refresh_at, consecutive_failures, next_attempt_at FROM history_collections WHERE provider=? AND symbol=?")
            .bind(provider).bind(symbol).fetch_optional(&self.pool).await?;
        let Some((last_full_refresh_at, failures, next_attempt_at)) = meta else {
            return Ok(None);
        };
        let rows = sqlx::query_scalar::<_, String>("SELECT record_json FROM history_records WHERE provider=? AND symbol=? ORDER BY record_date")
            .bind(provider).bind(symbol).fetch_all(&self.pool).await?;
        let records = rows
            .into_iter()
            .map(|json| {
                serde_json::from_str(&json).map_err(|error| sqlx::Error::Decode(Box::new(error)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(CollectionState {
            provider: provider.into(),
            symbol: symbol.into(),
            records,
            last_full_refresh_at,
            consecutive_failures: failures.max(0) as u32,
            next_attempt_at,
        }))
    }

    pub async fn collections(
        &self,
    ) -> Result<Vec<(String, String, Option<i64>, u32, Option<i64>)>, sqlx::Error> {
        sqlx::query_as("SELECT provider, symbol, last_full_refresh_at, consecutive_failures, next_attempt_at FROM history_collections ORDER BY provider, symbol")
            .fetch_all(&self.pool).await.map(|rows: Vec<(String, String, Option<i64>, i64, Option<i64>)>| rows.into_iter().map(|(p,s,last,fail,next)|(p,s,last,fail.max(0) as u32,next)).collect())
    }

    pub async fn merge_records(
        &self,
        provider: &str,
        symbol: &str,
        records: &[DailyMarketRecord],
        full_success: bool,
        now: i64,
    ) -> Result<(), sqlx::Error> {
        let mut identities = HashSet::with_capacity(records.len());
        for record in records {
            let date = record.date.to_rfc3339();
            if !identities.insert(date) {
                return Err(sqlx::Error::Protocol(
                    "duplicate history record date".into(),
                ));
            }
        }
        let latest = records.iter().map(|r| r.date).max().map(|d| d.to_rfc3339());
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO history_collections(provider,symbol,latest_record_date,last_full_refresh_at,consecutive_failures,next_attempt_at,updated_at) VALUES(?,?,?,?,0,NULL,?) ON CONFLICT(provider,symbol) DO UPDATE SET latest_record_date=CASE WHEN excluded.latest_record_date IS NULL THEN history_collections.latest_record_date WHEN history_collections.latest_record_date IS NULL OR excluded.latest_record_date > history_collections.latest_record_date THEN excluded.latest_record_date ELSE history_collections.latest_record_date END, last_full_refresh_at=CASE WHEN ? THEN excluded.last_full_refresh_at ELSE history_collections.last_full_refresh_at END, consecutive_failures=CASE WHEN ? THEN 0 ELSE history_collections.consecutive_failures END, next_attempt_at=CASE WHEN ? THEN NULL ELSE history_collections.next_attempt_at END, updated_at=excluded.updated_at")
            .bind(provider).bind(symbol).bind(latest).bind(if full_success {Some(now)} else {None}).bind(now).bind(full_success).bind(full_success).bind(full_success).execute(&mut *tx).await?;
        for record in records {
            sqlx::query("INSERT INTO history_records(provider,symbol,record_date,record_json) VALUES(?,?,?,?) ON CONFLICT(provider,symbol,record_date) DO UPDATE SET record_json=excluded.record_json")
                .bind(provider).bind(symbol).bind(record.date.to_rfc3339()).bind(serde_json::to_string(record).map_err(|e| sqlx::Error::Encode(Box::new(e)))?).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        self.changed.notify_one();
        Ok(())
    }

    pub async fn record_failure(
        &self,
        provider: &str,
        symbol: &str,
        retryable: bool,
        interval_secs: u64,
        backoff_cap_secs: u64,
        now: i64,
    ) -> Result<(), sqlx::Error> {
        let prior: i64 = sqlx::query_scalar(
            "SELECT consecutive_failures FROM history_collections WHERE provider=? AND symbol=?",
        )
        .bind(provider)
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(0);
        let count = prior.saturating_add(1);
        let delay = if retryable {
            1u64.checked_shl((count.saturating_sub(1).min(63)) as u32)
                .unwrap_or(u64::MAX)
                .min(backoff_cap_secs)
        } else {
            interval_secs
        };
        sqlx::query("UPDATE history_collections SET consecutive_failures=?, next_attempt_at=?, updated_at=? WHERE provider=? AND symbol=?")
            .bind(count).bind(now.saturating_add(delay.min(i64::MAX as u64) as i64)).bind(now).bind(provider).bind(symbol).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get(&self, symbol: &str) -> Result<Option<StoredResponse>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Vec<u8>, i64)>(
            "SELECT response_body, expires_at FROM history_cache WHERE symbol = ?",
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((body, _)) => Ok(Some(StoredResponse {
                body: Bytes::from(body),
                fresh_for: Duration::MAX,
            })),
            None => Ok(None),
        }
    }

    pub async fn put(&self, symbol: &str, body: &Bytes, ttl: Duration) -> Result<(), sqlx::Error> {
        let now = unix_millis();
        let _ = ttl;
        let expires_at = i64::MAX;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO history_cache(symbol, response_body, fetched_at, expires_at, response_bytes)
             VALUES(?, ?, ?, ?, ?)
             ON CONFLICT(symbol) DO UPDATE SET
               response_body = excluded.response_body,
               fetched_at = excluded.fetched_at,
               expires_at = excluded.expires_at,
               response_bytes = excluded.response_bytes",
        )
        .bind(symbol)
        .bind(body.as_ref())
        .bind(now)
        .bind(expires_at)
        .bind(body.len() as i64)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn is_healthy(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub fn change_notifier(&self) -> Arc<Notify> {
        self.changed.clone()
    }
}

fn unix_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn legacy_identity(key: &str) -> Option<(String, String)> {
    let (prefix, rest) = key.split_once(':')?;
    let provider = match prefix {
        "MOEX" => "moex",
        "SPBEX" => "spbex",
        "CBR" => "cbr",
        _ => return None,
    };
    let symbol = if provider == "spbex" {
        rest.rsplit_once(':')
            .map(|(symbol, _)| symbol)
            .unwrap_or(rest)
    } else {
        rest
    };
    let symbol = symbol.trim().to_ascii_uppercase();
    (!symbol.is_empty()).then_some((provider.to_owned(), symbol))
}

#[cfg(test)]
mod tests {
    use super::CacheStore;
    use bytes::Bytes;
    use std::time::Duration;

    #[tokio::test]
    async fn persists_entries_when_store_is_reopened() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite3");
        let body = Bytes::from_static(b"[]");
        {
            let store = CacheStore::open(&path, 1024).await.unwrap();
            store
                .put("SBER", &body, Duration::from_secs(30))
                .await
                .unwrap();
        }
        let store = CacheStore::open(&path, 1024).await.unwrap();
        assert_eq!(store.get("SBER").await.unwrap().unwrap().body, body);
    }

    #[tokio::test]
    async fn retains_rows_beyond_byte_capacity_and_ignores_ttl() {
        let dir = tempfile::tempdir().unwrap();
        let store = CacheStore::open(dir.path().join("history.sqlite3"), 4)
            .await
            .unwrap();
        store
            .put("AAA", &Bytes::from_static(b"1234"), Duration::from_secs(30))
            .await
            .unwrap();
        store
            .put("BBB", &Bytes::from_static(b"5678"), Duration::from_secs(30))
            .await
            .unwrap();
        assert_eq!(
            store.get("AAA").await.unwrap().unwrap().body,
            Bytes::from_static(b"1234")
        );
        assert_eq!(
            store.get("BBB").await.unwrap().unwrap().body,
            Bytes::from_static(b"5678")
        );
    }
}
