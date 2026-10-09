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
        mapping::map_history(&rows).map_err(|e| ProviderError::InvalidData(e.to_string()))
    }
    async fn history_since(
        &self,
        symbol: &str,
        after: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        let rows = self
            .0
            .history_since(symbol, after)
            .await
            .map_err(map_error)?;
        let records =
            mapping::map_history(&rows).map_err(|e| ProviderError::InvalidData(e.to_string()))?;
        Ok(records
            .into_iter()
            .filter(|record| after.is_none_or(|date| record.date > date))
            .collect())
    }
    async fn quote(&self, symbol: &str) -> Result<LatestQuoteRecord, ProviderError> {
        let row = self.0.latest(symbol).await.map_err(map_error)?;
        mapping::map_latest(row).map_err(|e| ProviderError::InvalidData(e.to_string()))
    }
}
fn map_error(error: CbrError) -> ProviderError {
    match error {
        CbrError::InvalidSymbol => ProviderError::InvalidSymbol,
        CbrError::HttpStatus(status) => ProviderError::HttpStatus {
            status,
            message: format!("HTTP {status}"),
        },
        CbrError::Transport(message) => ProviderError::Transport(message),
        CbrError::InvalidData(message) => ProviderError::InvalidData(message),
        other => ProviderError::Upstream(other.to_string()),
    }
}
