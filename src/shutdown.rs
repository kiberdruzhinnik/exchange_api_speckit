use std::{future::Future, io, time::Duration};

/// Stop accepting connections after the signal future completes and allow the server
/// future to drain until `grace_period` elapses. When this returns at the deadline,
/// the caller should return from its runtime entry point so runtime teardown cancels
/// any remaining detached connection tasks.
pub async fn run_until_shutdown<S, G>(
    server: S,
    signal: G,
    grace_period: Duration,
) -> io::Result<()>
where
    S: Future<Output = io::Result<()>>,
    G: Future<Output = io::Result<()>>,
{
    tokio::pin!(server);
    tokio::pin!(signal);

    tokio::select! {
        result = &mut server => result,
        signal_result = &mut signal => {
            signal_result?;
            tracing::info!(grace_period_seconds = grace_period.as_secs(), "shutdown signal received; draining in-flight requests");

            match tokio::time::timeout(grace_period, &mut server).await {
                Ok(result) => result,
                Err(_) => {
                    tracing::warn!(grace_period_seconds = grace_period.as_secs(), "shutdown grace period expired; cancelling remaining server work");
                    Ok(())
                }
            }
        }
    }
}
