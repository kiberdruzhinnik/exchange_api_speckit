use reqwest::{Client, Url};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct MoexClient {
    client: Client,
    base: Url,
    max_response_bytes: usize,
    max_history_bytes: usize,
}

impl MoexClient {
    pub fn new(base: &str, timeout: Duration) -> anyhow::Result<Self> {
        Self::with_limits(base, timeout, 4 * 1024 * 1024, 64 * 1024 * 1024)
    }

    pub fn with_limits(
        base: &str,
        timeout: Duration,
        max_response_bytes: usize,
        max_history_bytes: usize,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            max_response_bytes > 0,
            "maximum response size must be positive"
        );
        anyhow::ensure!(
            max_history_bytes > 0,
            "maximum history size must be positive"
        );
        let base = Url::parse(base)?;
        let client = Client::builder().timeout(timeout).build()?;
        Ok(Self {
            client,
            base,
            max_response_bytes,
            max_history_bytes,
        })
    }
    fn url(&self, path: &str) -> anyhow::Result<Url> {
        Ok(self.base.join(path)?)
    }
    async fn decode_json(&self, mut response: reqwest::Response) -> anyhow::Result<Value> {
        if response
            .content_length()
            .is_some_and(|size| size > self.max_response_bytes as u64)
        {
            anyhow::bail!("MOEX response exceeds configured byte limit");
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            anyhow::ensure!(
                chunk.len() <= self.max_response_bytes.saturating_sub(body.len()),
                "MOEX response exceeds configured byte limit"
            );
            body.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice(&body)?)
    }

    async fn get_json(&self, path: &str, query: &[(&str, String)]) -> anyhow::Result<Value> {
        let response = self
            .client
            .get(self.url(path)?)
            .query(query)
            .send()
            .await?
            .error_for_status()?;
        self.decode_json(response).await
    }
    pub async fn security(&self, symbol: &str) -> anyhow::Result<Option<Value>> {
        let response = self
            .client
            .get(self.url(&format!("securities/{symbol}.json"))?)
            .query(&[("iss.only", "description,boards")])
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(self.decode_json(response.error_for_status()?).await?))
    }

    pub async fn board_security(&self, symbol: &str, board: &str) -> anyhow::Result<Value> {
        self.get_json(
            &format!("engines/stock/markets/shares/boards/{board}/securities/{symbol}.json"),
            &[
                ("iss.only", "securities".into()),
                ("iss.meta", "off".into()),
            ],
        )
        .await
    }

    pub async fn history(&self, symbol: &str) -> anyhow::Result<Value> {
        let mut offset = 0usize;
        let mut rows = Vec::new();
        let mut columns = None;
        let mut history_bytes = 0usize;
        loop {
            let value = self
                .get_json(
                    &format!("history/engines/stock/markets/shares/securities/{symbol}.json"),
                    &[
                        ("iss.only", "history,history.cursor".into()),
                        ("iss.meta", "off".into()),
                        ("start", offset.to_string()),
                        ("limit", "100".into()),
                    ],
                )
                .await?;
            let history = &value["history"];
            let page_columns = history["columns"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("MOEX history has no columns"))?;
            let page_rows = history["data"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("MOEX history has no data"))?;
            columns.get_or_insert_with(|| page_columns.clone());
            if history_bytes == 0 {
                history_bytes = serde_json::to_vec(page_columns)?.len();
                anyhow::ensure!(
                    history_bytes <= self.max_history_bytes,
                    "MOEX history exceeds configured byte limit"
                );
            }
            if page_rows.is_empty() {
                break;
            }
            let page_bytes = serde_json::to_vec(page_rows)?.len();
            anyhow::ensure!(
                page_bytes <= self.max_history_bytes.saturating_sub(history_bytes),
                "MOEX history exceeds configured byte limit"
            );
            history_bytes += page_bytes;
            offset += page_rows.len();
            rows.extend(page_rows.iter().cloned());
            let cursor = &value["history.cursor"];
            let cursor_columns = cursor["columns"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("MOEX cursor missing columns"))?;
            let cursor_row = cursor["data"]
                .as_array()
                .and_then(|v| v.first())
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow::anyhow!("MOEX cursor missing data"))?;
            let total_idx = cursor_columns
                .iter()
                .position(|v| v.as_str() == Some("TOTAL"))
                .ok_or_else(|| anyhow::anyhow!("MOEX cursor missing total"))?;
            let total = cursor_row[total_idx]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("invalid MOEX cursor total"))?
                as usize;
            if offset >= total {
                break;
            }
        }
        Ok(serde_json::json!({"columns": columns.unwrap_or_default(), "data": rows}))
    }
}
