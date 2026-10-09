use crate::{
    AppState,
    cbr::{
        client::{CbrClient, CbrError},
        mapping::{map_history as map_cbr_history, map_latest as map_cbr_latest},
        validation::normalize_symbol as normalize_cbr_symbol,
    },
    http::errors::ApiError,
    moex::{
        board::{board_on_date, primary_board_by_date},
        mapping::{lotsize, map_history, map_latest_trade},
        validation::validate_symbol,
    },
    spbex::{
        client::{SpbexClient, SpbexError},
        mapping::{map_history as map_spbex_history, map_latest as map_spbex_latest},
        validation::normalize_symbol,
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

async fn spbex_history(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Response<Body>, ApiError> {
    let started = Instant::now();
    let Some(normalized) = normalize_symbol(&symbol) else {
        tracing::info!(
            symbol,
            elapsed_ms = started.elapsed().as_millis() as u64,
            cache = "bypass",
            status = 400,
            outcome = "invalid_symbol",
            "SPBEX history request completed"
        );
        return Err(ApiError::InvalidSymbol);
    };
    let client = state.spbex.clone();
    let request_symbol = normalized.clone();
    let history_date = chrono::Utc::now().date_naive();
    let key = format!("SPBEX:{normalized}:{history_date}");
    match state
        .history_cache
        .get_or_fetch(key, move || async move {
            build_spbex_history_response(&client, &request_symbol, history_date)
                .await
                .map_err(|error| error.to_string())
        })
        .await
    {
        Ok((body, cache_outcome)) => {
            tracing::info!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = cache_outcome.as_str(), status = 200, outcome = "success", "SPBEX history request completed");
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .map_err(|error| ApiError::Spbex(anyhow::anyhow!(error)))
        }
        Err(error) => {
            let (status, outcome, api_error) = if error.starts_with("cache store:") {
                (503, "history_store_error", ApiError::Store(error))
            } else if error.starts_with("invalid symbol:") {
                (400, "invalid_symbol", ApiError::InvalidSymbol)
            } else {
                (
                    502,
                    "upstream_error",
                    ApiError::Spbex(anyhow::anyhow!(error)),
                )
            };
            tracing::warn!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = "miss", status, outcome, "SPBEX history request failed");
            Err(api_error)
        }
    }
}

async fn spbex_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<crate::domain::LatestTradeRecord>>, ApiError> {
    let started = Instant::now();
    let Some(normalized) = normalize_symbol(&symbol) else {
        tracing::info!(
            symbol,
            elapsed_ms = started.elapsed().as_millis() as u64,
            cache = "bypass",
            status = 400,
            outcome = "invalid_symbol",
            "SPBEX quote request completed"
        );
        return Err(ApiError::InvalidSymbol);
    };
    let candle = match state.spbex.latest_candle(&normalized).await {
        Ok(candle) => candle,
        Err(SpbexError::InvalidSymbol) => {
            tracing::info!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = "bypass", status = 400, outcome = "invalid_symbol", "SPBEX quote request completed");
            return Err(ApiError::InvalidSymbol);
        }
        Err(error) => {
            tracing::warn!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = "bypass", status = 502, outcome = "upstream_error", "SPBEX quote request failed");
            return Err(ApiError::Spbex(anyhow::anyhow!(error)));
        }
    };
    let record = map_spbex_latest(candle.as_ref()).map_err(ApiError::Spbex)?;
    tracing::info!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = "bypass", status = 200, outcome = "success", "SPBEX quote request completed");
    Ok(Json(vec![record]))
}

