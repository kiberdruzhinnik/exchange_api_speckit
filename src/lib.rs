pub mod cache;
pub mod cache_store;
pub mod config;
pub mod domain;
pub mod http;
pub mod moex;

use moex::client::MoexClient;

#[derive(Clone)]
pub struct AppState {
    pub moex: MoexClient,
    pub history_cache: cache::HistoryCache,
    pub cache_store: Option<cache_store::CacheStore>,
}

impl AppState {
    pub fn new(moex: MoexClient) -> Self {
        Self::with_cache(moex, std::time::Duration::from_secs(60), 64 * 1024 * 1024)
    }

    pub fn with_cache(moex: MoexClient, ttl: std::time::Duration, max_bytes: u64) -> Self {
        Self {
            moex,
            history_cache: cache::HistoryCache::new(ttl, max_bytes),
            cache_store: None,
        }
    }

    pub fn with_store(
        moex: MoexClient,
        ttl: std::time::Duration,
        max_bytes: u64,
        store: cache_store::CacheStore,
    ) -> Self {
        Self {
            moex,
            history_cache: cache::HistoryCache::with_store(ttl, max_bytes, store.clone()),
            cache_store: Some(store),
        }
    }
}
