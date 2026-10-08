use bytes::Bytes;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{path::Path, time::Duration};

#[derive(Clone, Debug, PartialEq)]
pub struct StoredResponse {
    pub body: Bytes,
    pub fresh_for: Duration,
}

#[derive(Clone)]
pub struct CacheStore {
    pool: SqlitePool,
    max_bytes: u64,
}

impl CacheStore {
    pub async fn open(path: impl AsRef<Path>, max_bytes: u64) -> Result<Self, sqlx::Error> {
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
                expires_at INTEGER NOT NULL,
                response_bytes INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS history_cache_expiry_idx ON history_cache(expires_at)",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool, max_bytes })
    }

    pub async fn get(&self, symbol: &str) -> Result<Option<StoredResponse>, sqlx::Error> {
        let now = unix_millis();
        let row = sqlx::query_as::<_, (Vec<u8>, i64)>(
            "SELECT response_body, expires_at FROM history_cache WHERE symbol = ?",
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((body, expires_at)) if expires_at > now => Ok(Some(StoredResponse {
                body: Bytes::from(body),
                fresh_for: Duration::from_millis((expires_at - now) as u64),
            })),
            Some(_) => {
                sqlx::query("DELETE FROM history_cache WHERE symbol = ?")
                    .bind(symbol)
                    .execute(&self.pool)
                    .await?;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    pub async fn put(&self, symbol: &str, body: &Bytes, ttl: Duration) -> Result<(), sqlx::Error> {
        let now = unix_millis();
        let ttl_millis = ttl.as_millis().min(i64::MAX as u128) as i64;
        let expires_at = now.saturating_add(ttl_millis);
        let mut tx = self.pool.begin().await?;
        if body.len() as u64 > self.max_bytes {
            sqlx::query("DELETE FROM history_cache WHERE symbol = ?")
                .bind(symbol)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(());
        }
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
        sqlx::query("DELETE FROM history_cache WHERE expires_at <= ?")
            .bind(now)
            .execute(&mut *tx)
            .await?;
        // Remove oldest rows until the byte budget is met. The loop is bounded by row count.
        loop {
            let used: i64 =
                sqlx::query_scalar("SELECT COALESCE(SUM(response_bytes), 0) FROM history_cache")
                    .fetch_one(&mut *tx)
                    .await?;
            if used <= self.max_bytes as i64 {
                break;
            }
            sqlx::query(
                "DELETE FROM history_cache WHERE symbol = (
                    SELECT symbol FROM history_cache ORDER BY fetched_at ASC LIMIT 1
                 )",
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn is_healthy(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

fn unix_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
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
    async fn prunes_expired_and_oldest_entries_to_respect_capacity() {
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
        assert_eq!(store.get("AAA").await.unwrap(), None);
        assert_eq!(
            store.get("BBB").await.unwrap().unwrap().body,
            Bytes::from_static(b"5678")
        );
    }
}