async fn cbr_history(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Response<Body>, ApiError> {
    let started = Instant::now();
    let Some(normalized) = normalize_cbr_symbol(&symbol) else {
        tracing::info!(
            symbol,
            elapsed_ms = started.elapsed().as_millis() as u64,
            cache = "bypass",
            status = 400,
            outcome = "invalid_symbol",
            "CBR history request completed"
        );
        return Err(ApiError::InvalidSymbol);
    };
    let client = state.cbr.clone();
    let request_symbol = normalized.clone();
    let key = format!("CBR:{normalized}");
    match state
        .history_cache
        .get_or_fetch(key, move || async move {
            build_cbr_history_response(&client, &request_symbol)
                .await
                .map_err(|error| error.to_string())
        })
        .await
    {
        Ok((body, cache_outcome)) => {
            tracing::info!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = cache_outcome.as_str(), status = 200, outcome = "success", "CBR history request completed");
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .map_err(|error| ApiError::Cbr(anyhow::anyhow!(error)))
        }
        Err(error) => {
            if error.starts_with("cache store:") {
                tracing::error!(symbol = %normalized, error = %error, elapsed_ms = started.elapsed().as_millis() as u64, status = 503, "CBR history store failed");
                Err(ApiError::Store(error))
            } else if error.starts_with("invalid symbol:") {
                tracing::info!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, status = 400, outcome = "invalid_symbol", "CBR history request completed");
                Err(ApiError::InvalidSymbol)
            } else {
                tracing::warn!(symbol = %normalized, error = %error, elapsed_ms = started.elapsed().as_millis() as u64, status = 502, "CBR history request failed");
                Err(ApiError::Cbr(anyhow::anyhow!(error)))
            }
        }
    }
}

async fn cbr_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<crate::domain::LatestTradeRecord>>, ApiError> {
    let started = Instant::now();
    let Some(normalized) = normalize_cbr_symbol(&symbol) else {
        tracing::info!(
            symbol,
            elapsed_ms = started.elapsed().as_millis() as u64,
            cache = "bypass",
            status = 400,
            outcome = "invalid_symbol",
            "CBR quote request completed"
        );
        return Err(ApiError::InvalidSymbol);
    };
    let latest = match state.cbr.latest(&normalized).await {
        Ok(latest) => latest,
        Err(CbrError::InvalidSymbol) => {
            tracing::info!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = "bypass", status = 400, outcome = "invalid_symbol", "CBR quote request completed");
            return Err(ApiError::InvalidSymbol);
        }
        Err(error) => {
            tracing::warn!(symbol = %normalized, error = %error, elapsed_ms = started.elapsed().as_millis() as u64, cache = "bypass", status = 502, "CBR quote request failed");
            return Err(ApiError::Cbr(anyhow::anyhow!(error)));
        }
    };
    let record = map_cbr_latest(latest).map_err(ApiError::Cbr)?;
    tracing::info!(symbol = %normalized, elapsed_ms = started.elapsed().as_millis() as u64, cache = "bypass", status = 200, outcome = "success", "CBR quote request completed");
    Ok(Json(vec![record]))
}

async fn build_cbr_history_response(
    client: &CbrClient,
    symbol: &str,
) -> anyhow::Result<bytes::Bytes> {
    tracing::info!(
        symbol,
        outcome = "source_fetch",
        "CBR history source fetch started"
    );
    let history = client.history(symbol).await.map_err(|error| match error {
        CbrError::InvalidSymbol => anyhow::anyhow!("invalid symbol: currency is not supported"),
        other => anyhow::anyhow!("{other}"),
    })?;
    let records = map_cbr_history(&history)?;
    Ok(bytes::Bytes::from(serde_json::to_vec(&records)?))
}

async fn build_spbex_history_response(
    client: &SpbexClient,
    symbol: &str,
    current_date: chrono::NaiveDate,
) -> anyhow::Result<bytes::Bytes> {
    let candles = client.history(symbol).await.map_err(|error| match error {
        SpbexError::InvalidSymbol => anyhow::anyhow!("invalid symbol: explicit SPBEX rejection"),
        other => anyhow::anyhow!("{other}"),
    })?;
    let records = map_spbex_history(&candles, current_date)?;
    Ok(bytes::Bytes::from(serde_json::to_vec(&records)?))
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
        .route("/v1/spbex/{symbol}", get(spbex_history))
        .route("/v1/spbex/{symbol}/quote", get(spbex_quote))
        .route("/v1/cbr/{symbol}", get(cbr_history))
        .route("/v1/cbr/{symbol}/quote", get(cbr_quote))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
