use crate::cbr::models::{CurrencyDirectory, CurrencyItem, DailyRates, DynamicRates};
use bytes::BytesMut;
use reqwest::{Client, StatusCode, Url};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

const DIRECTORY_TTL: Duration = Duration::from_secs(60);
const HISTORY_START_DATE: &str = "01/01/1990";
type CachedDirectory = Arc<RwLock<Option<(Instant, Vec<CurrencyItem>)>>>;

#[derive(Clone, Debug, thiserror::Error)]
pub enum CbrError {
    #[error("invalid symbol: currency is not supported by the Bank of Russia")]
    InvalidSymbol,
    #[error("Bank of Russia request failed: {0}")]
    Upstream(String),
    #[error("Bank of Russia HTTP {0}")]
    HttpStatus(u16),
    #[error("Bank of Russia transport failure: {0}")]
    Transport(String),
    #[error("Bank of Russia invalid response: {0}")]
    InvalidData(String),
}

#[derive(Clone)]
pub struct CbrClient {
    client: Client,
    base: Url,
    max_response_bytes: usize,
    directory: CachedDirectory,
}

impl CbrClient {
    pub fn new(base: &str, timeout: Duration, max_response_bytes: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            max_response_bytes > 0,
            "maximum response size must be positive"
        );
        let mut base = Url::parse(base)?;
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        Ok(Self {
            client: Client::builder().timeout(timeout).build()?,
            base,
            max_response_bytes,
            directory: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn currency(&self, symbol: &str) -> Result<CurrencyItem, CbrError> {
        let items = self.currency_directory().await?;
        crate::cbr::validation::currency_for(&items, symbol)
            .cloned()
            .ok_or(CbrError::InvalidSymbol)
    }

    pub async fn history(&self, symbol: &str) -> Result<DynamicRates, CbrError> {
        self.history_since(symbol, None).await
    }

    pub async fn history_since(
        &self,
        symbol: &str,
        after: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<DynamicRates, CbrError> {
        let currency = self.currency(symbol).await?;
        let end_date = chrono::Utc::now().format("%d/%m/%Y").to_string();
        let url = self.endpoint("scripts/XML_dynamic.asp")?;
        let bytes = self
            .fetch(
                url,
                &[
                    (
                        "date_req1",
                        after
                            .map(|date| {
                                (date.date_naive() + chrono::Days::new(1))
                                    .format("%d/%m/%Y")
                                    .to_string()
                            })
                            .unwrap_or_else(|| HISTORY_START_DATE.to_owned()),
                    ),
                    ("date_req2", end_date),
                    ("VAL_NM_RQ", currency.id),
                ],
            )
            .await?;
        quick_xml::de::from_reader(bytes.as_ref())
            .map_err(|error| CbrError::InvalidData(format!("invalid history XML: {error}")))
    }

    pub async fn latest(
        &self,
        symbol: &str,
    ) -> Result<Option<(String, crate::cbr::models::DailyRate)>, CbrError> {
        self.currency(symbol).await?;
        let url = self.endpoint("scripts/XML_daily.asp")?;
        let bytes = self.fetch(url, &[]).await?;
        let daily: DailyRates = quick_xml::de::from_reader(bytes.as_ref())
            .map_err(|error| CbrError::Upstream(format!("invalid daily XML: {error}")))?;
        let date = daily
            .date
            .ok_or_else(|| CbrError::Upstream("daily response has no effective date".to_owned()))?;
        Ok(daily
            .rates
            .into_iter()
            .find(|rate| {
                rate.char_code
                    .as_deref()
                    .is_some_and(|code| code.eq_ignore_ascii_case(symbol))
            })
            .map(|rate| (date, rate)))
    }

    async fn currency_directory(&self) -> Result<Vec<CurrencyItem>, CbrError> {
        if let Some((fetched_at, items)) = self.directory.read().await.as_ref()
            && fetched_at.elapsed() < DIRECTORY_TTL
        {
            return Ok(items.clone());
        }
        let mut guard = self.directory.write().await;
        if let Some((fetched_at, items)) = guard.as_ref()
            && fetched_at.elapsed() < DIRECTORY_TTL
        {
            return Ok(items.clone());
        }
        // XML_val.asp omits ISO_Char_Code. The full directory supplies the
        // stable CBR IDs; intersect it with today's published rate list so
        // discontinued currencies are not advertised as currently supported.
        let url = self.endpoint("scripts/XML_valFull.asp")?;
        let bytes = self.fetch(url, &[]).await?;
        let directory: CurrencyDirectory =
            quick_xml::de::from_reader(bytes.as_ref()).map_err(|error| {
                CbrError::Upstream(format!("invalid currency directory XML: {error}"))
            })?;
        let daily_url = self.endpoint("scripts/XML_daily.asp")?;
        let daily_bytes = self.fetch(daily_url, &[("d", "0".to_owned())]).await?;
        let daily: DailyRates = quick_xml::de::from_reader(daily_bytes.as_ref())
            .map_err(|error| CbrError::Upstream(format!("invalid daily XML: {error}")))?;
        let current_codes: std::collections::HashSet<String> = daily
            .rates
            .iter()
            .filter_map(|rate| rate.char_code.as_ref())
            .map(|code| code.to_ascii_uppercase())
            .collect();
        let items: Vec<CurrencyItem> = directory
            .items
            .into_iter()
            .filter(|item| {
                item.char_code
                    .as_ref()
                    .is_some_and(|code| current_codes.contains(&code.to_ascii_uppercase()))
            })
            .collect();
        if items.is_empty() {
            return Err(CbrError::Upstream("currency directory is empty".to_owned()));
        }
        *guard = Some((Instant::now(), items.clone()));
        Ok(items)
    }

    fn endpoint(&self, path: &str) -> Result<Url, CbrError> {
        self.base
            .join(path)
            .map_err(|error| CbrError::Upstream(error.to_string()))
    }

    async fn fetch(&self, url: Url, query: &[(&str, String)]) -> Result<Vec<u8>, CbrError> {
        let response = self
            .client
            .get(url)
            .query(query)
            .send()
            .await
            .map_err(|error| CbrError::Transport(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            if status == StatusCode::NOT_FOUND {
                return Err(CbrError::Upstream(format!("HTTP {status}")));
            }
            return Err(CbrError::HttpStatus(status.as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(CbrError::Upstream(
                "response exceeds configured byte limit".to_owned(),
            ));
        }
        let mut response = response;
        let mut bytes = BytesMut::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| CbrError::Transport(error.to_string()))?
        {
            if bytes.len().saturating_add(chunk.len()) > self.max_response_bytes {
                return Err(CbrError::Upstream(
                    "response exceeds configured byte limit".to_owned(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes.to_vec())
    }
}
