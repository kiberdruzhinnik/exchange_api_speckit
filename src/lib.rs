pub mod cache_store;
pub mod cbr;
pub mod config;
pub mod domain;
pub mod history_refresh;
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
    pub cache_store: Option<cache_store::CacheStore>,
    pub admin_api_key: Option<Arc<str>>,
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
        )
    }

    pub fn with_services(moex: MoexClient, spbex: SpbexClient) -> Self {
        let cbr = CbrClient::new(
            "https://www.cbr.ru/",
            std::time::Duration::from_secs(15),
            16 * 1024 * 1024,
        )
        .expect("valid CBR default client");
        Self::with_all_services(moex, spbex, cbr)
    }

    pub fn with_store(moex: MoexClient, store: cache_store::CacheStore) -> Self {
        let spbex = SpbexClient::new(
            "https://spbexchange.ru/api/",
            std::time::Duration::from_secs(15),
            16 * 1024 * 1024,
        )
        .expect("valid SPBEX default client");
        Self::with_services_and_store(moex, spbex, store)
    }

    pub fn with_services_and_store(
        moex: MoexClient,
        spbex: SpbexClient,
        store: cache_store::CacheStore,
    ) -> Self {
        let cbr = CbrClient::new(
            "https://www.cbr.ru/",
            std::time::Duration::from_secs(15),
            16 * 1024 * 1024,
        )
        .expect("valid CBR default client");
        Self::with_all_services_and_store(moex, spbex, cbr, store)
    }

    pub fn with_all_services_and_store(
        moex: MoexClient,
        spbex: SpbexClient,
        cbr: CbrClient,
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
            cache_store: Some(store),
            admin_api_key: None,
        }
    }

    pub fn with_all_services(moex: MoexClient, spbex: SpbexClient, cbr: CbrClient) -> Self {
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
            cache_store: None,
            admin_api_key: None,
        }
    }

    pub fn with_admin_api_key(mut self, key: Option<String>) -> Self {
        self.admin_api_key = key.map(Arc::<str>::from);
        self
    }
}
