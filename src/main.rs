use exchange_api::{AppState, config::AppConfig, http::router, moex::client::MoexClient};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let config = AppConfig::from_env().map_err(std::io::Error::other)?;
    let moex = MoexClient::with_limits(
        &config.moex_iss_base_url,
        config.upstream_timeout,
        config.moex_max_response_bytes,
        config.moex_max_history_bytes,
    )?;
    let app = router(AppState::with_cache(
        moex,
        config.history_cache_ttl,
        config.history_cache_max_bytes,
    ));
    let listener = TcpListener::bind(config.listen_addr).await?;

    tracing::info!(address = %config.listen_addr, "starting exchange API");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
