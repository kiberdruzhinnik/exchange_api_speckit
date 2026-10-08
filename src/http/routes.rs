use crate::{
    AppState,
    http::errors::ApiError,
    moex::{
        board::{board_on_date, primary_board_by_date},
        mapping::{lotsize, map_history},
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
use std::time::Instant;
use tower_http::trace::TraceLayer;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}
async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
async fn ready() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ready" })
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
        Ok((body, was_hit)) => {
            tracing::info!(
                symbol,
                elapsed_ms = started.elapsed().as_millis() as u64,
                cache = if was_hit { "hit" } else { "miss" },
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
            } else {
                Err(ApiError::Upstream(anyhow::anyhow!(error)))
            }
        }
    }
}

async fn build_history_response(
    moex: &crate::moex::client::MoexClient,
    symbol: &str,
) -> anyhow::Result<bytes::Bytes> {
    let security = moex
        .security(symbol)
        .await?
        .ok_or_else(|| anyhow::anyhow!("invalid symbol: unknown MOEX symbol"))?;
    let boards = primary_board_by_date(&security)?;
    if boards.is_empty() {
        anyhow::bail!("invalid symbol: MOEX symbol has no primary board history");
    }
    let history = moex.history(symbol).await?;
    let mut selected = Vec::new();
    for (board, from, till) in &boards {
        let reference = moex.board_security(symbol, board).await?;
        let board_lotsize = lotsize(&reference)?;
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
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
