use crate::{
    AppState,
    http::errors::ApiError,
    moex::{
        board::{board_on_date, primary_board_by_date},
        mapping::{lotsize, map_history, map_latest_trade},
        validation::validate_symbol,
    },
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode, header::CONTENT_TYPE},
    routing::get,
};
use serde::Serialize;
use std::{collections::HashMap, time::Instant};
use tokio::task::JoinSet;
use tower_http::trace::TraceLayer;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}
async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
async fn ready(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    if let Some(store) = &state.cache_store {
        store
            .is_healthy()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    Ok(Json(HealthResponse { status: "ready" }))
}

async fn ticker_history(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Response<Body>, ApiError> {
    let started = Instant::now();
    if !validate_symbol(&symbol) {
        tracing::info!(
            symbol,
            elapsed_ms = started.elapsed().as_millis() as u64,
            cache = "bypass",
            status = 400,
            "ticker history request completed"
        );
        return Err(ApiError::InvalidSymbol);
    }
    let moex = state.moex.clone();
    let request_symbol = symbol.clone();
    match state
        .history_cache
        .get_or_fetch(symbol.clone(), move || async move {
            build_history_response(&moex, &request_symbol)
                .await
                .map_err(|error| error.to_string())
        })
        .await
    {
        Ok((body, cache_outcome)) => {
            tracing::info!(
                symbol,
                elapsed_ms = started.elapsed().as_millis() as u64,
                cache = cache_outcome.as_str(),
                status = 200,
                "ticker history request completed"
            );
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .map_err(|error| ApiError::Upstream(anyhow::anyhow!(error)))
        }
        Err(error) => {
            tracing::warn!(
                symbol,
                elapsed_ms = started.elapsed().as_millis() as u64,
                cache = "miss",
                "ticker history request failed"
            );
            if error.starts_with("invalid symbol:") {
                Err(ApiError::InvalidSymbol)
            } else if error.starts_with("cache store:") {
                Err(ApiError::Store(error))
            } else {
                Err(ApiError::Upstream(anyhow::anyhow!(error)))
            }
        }
    }
}

async fn ticker_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<crate::domain::LatestTradeRecord>>, ApiError> {
    let started = Instant::now();
    if !validate_symbol(&symbol) {
        tracing::info!(
            symbol,
            elapsed_ms = started.elapsed().as_millis() as u64,
            cache = "bypass",
            status = 400,
            outcome = "invalid_symbol",
            "ticker quote request completed"
        );
        return Err(ApiError::InvalidSymbol);
    }
    let response = match state.moex.latest_trade(&symbol).await {
        Ok(response) => response,
        Err(error) => {
            let (status, outcome, api_error) = if error.to_string().starts_with("invalid symbol:") {
                (400, "invalid_symbol", ApiError::InvalidSymbol)
            } else {
                (502, "upstream_error", ApiError::Upstream(error))
            };
            tracing::info!(
                symbol,
                elapsed_ms = started.elapsed().as_millis() as u64,
                cache = "bypass",
                status,
                outcome,
                "ticker quote request completed"
            );
            return Err(api_error);
        }
    };
    let record = match map_latest_trade(&response) {
        Ok(record) => record,
        Err(error) => {
            tracing::info!(
                symbol,
                elapsed_ms = started.elapsed().as_millis() as u64,
                cache = "bypass",
                status = 502,
                outcome = "malformed_response",
                "ticker quote request completed"
            );
            return Err(ApiError::Upstream(error));
        }
    };
    tracing::info!(
        symbol,
        elapsed_ms = started.elapsed().as_millis() as u64,
        cache = "bypass",
        status = 200,
        outcome = "success",
        "ticker quote request completed"
    );
    Ok(Json(vec![record]))
}

async fn build_history_response(
    moex: &crate::moex::client::MoexClient,
    symbol: &str,
) -> anyhow::Result<bytes::Bytes> {
    let history_client = moex.clone();
    let history_symbol = symbol.to_owned();
    let history_task = tokio::spawn(async move { history_client.history(&history_symbol).await });
    let security_result = moex.security(symbol).await;
    let security = match security_result {
        Err(error) => {
            history_task.abort();
            return Err(error);
        }
        Ok(Some(security)) => security,
        Ok(None) => {
            history_task.abort();
            anyhow::bail!("invalid symbol: unknown MOEX symbol");
        }
    };
    let boards = primary_board_by_date(&security)?;
    if boards.is_empty() {
        history_task.abort();
        anyhow::bail!("invalid symbol: MOEX symbol has no primary board history");
    }
    let mut board_references = JoinSet::new();
    let mut unique_boards = boards
        .iter()
        .map(|(board, _, _)| board.as_str())
        .collect::<Vec<_>>();
    unique_boards.sort_unstable();
    unique_boards.dedup();
    for board in unique_boards {
        let client = moex.clone();
        let ticker = symbol.to_owned();
        let board_id = board.to_owned();
        board_references.spawn(async move {
            let reference = client.board_security(&ticker, &board_id).await?;
            Ok::<_, anyhow::Error>((board_id, lotsize(&reference)?))
        });
    }
    let mut board_lotsizes = HashMap::new();
    while let Some(reference) = board_references.join_next().await {
        let (board, board_lotsize) = reference??;
        board_lotsizes.insert(board, board_lotsize);
    }
    let history = history_task.await??;
    let mut selected = Vec::new();
    for (board, from, till) in &boards {
        let board_lotsize = board_lotsizes.get(board).copied().flatten();
        let candidates = map_history(&history, board_lotsize, Some(board))?;
        selected.extend(candidates.into_iter().filter(|record| {
            let day = record.date.format("%Y-%m-%d").to_string();
            day.as_str() >= from.as_str()
                && day.as_str() <= till.as_str()
                && board_on_date(&boards, &day) == Some(board.as_str())
        }));
    }
    selected.sort_by_key(|record| record.date);
    selected.dedup_by_key(|record| record.date);
    Ok(bytes::Bytes::from(serde_json::to_vec(&selected)?))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/v1/moex/{symbol}", get(ticker_history))
        .route("/v1/moex/{symbol}/quote", get(ticker_quote))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
