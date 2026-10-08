use crate::spbex::{models::SourceCandle, validation::normalize_symbol};
use bytes::BytesMut;
use reqwest::{Client, StatusCode, Url};
use std::{
    sync::Arc,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

#[derive(Clone, Debug, thiserror::Error)]
pub enum SpbexError {
    #[error("invalid symbol: SPBEX rejected symbol")]
    InvalidSymbol,
    #[error("SPBEX request failed: {0}")]
    Upstream(String),
}

#[derive(Clone)]
pub struct SpbexClient {
    client: Client,
    base: Url,
    max_response_bytes: usize,
    request_limit: Arc<Semaphore>,
}

impl SpbexClient {
    pub fn new(base: &str, timeout: Duration, max_response_bytes: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            max_response_bytes > 0,
            "maximum response size must be positive"
        );
        let mut base = Url::parse(base)?;
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let root = reqwest::Certificate::from_pem(include_bytes!(
            "../../certs/russian_trusted_root_ca.crt"
        ))?;
        let client = Client::builder()
            .add_root_certificate(root)
            .timeout(timeout)
            .build()?;
        Ok(Self {
            client,
            base,
            max_response_bytes,
            request_limit: Arc::new(Semaphore::new(8)),
        })
    }

    pub async fn history(&self, symbol: &str) -> Result<Vec<SourceCandle>, SpbexError> {
        let symbol = normalize_symbol(symbol).ok_or(SpbexError::InvalidSymbol)?;
        let now = unix_seconds();
        self.fetch(&symbol, 0, now).await
    }

    pub async fn latest_candle(&self, symbol: &str) -> Result<Option<SourceCandle>, SpbexError> {
        let symbol = normalize_symbol(symbol).ok_or(SpbexError::InvalidSymbol)?;
        let now = unix_seconds();
        let mut lookback = 24 * 60 * 60i64;
        loop {
            let from = now.saturating_sub(lookback).max(0);
            let candles = self.fetch(&symbol, from, now).await?;
            if let Some(candle) = candles.into_iter().max_by_key(|candle| candle.time) {
                return Ok(Some(candle));
            }
            if from == 0 {
                return Ok(None);
            }
            lookback = lookback.saturating_mul(2);
        }
    }

    async fn fetch(
        &self,
        symbol: &str,
        from: i64,
        to: i64,
    ) -> Result<Vec<SourceCandle>, SpbexError> {
        let _permit = self
            .request_limit
            .acquire()
            .await
            .map_err(|error| SpbexError::Upstream(error.to_string()))?;
        let endpoint = self
            .base
            .join("reader/marketdata/charts/chistory")
            .map_err(|error| SpbexError::Upstream(error.to_string()))?;
        let response = self
            .client
            .get(endpoint)
            .query(&[("symbol", symbol), ("resolution", "1440")])
            .query(&[("from", from.to_string()), ("to", to.to_string())])
            .send()
            .await
            .map_err(|error| SpbexError::Upstream(error.to_string()))?;
        let status = response.status();
        if status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND {
            return Err(SpbexError::InvalidSymbol);
        }
        if !status.is_success() {
            return Err(SpbexError::Upstream(format!("HTTP {status}")));
        }
        let declared = response.content_length().unwrap_or(0);
        if declared > self.max_response_bytes as u64 {
            return Err(SpbexError::Upstream(
                "response exceeds configured byte limit".to_owned(),
            ));
        }
        let mut bytes = BytesMut::new();
        let mut response = response;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| SpbexError::Upstream(error.to_string()))?
        {
            if bytes.len().saturating_add(chunk.len()) > self.max_response_bytes {
                return Err(SpbexError::Upstream(
                    "response exceeds configured byte limit".to_owned(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|error| SpbexError::Upstream(format!("invalid chart JSON: {error}")))
    }
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(i64::MAX as u64) as i64
}
