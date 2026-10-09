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
    _include_current_date: bool,
) -> Result<Response<Body>, ApiError> {
    let started = Instant::now();
    let symbol = provider
        .normalize_symbol(&input_symbol)
        .ok_or(ApiError::InvalidSymbol)?;
    if let Some(store) = &state.cache_store {
        let previous = store
            .collection(namespace.to_ascii_lowercase().as_str(), &symbol)
            .await
            .map_err(|e| ApiError::Store(e.to_string()))?;
        let after = previous
            .as_ref()
            .and_then(|c| c.records.last().map(|r| r.date));
        tracing::info!(symbol = %symbol, provider = namespace, phase = "source_fetch", "history source fetch started");
        let records = provider
            .history_since(&symbol, after)
            .await
            .map_err(|e| map_history_provider_error(provider.upstream_code(), e))?;
        validate_request_records(provider.upstream_code(), &records)?;
        store
            .merge_records(
                namespace.to_ascii_lowercase().as_str(),
                &symbol,
                &records,
                false,
                chrono::Utc::now().timestamp(),
            )
            .await
            .map_err(|e| ApiError::Store(e.to_string()))?;
        let collection = store
            .collection(namespace.to_ascii_lowercase().as_str(), &symbol)
            .await
            .map_err(|e| ApiError::Store(e.to_string()))?
            .expect("collection created by merge");
        let body = serde_json::to_vec(&collection.records)
            .map_err(|e| ApiError::provider(provider.upstream_code(), anyhow::anyhow!(e)))?;
        tracing::info!(
            symbol,
            provider = namespace,
            elapsed_ms = started.elapsed().as_millis() as u64,
            cache = "persistent_collection",
            status = 200,
            "history request completed"
        );
        return Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .map_err(|e| ApiError::provider(provider.upstream_code(), anyhow::anyhow!(e)));
    }
    let key = format!("{namespace}:{symbol}");
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
                        error => if matches!(error, crate::provider::ProviderError::InvalidSymbol) { format!("invalid symbol: {error}") } else { format!("upstream: {error}") },
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
    let quote = provider
        .quote(&symbol)
        .await
        .map_err(|e| map_history_provider_error(provider.upstream_code(), e))?;
    tracing::info!(
        symbol,
        elapsed_ms = started.elapsed().as_millis() as u64,
        cache = "bypass",
        status = 200,
        "quote request completed"
    );
    Ok(Json(vec![quote]))
}

fn map_history_provider_error(
    code: &'static str,
    error: crate::provider::ProviderError,
) -> ApiError {
    match error {
        crate::provider::ProviderError::InvalidSymbol => ApiError::InvalidSymbol,
        other => ApiError::provider(code, anyhow::anyhow!(other.to_string())),
    }
}

fn validate_request_records(
    code: &'static str,
    records: &[crate::domain::DailyMarketRecord],
) -> Result<(), ApiError> {
    let mut dates = std::collections::HashSet::with_capacity(records.len());
    for record in records {
        if !dates.insert(record.date)
            || [
                record.close,
                record.high,
                record.low,
                record.volume,
                record.facevalue,
            ]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(ApiError::provider(
                code,
                anyhow::anyhow!("upstream returned duplicate or invalid history records"),
            ));
        }
    }
    Ok(())
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
