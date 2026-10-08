pub mod cache;
pub mod cache_store;
pub mod config;
pub mod domain;
pub mod http;
pub mod moex;
pub mod shutdown;
pub mod spbex;

use moex::client::MoexClient;
use spbex::client::SpbexClient;

#[derive(Clone)]
pub struct AppState {
    pub moex: MoexClient,
    pub spbex: SpbexClient,
    pub history_cache: cache::HistoryCache,
    pub cache_store: Option<cache_store::CacheStore>,
}

impl AppState {
    pub fn new(moex: MoexClient) -> Self {
        Self::with_services(
            moex,
            SpbexClient::new(
                "https://spbexchange.ru/api/",
                std::time::Duration::from_secs(15),
                16 * 1024 * 1024,
            )
            .expect("valid SPBEX default client"),
            std::time::Duration::from_secs(60),
            64 * 1024 * 1024,
        )
    }

    pub fn with_cache(moex: MoexClient, ttl: std::time::Duration, max_bytes: u64) -> Self {
        Self::with_services(
            moex,
            SpbexClient::new(
                "https://spbexchange.ru/api/",
                std::time::Duration::from_secs(15),
                16 * 1024 * 1024,
            )
            .expect("valid SPBEX default client"),
            ttl,
            max_bytes,
        )
    }

    pub fn with_services(
        moex: MoexClient,
        spbex: SpbexClient,
        ttl: std::time::Duration,
        max_bytes: u64,
    ) -> Self {
        Self {
            moex,
            spbex,
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
        let spbex = SpbexClient::new(
            "https://spbexchange.ru/api/",
            std::time::Duration::from_secs(15),
            16 * 1024 * 1024,
        )
        .expect("valid SPBEX default client");
        Self::with_services_and_store(moex, spbex, ttl, max_bytes, store)
    }

    pub fn with_services_and_store(
        moex: MoexClient,
        spbex: SpbexClient,
        ttl: std::time::Duration,
        max_bytes: u64,
        store: cache_store::CacheStore,
    ) -> Self {
        Self {
            moex,
            spbex,
            history_cache: cache::HistoryCache::with_store(ttl, max_bytes, store.clone()),
            cache_store: Some(store),
        }
    }
}
