use crate::{
    domain::{DailyMarketRecord, LatestQuoteRecord},
    provider::{ExchangeProvider, ProviderError},
    spbex::{
        client::{SpbexClient, SpbexError},
        mapping,
        validation::normalize_symbol,
    },
};

#[derive(Clone)]
pub struct SpbexProvider(pub SpbexClient);

#[async_trait::async_trait]
impl ExchangeProvider for SpbexProvider {
    fn normalize_symbol(&self, symbol: &str) -> Option<String> {
        normalize_symbol(symbol)
    }
    fn upstream_code(&self) -> &'static str {
        "spbex_unavailable"
    }
    async fn history(&self, symbol: &str) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        let candles = self.0.history(symbol).await.map_err(map_error)?;
        mapping::map_history(&candles, chrono::Utc::now().date_naive())
            .map_err(|e| ProviderError::InvalidData(e.to_string()))
    }
    async fn history_since(
        &self,
        symbol: &str,
        after: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        let candles = self
            .0
            .history_since(symbol, after)
            .await
            .map_err(map_error)?;
        let records = mapping::map_history(&candles, chrono::Utc::now().date_naive())
            .map_err(|e| ProviderError::InvalidData(e.to_string()))?;
        Ok(records
            .into_iter()
            .filter(|record| after.is_none_or(|date| record.date > date))
            .collect())
    }
    async fn quote(&self, symbol: &str) -> Result<LatestQuoteRecord, ProviderError> {
        let candle = self.0.latest_candle(symbol).await.map_err(map_error)?;
        mapping::map_latest(candle.as_ref()).map_err(|e| ProviderError::Upstream(e.to_string()))
    }
}
fn map_error(error: SpbexError) -> ProviderError {
    match error {
        SpbexError::InvalidSymbol => ProviderError::InvalidSymbol,
        SpbexError::HttpStatus(status) => ProviderError::HttpStatus {
            status,
            message: format!("HTTP {status}"),
        },
        SpbexError::Transport(message) => ProviderError::Transport(message),
        SpbexError::InvalidData(message) => ProviderError::InvalidData(message),
        other => ProviderError::Upstream(other.to_string()),
    }
}
