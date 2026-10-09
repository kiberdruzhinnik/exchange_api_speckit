use crate::domain::{DailyMarketRecord, LatestQuoteRecord};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("symbol is malformed or unsupported")]
    InvalidSymbol,
    #[error("{0}")]
    Upstream(String),
    #[error("upstream returned HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },
    #[error("upstream transport failure: {0}")]
    Transport(String),
    #[error("upstream returned invalid or incomplete data: {0}")]
    InvalidData(String),
}

impl ProviderError {
    pub fn retryable(&self) -> bool {
        match self {
            Self::InvalidSymbol => false,
            Self::HttpStatus { status, .. } => {
                *status == 408 || *status == 425 || *status == 429 || *status >= 500
            }
            Self::Upstream(_) | Self::Transport(_) | Self::InvalidData(_) => true,
        }
    }
}

#[async_trait::async_trait]
pub trait ExchangeProvider: Send + Sync {
    fn normalize_symbol(&self, symbol: &str) -> Option<String>;
    fn upstream_code(&self) -> &'static str;
    async fn history(&self, symbol: &str) -> Result<Vec<DailyMarketRecord>, ProviderError>;
    async fn history_since(
        &self,
        symbol: &str,
        _after: Option<DateTime<Utc>>,
    ) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        self.history(symbol).await
    }
    async fn quote(&self, symbol: &str) -> Result<LatestQuoteRecord, ProviderError>;
}
