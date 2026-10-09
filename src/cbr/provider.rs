use crate::{
    cbr::{
        client::{CbrClient, CbrError},
        mapping, validation,
    },
    domain::{DailyMarketRecord, LatestQuoteRecord},
    provider::{ExchangeProvider, ProviderError},
};

#[derive(Clone)]
pub struct CbrProvider(pub CbrClient);

#[async_trait::async_trait]
impl ExchangeProvider for CbrProvider {
    fn normalize_symbol(&self, symbol: &str) -> Option<String> {
        validation::normalize_symbol(symbol)
    }
    fn upstream_code(&self) -> &'static str {
        "cbr_unavailable"
    }
    async fn history(&self, symbol: &str) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        let rows = self.0.history(symbol).await.map_err(map_error)?;
        mapping::map_history(&rows).map_err(|e| ProviderError::Upstream(e.to_string()))
    }
    async fn quote(&self, symbol: &str) -> Result<LatestQuoteRecord, ProviderError> {
        let row = self.0.latest(symbol).await.map_err(map_error)?;
        mapping::map_latest(row).map_err(|e| ProviderError::Upstream(e.to_string()))
    }
}
fn map_error(error: CbrError) -> ProviderError {
    match error {
        CbrError::InvalidSymbol => ProviderError::InvalidSymbol,
        other => ProviderError::Upstream(other.to_string()),
    }
}
