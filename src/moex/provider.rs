use crate::{
    domain::{DailyMarketRecord, LatestQuoteRecord},
    moex::{
        board::{board_on_date, primary_board_by_date},
        client::MoexClient,
        mapping::{lotsize, map_history, map_latest_trade},
        validation::validate_symbol,
    },
    provider::{ExchangeProvider, ProviderError},
};
use std::collections::HashMap;
use tokio::task::JoinSet;

#[derive(Clone)]
pub struct MoexProvider(pub MoexClient);

#[async_trait::async_trait]
impl ExchangeProvider for MoexProvider {
    fn normalize_symbol(&self, symbol: &str) -> Option<String> {
        let value = symbol.trim().to_ascii_uppercase();
        validate_symbol(&value).then_some(value)
    }
    fn upstream_code(&self) -> &'static str {
        "moex_unavailable"
    }
    async fn history(&self, symbol: &str) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        build_history(&self.0, symbol).await.map_err(|error| {
            if error.to_string().starts_with("invalid symbol:") {
                ProviderError::InvalidSymbol
            } else {
                ProviderError::Upstream(error.to_string())
            }
        })
    }
    async fn quote(&self, symbol: &str) -> Result<LatestQuoteRecord, ProviderError> {
        let value = self.0.latest_trade(symbol).await.map_err(|error| {
            if error.to_string().starts_with("invalid symbol:") {
                ProviderError::InvalidSymbol
            } else {
                ProviderError::Upstream(error.to_string())
            }
        })?;
        map_latest_trade(&value).map_err(|error| ProviderError::Upstream(error.to_string()))
    }
}

async fn build_history(
    client: &MoexClient,
    symbol: &str,
) -> anyhow::Result<Vec<DailyMarketRecord>> {
    let history_client = client.clone();
    let history_symbol = symbol.to_owned();
    let history_task = tokio::spawn(async move { history_client.history(&history_symbol).await });
    let security = match client.security(symbol).await {
        Err(error) => {
            history_task.abort();
            return Err(error);
        }
        Ok(Some(value)) => value,
        Ok(None) => {
            history_task.abort();
            anyhow::bail!("invalid symbol: unknown MOEX symbol");
        }
    };
    let boards = primary_board_by_date(&security)?;
    if boards.is_empty() {
        history_task.abort();
        anyhow::bail!("invalid symbol: no primary board history");
    }
    let mut refs = JoinSet::new();
    let mut unique: Vec<_> = boards.iter().map(|(board, _, _)| board.as_str()).collect();
    unique.sort_unstable();
    unique.dedup();
    for board in unique {
        let client = client.clone();
        let symbol = symbol.to_owned();
        let board = board.to_owned();
        refs.spawn(async move {
            Ok::<_, anyhow::Error>((
                board.clone(),
                lotsize(&client.board_security(&symbol, &board).await?)?,
            ))
        });
    }
    let mut board_lotsizes = HashMap::new();
    while let Some(result) = refs.join_next().await {
        let (board, size) = result??;
        board_lotsizes.insert(board, size);
    }
    let history = history_task.await??;
    let mut selected = Vec::new();
    for (board, from, till) in &boards {
        let candidates = map_history(
            &history,
            board_lotsizes.get(board).copied().flatten(),
            Some(board),
        )?;
        selected.extend(candidates.into_iter().filter(|record| {
            let day = record.date.format("%Y-%m-%d").to_string();
            day.as_str() >= from.as_str()
                && day.as_str() <= till.as_str()
                && board_on_date(&boards, &day) == Some(board.as_str())
        }));
    }
    selected.sort_by_key(|record| record.date);
    selected.dedup_by_key(|record| record.date);
    Ok(selected)
}
