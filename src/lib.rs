pub mod cache;
pub mod cache_store;
pub mod cbr;
pub mod config;
pub mod domain;
pub mod http;
pub mod moex;
pub mod provider;
pub mod shutdown;
pub mod spbex;

use cbr::client::CbrClient;
use moex::client::MoexClient;
use spbex::client::SpbexClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct Providers {
    pub moex: Arc<dyn provider::ExchangeProvider>,
    pub spbex: Arc<dyn provider::ExchangeProvider>,
    pub cbr: Arc<dyn provider::ExchangeProvider>,
}

#[derive(Clone)]
pub struct AppState {
    pub moex: MoexClient,
    pub spbex: SpbexClient,
    pub cbr: CbrClient,
    pub providers: Providers,
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
        let cbr = CbrClient::new(
            "https://www.cbr.ru/",
            std::time::Duration::from_secs(15),
            16 * 1024 * 1024,
        )
        .expect("valid CBR default client");
        Self::with_all_services(moex, spbex, cbr, ttl, max_bytes)
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
        let cbr = CbrClient::new(
            "https://www.cbr.ru/",
            std::time::Duration::from_secs(15),
            16 * 1024 * 1024,
        )
        .expect("valid CBR default client");
        Self::with_all_services_and_store(moex, spbex, cbr, ttl, max_bytes, store)
    }

    pub fn with_all_services_and_store(
        moex: MoexClient,
        spbex: SpbexClient,
        cbr: CbrClient,
        ttl: std::time::Duration,
        max_bytes: u64,
        store: cache_store::CacheStore,
    ) -> Self {
        let providers = Providers {
            moex: Arc::new(moex::provider::MoexProvider(moex.clone())),
            spbex: Arc::new(spbex::provider::SpbexProvider(spbex.clone())),
            cbr: Arc::new(cbr::provider::CbrProvider(cbr.clone())),
        };
        Self {
            moex,
            spbex,
            cbr,
            providers,
            history_cache: cache::HistoryCache::with_store(ttl, max_bytes, store.clone()),
            cache_store: Some(store),
        }
    }

    pub fn with_all_services(
        moex: MoexClient,
        spbex: SpbexClient,
        cbr: CbrClient,
        ttl: std::time::Duration,
        max_bytes: u64,
    ) -> Self {
        let providers = Providers {
            moex: Arc::new(moex::provider::MoexProvider(moex.clone())),
            spbex: Arc::new(spbex::provider::SpbexProvider(spbex.clone())),
            cbr: Arc::new(cbr::provider::CbrProvider(cbr.clone())),
        };
        Self {
            moex,
            spbex,
            cbr,
            providers,
            history_cache: cache::HistoryCache::new(ttl, max_bytes),
            cache_store: None,
        }
    }
}
