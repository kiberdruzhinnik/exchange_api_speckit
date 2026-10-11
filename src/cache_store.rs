use crate::domain::DailyMarketRecord;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::collections::HashSet;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::Notify;

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
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, sqlx::Error> {
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
        sqlx::query("CREATE TABLE IF NOT EXISTS history_collections (provider TEXT NOT NULL, symbol TEXT NOT NULL, latest_record_date TEXT, last_full_refresh_at INTEGER, consecutive_failures INTEGER NOT NULL DEFAULT 0, next_attempt_at INTEGER, updated_at INTEGER NOT NULL, PRIMARY KEY(provider, symbol))")
            .execute(&pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS history_records (provider TEXT NOT NULL, symbol TEXT NOT NULL, record_date TEXT NOT NULL, record_json TEXT NOT NULL, PRIMARY KEY(provider, symbol, record_date), FOREIGN KEY(provider, symbol) REFERENCES history_collections(provider, symbol) ON DELETE CASCADE)")
            .execute(&pool).await?;
        let store = Self {
            pool,
            changed: Arc::new(Notify::new()),
        };
        sqlx::query("DROP TABLE IF EXISTS history_schema_migrations")
            .execute(&store.pool)
            .await?;
        sqlx::query("DROP TABLE IF EXISTS history_cache")
            .execute(&store.pool)
            .await?;
        Ok(store)
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

    /// Remove one provider/symbol history collection and its records.
    pub async fn clear_history(&self, provider: &str, symbol: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM history_records WHERE provider=? AND symbol=?")
            .bind(provider)
            .bind(symbol)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM history_collections WHERE provider=? AND symbol=?")
            .bind(provider)
            .bind(symbol)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.changed.notify_one();
        Ok(())
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

    pub async fn is_healthy(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub fn change_notifier(&self) -> Arc<Notify> {
        self.changed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::CacheStore;

    #[tokio::test]
    async fn durable_collections_survive_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite3");
        {
            let store = CacheStore::open(&path).await.unwrap();
            store
                .merge_records("moex", "SBER", &[], true, 10)
                .await
                .unwrap();
        }
        let store = CacheStore::open(&path).await.unwrap();
        assert!(store.collection("moex", "SBER").await.unwrap().is_some());
    }
}
