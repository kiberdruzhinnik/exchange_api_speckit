use crate::{
    Providers,
    cache_store::CacheStore,
    provider::{ExchangeProvider, ProviderError},
};
use chrono::Utc;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{
    sync::{Semaphore, watch},
    task::JoinSet,
};

const MAX_REFRESH_CONCURRENCY: usize = 4;

pub async fn run(
    store: CacheStore,
    providers: Providers,
    interval: Duration,
    retry_cap: Duration,
    mut shutdown: watch::Receiver<bool>,
) {
    let permits = Arc::new(Semaphore::new(MAX_REFRESH_CONCURRENCY));
    loop {
        let now = Utc::now().timestamp();
        let collections = match store.collections().await {
            Ok(rows) => rows,
            Err(error) => {
                tracing::error!(%error, "could not enumerate history collections for background refresh");
                tokio::select! {
                    _ = shutdown.changed() => return,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => continue,
                }
            }
        };
        let interval = interval.as_secs().min(i64::MAX as u64) as i64;
        let mut due = Vec::new();
        let mut next_delay = Duration::from_secs(31_536_000);
        for (provider, symbol, last, _, retry_at) in collections {
            let scheduled = last.map(|at| at.saturating_add(interval)).unwrap_or(now);
            let at = retry_at.unwrap_or(scheduled);
            if is_due(last, retry_at, interval, now) {
                due.push((provider, symbol));
            } else {
                next_delay = next_delay.min(Duration::from_secs((at - now) as u64));
            }
        }
        if due.is_empty() {
            let notifier = store.change_notifier();
            tokio::select! {
                _ = shutdown.changed() => return,
                _ = notifier.notified() => continue,
                _ = tokio::time::sleep(next_delay) => continue,
            }
        }
        let mut tasks = JoinSet::new();
        for (provider_name, symbol) in due {
            let Some(provider) = select_provider(&providers, &provider_name) else {
                tracing::warn!(%provider_name, %symbol, "unknown provider in history refresh store");
                continue;
            };
            let store = store.clone();
            let permits = permits.clone();
            let retry_cap = retry_cap.as_secs();
            let interval = interval as u64;
            tasks.spawn(async move {
                let Ok(_permit) = permits.acquire_owned().await else {
                    return;
                };
                refresh_one(store, provider, provider_name, symbol, interval, retry_cap).await;
            });
        }
        loop {
            tokio::select! {
                _ = shutdown.changed() => return,
                result = tasks.join_next() => if result.is_none() { break; },
            }
        }
    }
}

pub fn is_due(
    last_full_refresh_at: Option<i64>,
    next_attempt_at: Option<i64>,
    interval_secs: i64,
    now: i64,
) -> bool {
    let normal_due = last_full_refresh_at
        .map(|at| at.saturating_add(interval_secs))
        .unwrap_or(now);
    next_attempt_at.unwrap_or(normal_due) <= now
}

async fn refresh_one(
    store: CacheStore,
    provider: Arc<dyn ExchangeProvider>,
    namespace: String,
    symbol: String,
    interval: u64,
    cap: u64,
) {
    let result = provider
        .history(&symbol)
        .await
        .and_then(|records| validate(records));
    let now = Utc::now().timestamp();
    match result {
        Ok(records) => match store
            .merge_records(&namespace, &symbol, &records, true, now)
            .await
        {
            Ok(()) => {
                tracing::info!(provider=%namespace, %symbol, records=records.len(), "background history refresh succeeded")
            }
            Err(error) => {
                let _ = store
                    .record_failure(&namespace, &symbol, true, interval, cap, now)
                    .await;
                tracing::error!(provider=%namespace, %symbol, %error, "could not commit background history refresh")
            }
        },
        Err(error) => {
            let retryable = error.retryable();
            if let Err(store_error) = store
                .record_failure(&namespace, &symbol, retryable, interval, cap, now)
                .await
            {
                tracing::error!(provider=%namespace, %symbol, %store_error, "could not persist background refresh failure state");
            }
            tracing::warn!(provider=%namespace, %symbol, retryable, error=%error, "background history refresh failed; retained history is unchanged");
        }
    }
}

fn validate(
    records: Vec<crate::domain::DailyMarketRecord>,
) -> Result<Vec<crate::domain::DailyMarketRecord>, ProviderError> {
    let mut dates = HashSet::with_capacity(records.len());
    for record in &records {
        if !dates.insert(record.date) {
            return Err(ProviderError::InvalidData(format!(
                "duplicate history date {}",
                record.date
            )));
        }
        if [
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
            return Err(ProviderError::InvalidData(
                "history contains a non-finite numeric value".into(),
            ));
        }
    }
    Ok(records)
}

fn select_provider(providers: &Providers, name: &str) -> Option<Arc<dyn ExchangeProvider>> {
    match name {
        "moex" => Some(providers.moex.clone()),
        "spbex" => Some(providers.spbex.clone()),
        "cbr" => Some(providers.cbr.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_due, validate};
    #[test]
    fn honors_intervals_longer_than_seven_days_and_retry_override() {
        let interval = 14 * 24 * 60 * 60;
        let last = 1_000;
        assert!(!is_due(Some(last), None, interval, last + interval - 1));
        assert!(is_due(Some(last), None, interval, last + interval));
        assert!(is_due(Some(last), Some(last + 5), interval, last + 5));
        assert!(!is_due(Some(last), Some(last + 5), interval, last + 4));
    }

    #[test]
    fn rejects_identical_duplicate_records_as_invalid_data() {
        let row = crate::domain::DailyMarketRecord {
            date: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            close: Some(1.0),
            high: None,
            low: None,
            volume: None,
            facevalue: None,
        };
        assert!(matches!(
            validate(vec![row.clone(), row]),
            Err(crate::provider::ProviderError::InvalidData(_))
        ));
    }
}
