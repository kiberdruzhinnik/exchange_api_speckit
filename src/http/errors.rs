use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("symbol is malformed or unsupported")]
    InvalidSymbol,
    #[error("MOEX market data is temporarily unavailable")]
    Upstream(#[source] anyhow::Error),
    #[error("history store is temporarily unavailable")]
    Store(String),
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match self {
            Self::InvalidSymbol => (
                StatusCode::BAD_REQUEST,
                "invalid_symbol",
                "Symbol is malformed or unsupported".to_owned(),
            ),
            Self::Upstream(error) => {
                tracing::warn!(error = %error, "MOEX ISS request failed");
                (
                    StatusCode::BAD_GATEWAY,
                    "moex_unavailable",
                    "MOEX market data is temporarily unavailable".to_owned(),
                )
            }
            Self::Store(error) => {
                tracing::error!(error = %error, "history cache store failed");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "history_store_unavailable",
                    "Ticker history is temporarily unavailable".to_owned(),
                )
            }
        };

        let body = ErrorResponse {
            error: ErrorDetail { code, message },
        };
        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Upstream(error)
    }
}
