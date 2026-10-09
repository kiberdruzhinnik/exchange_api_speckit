use crate::{AppState, http::errors::ApiError};
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
    serve_history(
        state.clone(),
        state.providers.moex.clone(),
        symbol,
        "MOEX",
        false,
    )
    .await
}
async fn spbex_history(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Response<Body>, ApiError> {
    serve_history(
        state.clone(),
        state.providers.spbex.clone(),
        symbol,
        "SPBEX",
        true,
    )
    .await
}
async fn cbr_history(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Response<Body>, ApiError> {
    serve_history(
        state.clone(),
        state.providers.cbr.clone(),
        symbol,
        "CBR",
        false,
    )
    .await
}

async fn serve_history(
    state: AppState,
    provider: std::sync::Arc<dyn crate::provider::ExchangeProvider>,
    input_symbol: String,
    namespace: &'static str,
    include_current_date: bool,
) -> Result<Response<Body>, ApiError> {
    let started = Instant::now();
    let symbol = provider
        .normalize_symbol(&input_symbol)
        .ok_or(ApiError::InvalidSymbol)?;
    let date_key = if include_current_date {
        format!(":{}", chrono::Utc::now().date_naive())
    } else {
        String::new()
    };
    let key = format!("{namespace}:{symbol}{date_key}");
    let fetch_provider = provider.clone();
    let fetch_symbol = symbol.clone();
    let (body, outcome) = state
        .history_cache
        .get_or_fetch(key, move || async move {
            tracing::info!(symbol = %fetch_symbol, provider = namespace, phase = "source_fetch", "history source fetch started");
            let records =
                fetch_provider
                    .history(&fetch_symbol)
                    .await
                    .map_err(|error| match error {
                        crate::provider::ProviderError::InvalidSymbol => {
                            "invalid symbol: provider rejected symbol".to_owned()
                        }
                        crate::provider::ProviderError::Upstream(message) => {
                            format!("upstream: {message}")
                        }
                    })?;
            serde_json::to_vec(&records)
                .map(bytes::Bytes::from)
                .map_err(|error| format!("upstream: {error}"))
        })
        .await
        .map_err(|error| {
            if error.starts_with("invalid symbol:") {
                ApiError::InvalidSymbol
            } else if error.starts_with("cache store:") {
                ApiError::Store(error)
            } else {
                ApiError::provider(provider.upstream_code(), anyhow::anyhow!(error))
            }
        })?;
    tracing::info!(
        symbol,
        provider = namespace,
        elapsed_ms = started.elapsed().as_millis() as u64,
        cache = outcome.as_str(),
        status = 200,
        "history request completed"
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .map_err(|error| ApiError::provider(provider.upstream_code(), anyhow::anyhow!(error)))
}

async fn ticker_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<crate::domain::LatestQuoteRecord>>, ApiError> {
    serve_quote(state.providers.moex.clone(), symbol).await
}
async fn spbex_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<crate::domain::LatestQuoteRecord>>, ApiError> {
    serve_quote(state.providers.spbex.clone(), symbol).await
}
async fn cbr_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<crate::domain::LatestQuoteRecord>>, ApiError> {
    serve_quote(state.providers.cbr.clone(), symbol).await
}
async fn serve_quote(
    provider: std::sync::Arc<dyn crate::provider::ExchangeProvider>,
    input_symbol: String,
) -> Result<Json<Vec<crate::domain::LatestQuoteRecord>>, ApiError> {
    let started = Instant::now();
    let symbol = provider
        .normalize_symbol(&input_symbol)
        .ok_or(ApiError::InvalidSymbol)?;
    let quote = provider.quote(&symbol).await.map_err(|error| match error {
        crate::provider::ProviderError::InvalidSymbol => ApiError::InvalidSymbol,
        crate::provider::ProviderError::Upstream(message) => {
            ApiError::provider(provider.upstream_code(), anyhow::anyhow!(message))
        }
    })?;
    tracing::info!(
        symbol,
        elapsed_ms = started.elapsed().as_millis() as u64,
        cache = "bypass",
        status = 200,
        "quote request completed"
    );
    Ok(Json(vec![quote]))
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
