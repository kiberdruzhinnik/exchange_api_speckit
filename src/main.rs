use exchange_api::{
    AppState, cache_store::CacheStore, cbr::client::CbrClient, config::AppConfig, http::router,
    moex::client::MoexClient, shutdown::run_until_shutdown, spbex::client::SpbexClient,
};
use std::{future::IntoFuture, time::Duration};
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
    let spbex = SpbexClient::new(
        &config.spbex_api_base_url,
        config.upstream_timeout,
        config.spbex_max_response_bytes,
    )?;
    let cbr = CbrClient::new(
        &config.cbr_api_base_url,
        config.upstream_timeout,
        config.cbr_max_response_bytes,
    )?;
    let store = CacheStore::open(
        &config.history_cache_db_path,
        config.history_cache_max_bytes,
    )
    .await?;
    let app = router(AppState::with_all_services_and_store(
        moex,
        spbex,
        cbr,
        config.history_cache_ttl,
        config.history_cache_max_bytes,
        store,
    ));
    let listener = TcpListener::bind(config.listen_addr).await?;

    tracing::info!(address = %config.listen_addr, "starting exchange API");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .into_future();
    let signal = async move {
        tokio::signal::ctrl_c().await?;
        tracing::info!("received Ctrl+C (SIGINT)");
        let _ = shutdown_tx.send(());
        Ok::<(), std::io::Error>(())
    };

    run_until_shutdown(server, signal, Duration::from_secs(30)).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
