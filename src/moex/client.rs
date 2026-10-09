use chrono::NaiveDate;
use reqwest::{Client, Url};
use serde_json::Value;
use std::time::Duration;
use tokio::task::JoinSet;

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
        let response = self.client.get(self.url(path)?).query(query).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status().as_u16());
        }
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
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status().as_u16());
        }
        Ok(Some(self.decode_json(response).await?))
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

    pub async fn latest_trade(&self, symbol: &str) -> anyhow::Result<Value> {
        let response = self
            .client
            .get(self.url(&format!(
                "engines/stock/markets/shares/securities/{symbol}/trades.json"
            ))?)
            .query(&[("limit", "1"), ("reversed", "1"), ("iss.meta", "off")])
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("invalid symbol: unknown MOEX symbol");
        }
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status().as_u16());
        }
        self.decode_json(response).await
    }

    async fn history_page(
        &self,
        symbol: &str,
        offset: usize,
        after: Option<NaiveDate>,
    ) -> anyhow::Result<Value> {
        let mut query = vec![
            ("iss.only", "history,history.cursor".into()),
            ("iss.meta", "off".into()),
            ("start", offset.to_string()),
            ("limit", "100".into()),
        ];
        if let Some(after) = after {
            query.push((
                "from",
                (after + chrono::Days::new(1))
                    .format("%Y-%m-%d")
                    .to_string(),
            ));
        }
        self.get_json(
            &format!("history/engines/stock/markets/shares/securities/{symbol}.json"),
            &query,
        )
        .await
    }

    pub async fn history(&self, symbol: &str) -> anyhow::Result<Value> {
        self.history_since(symbol, None).await
    }

    pub async fn history_since(
        &self,
        symbol: &str,
        after: Option<NaiveDate>,
    ) -> anyhow::Result<Value> {
        const MAX_PARALLEL_HISTORY_PAGES: usize = 128;
        let first_page = self.history_page(symbol, 0, after).await?;
        let history = &first_page["history"];
        let columns = history["columns"]
            .as_array()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("MOEX history has no columns"))?;
        let first_rows = history["data"]
            .as_array()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("MOEX history has no data"))?;
        let cursor_columns = first_page["history.cursor"]["columns"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("MOEX cursor missing columns"))?;
        let cursor_row = first_page["history.cursor"]["data"]
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("MOEX cursor missing data"))?;
        let total_index = cursor_columns
            .iter()
            .position(|column| column.as_str() == Some("TOTAL"))
            .ok_or_else(|| anyhow::anyhow!("MOEX cursor missing total"))?;
        let page_size_index = cursor_columns
            .iter()
            .position(|column| column.as_str() == Some("PAGESIZE"))
            .ok_or_else(|| anyhow::anyhow!("MOEX cursor missing page size"))?;
        let total = cursor_row[total_index]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("invalid MOEX cursor total"))?
            as usize;
        let page_size = cursor_row[page_size_index]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| anyhow::anyhow!("invalid MOEX cursor page size"))?
            as usize;

        let mut rows = first_rows;
        anyhow::ensure!(
            rows.len() == total.min(page_size),
            "MOEX first page row count does not match cursor"
        );
        let mut history_bytes =
            serde_json::to_vec(&columns)?.len() + serde_json::to_vec(&rows)?.len();
        anyhow::ensure!(
            history_bytes <= self.max_history_bytes,
            "MOEX history exceeds configured byte limit"
        );
        let mut offsets = (page_size..total).step_by(page_size);
        let mut pages = JoinSet::new();
        let mut completed_pages = Vec::new();
        loop {
            while pages.len() < MAX_PARALLEL_HISTORY_PAGES {
                let Some(offset) = offsets.next() else { break };
                let client = self.clone();
                let symbol = symbol.to_owned();
                pages.spawn(async move {
                    let value = client.history_page(&symbol, offset, after).await?;
                    Ok::<_, anyhow::Error>((offset, value))
                });
            }
            let Some(joined) = pages.join_next().await else {
                break;
            };
            let (offset, page) = joined??;
            let page_history = &page["history"];
            let page_columns = page_history["columns"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("MOEX history page has no columns"))?;
            anyhow::ensure!(
                *page_columns == columns,
                "MOEX history page columns changed during pagination"
            );
            let page_rows = page_history["data"]
                .as_array()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("MOEX history page has no data"))?;
            anyhow::ensure!(
                page_rows.len() == (total - offset).min(page_size),
                "MOEX page row count does not match cursor"
            );
            let page_bytes = serde_json::to_vec(&page_rows)?.len();
            anyhow::ensure!(
                page_bytes <= self.max_history_bytes.saturating_sub(history_bytes),
                "MOEX history exceeds configured byte limit"
            );
            history_bytes += page_bytes;
            completed_pages.push((offset, page_rows));
        }
        completed_pages.sort_by_key(|(offset, _)| *offset);
        for (_, page_rows) in completed_pages {
            rows.extend(page_rows);
        }
        Ok(serde_json::json!({"columns": columns, "data": rows}))
    }
}
