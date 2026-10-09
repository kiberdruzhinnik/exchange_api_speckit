use crate::domain::{DailyMarketRecord, LatestQuoteRecord};

#[derive(Clone, Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("symbol is malformed or unsupported")]
    InvalidSymbol,
    #[error("{0}")]
    Upstream(String),
}

#[async_trait::async_trait]
pub trait ExchangeProvider: Send + Sync {
    fn normalize_symbol(&self, symbol: &str) -> Option<String>;
    fn upstream_code(&self) -> &'static str;
    async fn history(&self, symbol: &str) -> Result<Vec<DailyMarketRecord>, ProviderError>;
    async fn quote(&self, symbol: &str) -> Result<LatestQuoteRecord, ProviderError>;
}
