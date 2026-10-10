use crate::{
    domain::{DailyMarketRecord, LatestQuoteRecord},
    moex::{
        board::{board_on_date, resolve_instrument},
        client::MoexClient,
        mapping::{lotsize, map_history, map_latest_trade},
        models::{InstrumentCategory, ResolvedInstrument},
        validation::validate_symbol,
    },
    provider::{ExchangeProvider, ProviderError, ProviderHistoryContext},
};
use chrono::{DateTime, Utc};

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
        let context = self.resolve(symbol).await?;
        fetch_history(&self.0, symbol, None, &context).await
    }

    async fn history_since(
        &self,
        symbol: &str,
        after: Option<DateTime<Utc>>,
    ) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        let context = self.resolve(symbol).await?;
        fetch_history(
            &self.0,
            symbol,
            after.map(|date| date.date_naive()),
            &context,
        )
        .await
    }

    async fn history_preflight(
        &self,
        symbol: &str,
    ) -> Result<Option<ProviderHistoryContext>, ProviderError> {
        self.resolve(symbol)
            .await
            .map(|context| Some(ProviderHistoryContext::new(context)))
    }

    async fn history_with_context(
        &self,
        symbol: &str,
        after: Option<DateTime<Utc>>,
        context: Option<ProviderHistoryContext>,
    ) -> Result<Vec<DailyMarketRecord>, ProviderError> {
        let context =
            match context.and_then(|value| value.downcast_ref::<ResolvedInstrument>().cloned()) {
                Some(context) => context,
                None => self.resolve(symbol).await?,
            };
        fetch_history(
            &self.0,
            symbol,
            after.map(|date| date.date_naive()),
            &context,
        )
        .await
    }

    async fn quote(&self, symbol: &str) -> Result<LatestQuoteRecord, ProviderError> {
        let context = self.resolve(symbol).await?;
        let board = current_assignment(&context).ok_or(ProviderError::InvalidSymbol)?;
        match context.category {
            InstrumentCategory::Shares => {
                let value = self.0.latest_trade(symbol).await.map_err(map_error)?;
                let quote = map_latest_trade(&value)
                    .map_err(|error| ProviderError::InvalidData(error.to_string()))?;
                Ok(quote)
            }
            InstrumentCategory::Index | InstrumentCategory::Currency => {
                let value = self
                    .0
                    .current_marketdata(symbol, &board.engine, &board.market, &board.board)
                    .await
                    .map_err(map_error)?;
                let lotsize = if context.category == InstrumentCategory::Currency {
                    lotsize(&value)
                        .map_err(|error| ProviderError::InvalidData(error.to_string()))?
                } else {
                    None
                };
                if context.category == InstrumentCategory::Index {
                    crate::moex::index::map_quote(&value)
                        .map_err(|error| ProviderError::InvalidData(error.to_string()))
                } else {
                    crate::moex::currency::map_quote(&value, lotsize)
                        .map_err(|error| ProviderError::InvalidData(error.to_string()))
                }
            }
        }
    }
}

impl MoexProvider {
    async fn resolve(&self, symbol: &str) -> Result<ResolvedInstrument, ProviderError> {
        let metadata = self.0.security(symbol).await.map_err(map_error)?;
        let Some(metadata) = metadata else {
            return Err(ProviderError::InvalidSymbol);
        };
        let context = resolve_instrument(&metadata)
            .map_err(|error| ProviderError::InvalidData(error.to_string()))?
            .ok_or(ProviderError::InvalidSymbol)?;
        Ok(context)
    }
}

fn current_assignment(
    context: &ResolvedInstrument,
) -> Option<&crate::moex::models::BoardAssignment> {
    let today = Utc::now()
        .with_timezone(&chrono_tz::Europe::Moscow)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    context
        .boards
        .iter()
        .find(|board| {
            board.history_from.as_str() <= today.as_str()
                && today.as_str() <= board.history_till.as_str()
        })
        .or_else(|| context.boards.last())
}

async fn fetch_history(
    client: &MoexClient,
    symbol: &str,
    after: Option<chrono::NaiveDate>,
    context: &ResolvedInstrument,
) -> Result<Vec<DailyMarketRecord>, ProviderError> {
    let mut records = Vec::new();
    let mut resolved_context = context.clone();
    let unscoped_shares_history = if context.category == InstrumentCategory::Shares {
        Some(
            client
                .history_market_since(symbol, &context.engine, &context.market, "", after)
                .await
                .map_err(map_error)?,
        )
    } else {
        None
    };
    for assignment in &context.boards {
        let source = match &unscoped_shares_history {
            Some(source) => source.clone(),
            None => client
                .history_market_since(
                    symbol,
                    &assignment.engine,
                    &assignment.market,
                    &assignment.board,
                    after,
                )
                .await
                .map_err(map_error)?,
        };
        if context.category == InstrumentCategory::Shares
            && !resolved_context.lotsizes.contains_key(&assignment.board)
        {
            let security = client
                .board_security_in_market(
                    symbol,
                    &assignment.engine,
                    &assignment.market,
                    &assignment.board,
                )
                .await
                .map_err(map_error)?;
            let size = lotsize(&security)
                .map_err(|error| ProviderError::InvalidData(error.to_string()))?;
            resolved_context
                .lotsizes
                .insert(assignment.board.clone(), size);
        }
        let board_lotsize = resolved_context
            .lotsizes
            .get(&assignment.board)
            .copied()
            .flatten();
        let selected = match context.category {
            InstrumentCategory::Shares => {
                map_history(&source, board_lotsize, Some(&assignment.board))
            }
            InstrumentCategory::Index => {
                crate::moex::index::map_history(&source, &assignment.board)
            }
            InstrumentCategory::Currency => {
                crate::moex::currency::map_history(&source, &assignment.board)
            }
        }
        .map_err(|error| ProviderError::InvalidData(error.to_string()))?;
        records.extend(selected.into_iter().filter(|record| {
            let date = record.date.format("%Y-%m-%d").to_string();
            board_on_date(
                &context
                    .boards
                    .iter()
                    .map(|item| {
                        (
                            item.board.clone(),
                            item.history_from.clone(),
                            item.history_till.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
                &date,
            ) == Some(assignment.board.as_str())
                && after.is_none_or(|date| record.date.date_naive() > date)
        }));
    }
    records.sort_by_key(|record| record.date);
    Ok(records)
}

fn map_error(error: anyhow::Error) -> ProviderError {
    if let Some(status) = error
        .to_string()
        .strip_prefix("HTTP ")
        .and_then(|value| value.parse().ok())
    {
        ProviderError::HttpStatus {
            status,
            message: error.to_string(),
        }
    } else {
        ProviderError::Upstream(error.to_string())
    }
}
