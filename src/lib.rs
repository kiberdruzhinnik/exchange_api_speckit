pub mod cache;
pub mod config;
pub mod domain;
pub mod http;
pub mod moex;

use moex::client::MoexClient;

#[derive(Clone)]
pub struct AppState {
    pub moex: MoexClient,
    pub history_cache: cache::HistoryCache,
}

impl AppState {
    pub fn new(moex: MoexClient) -> Self {
        Self::with_cache(moex, std::time::Duration::from_secs(60), 64 * 1024 * 1024)
    }

    pub fn with_cache(moex: MoexClient, ttl: std::time::Duration, max_bytes: u64) -> Self {
        Self {
            moex,
            history_cache: cache::HistoryCache::new(ttl, max_bytes),
        }
    }
}
