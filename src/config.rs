use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub listen_addr: SocketAddr,
    pub upstream_timeout: Duration,
    pub moex_iss_base_url: String,
    pub history_cache_ttl: Duration,
    pub history_cache_max_bytes: u64,
    pub moex_max_response_bytes: usize,
    pub moex_max_history_bytes: usize,
    pub history_cache_db_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let listen_addr = env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
            .parse::<SocketAddr>()
            .map_err(|error| format!("LISTEN_ADDR must be a socket address: {error}"))?;

        let timeout_seconds = env::var("MOEX_REQUEST_TIMEOUT_SECS")
            .unwrap_or_else(|_| "15".to_owned())
            .parse::<u64>()
            .map_err(|error| {
                format!("MOEX_REQUEST_TIMEOUT_SECS must be a positive integer: {error}")
            })?;
        if timeout_seconds == 0 {
            return Err("MOEX_REQUEST_TIMEOUT_SECS must be greater than zero".to_owned());
        }

        let moex_iss_base_url = env::var("MOEX_ISS_BASE_URL")
            .unwrap_or_else(|_| "https://iss.moex.com/iss/".to_owned());

        let cache_ttl_seconds = env::var("MOEX_HISTORY_CACHE_TTL_SECS")
            .unwrap_or_else(|_| "60".to_owned())
            .parse::<u64>()
            .map_err(|error| {
                format!("MOEX_HISTORY_CACHE_TTL_SECS must be a positive integer: {error}")
            })?;
        if cache_ttl_seconds == 0 {
            return Err("MOEX_HISTORY_CACHE_TTL_SECS must be greater than zero".to_owned());
        }

        let history_cache_max_bytes = env::var("MOEX_HISTORY_CACHE_MAX_BYTES")
            .unwrap_or_else(|_| (64 * 1024 * 1024u64).to_string())
            .parse::<u64>()
            .map_err(|error| {
                format!("MOEX_HISTORY_CACHE_MAX_BYTES must be a positive integer: {error}")
            })?;
        if history_cache_max_bytes == 0 {
            return Err("MOEX_HISTORY_CACHE_MAX_BYTES must be greater than zero".to_owned());
        }

        let moex_max_response_bytes = env::var("MOEX_MAX_ISS_RESPONSE_BYTES")
            .unwrap_or_else(|_| (4 * 1024 * 1024usize).to_string())
            .parse::<usize>()
            .map_err(|error| {
                format!("MOEX_MAX_ISS_RESPONSE_BYTES must be a positive integer: {error}")
            })?;
        if moex_max_response_bytes == 0 {
            return Err("MOEX_MAX_ISS_RESPONSE_BYTES must be greater than zero".to_owned());
        }

        let moex_max_history_bytes = env::var("MOEX_MAX_HISTORY_BYTES")
            .unwrap_or_else(|_| (64 * 1024 * 1024usize).to_string())
            .parse::<usize>()
            .map_err(|error| {
                format!("MOEX_MAX_HISTORY_BYTES must be a positive integer: {error}")
            })?;
        if moex_max_history_bytes == 0 {
            return Err("MOEX_MAX_HISTORY_BYTES must be greater than zero".to_owned());
        }

        let history_cache_db_path = env::var_os("MOEX_HISTORY_CACHE_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/exchange-api/history.sqlite3"));

        Ok(Self {
            listen_addr,
            upstream_timeout: Duration::from_secs(timeout_seconds),
            moex_iss_base_url,
            history_cache_ttl: Duration::from_secs(cache_ttl_seconds),
            history_cache_max_bytes,
            moex_max_response_bytes,
            moex_max_history_bytes,
            history_cache_db_path,
        })
    }
}
