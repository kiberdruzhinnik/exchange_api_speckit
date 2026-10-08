use bytes::Bytes;
use moka::future::Cache;
use std::{future::Future, sync::Arc, time::Duration};

#[derive(Clone)]
pub struct HistoryCache {
    entries: Cache<String, Result<Bytes, Arc<String>>>,
    max_bytes: u64,
}

impl HistoryCache {
    pub fn new(ttl: Duration, max_bytes: u64) -> Self {
        let entries = Cache::builder()
            .max_capacity(max_bytes)
            .weigher(
                |_key: &String, value: &Result<Bytes, Arc<String>>| match value {
                    Ok(bytes) => u32::try_from(bytes.len()).unwrap_or(u32::MAX),
                    Err(_) => 1,
                },
            )
            .time_to_live(ttl)
            .build();
        Self { entries, max_bytes }
    }

    pub async fn get(&self, key: &str) -> Option<Bytes> {
        self.entries.get(key).await.and_then(Result::ok)
    }

    pub async fn get_or_fetch<F, Fut>(&self, key: String, fetch: F) -> Result<(Bytes, bool), String>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<Bytes, String>> + Send,
    {
        let was_cache_hit = self.get(&key).await.is_some();
        let cache_key = key.clone();
        let value = self
            .entries
            .get_with(key, async move { fetch().await.map_err(Arc::new) })
            .await;
        if value.is_err() {
            self.entries.invalidate(&cache_key).await;
        }
        if value
            .as_ref()
            .is_ok_and(|bytes| bytes.len() as u64 > self.max_bytes)
        {
            self.entries.invalidate(&cache_key).await;
        }
        value
            .map(|bytes| (bytes, was_cache_hit))
            .map_err(|error| (*error).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::HistoryCache;
    use bytes::Bytes;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[tokio::test]
    async fn caches_successful_values_and_deduplicates_concurrent_misses() {
        let cache = HistoryCache::new(Duration::from_secs(1), 1024);
        let calls = Arc::new(AtomicUsize::new(0));
        let mut requests = Vec::new();
        for _ in 0..8 {
            let cache = cache.clone();
            let calls = calls.clone();
            requests.push(tokio::spawn(async move {
                cache
                    .get_or_fetch("SBER".into(), || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok(Bytes::from_static(b"[]"))
                    })
                    .await
                    .unwrap()
            }));
        }
        for request in requests {
            assert_eq!(request.await.unwrap().0, Bytes::from_static(b"[]"));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let result = cache
            .get_or_fetch("SBER".into(), || async {
                panic!("cached response should not call the fetch closure")
            })
            .await
            .unwrap();
        assert_eq!(result, (Bytes::from_static(b"[]"), true));
    }

    #[tokio::test]
    async fn expires_entries_and_does_not_retain_failures() {
        let cache = HistoryCache::new(Duration::from_millis(15), 1024);
        let calls = Arc::new(AtomicUsize::new(0));
        let fail_calls = calls.clone();
        assert!(
            cache
                .get_or_fetch("BAD".into(), || async move {
                    fail_calls.fetch_add(1, Ordering::SeqCst);
                    Err("upstream failed".into())
                })
                .await
                .is_err()
        );
        let success_calls = calls.clone();
        cache
            .get_or_fetch("BAD".into(), || async move {
                success_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Bytes::from_static(b"[]"))
            })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        let expire_calls = calls.clone();
        cache
            .get_or_fetch("BAD".into(), || async move {
                expire_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Bytes::from_static(b"[1]"))
            })
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn does_not_retain_entries_larger_than_the_capacity() {
        let cache = HistoryCache::new(Duration::from_secs(1), 2);
        let calls = Arc::new(AtomicUsize::new(0));
        for expected_call in 1..=2 {
            let fetch_calls = calls.clone();
            let result = cache
                .get_or_fetch("LARGE".into(), || async move {
                    fetch_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(Bytes::from_static(b"123"))
                })
                .await
                .unwrap();
            assert_eq!(result.0, Bytes::from_static(b"123"));
            assert!(!result.1);
            assert_eq!(calls.load(Ordering::SeqCst), expected_call);
        }
    }
}
