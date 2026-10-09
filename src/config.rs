use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub listen_addr: SocketAddr,
    pub upstream_timeout: Duration,
    pub moex_iss_base_url: String,
    pub spbex_api_base_url: String,
    pub cbr_api_base_url: String,
    pub history_cache_ttl: Duration,
    pub history_cache_max_bytes: u64,
    pub moex_max_response_bytes: usize,
    pub moex_max_history_bytes: usize,
    pub spbex_max_response_bytes: usize,
    pub cbr_max_response_bytes: usize,
    pub history_cache_db_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|key| env::var_os(key))
    }

    fn from_lookup(
        mut get: impl FnMut(&str) -> Option<std::ffi::OsString>,
    ) -> Result<Self, String> {
        let listen_addr = get("EXCHANGE_API_LISTEN_ADDR")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
            .parse::<SocketAddr>()
            .map_err(|error| {
                format!("EXCHANGE_API_LISTEN_ADDR must be a socket address: {error}")
            })?;

        let timeout_seconds = get("EXCHANGE_API_REQUEST_TIMEOUT_SECS")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| "15".to_owned())
            .parse::<u64>()
            .map_err(|error| {
                format!("EXCHANGE_API_REQUEST_TIMEOUT_SECS must be a positive integer: {error}")
            })?;
        if timeout_seconds == 0 {
            return Err("EXCHANGE_API_REQUEST_TIMEOUT_SECS must be greater than zero".to_owned());
        }

        let moex_iss_base_url = get("MOEX_ISS_BASE_URL")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| "https://iss.moex.com/iss/".to_owned());

        let spbex_api_base_url = get("SPBEX_API_BASE_URL")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| "https://spbexchange.ru/api/".to_owned());

        let cbr_api_base_url = get("CBR_API_BASE_URL")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| "https://www.cbr.ru/".to_owned());

        let cbr_max_response_bytes = get("CBR_MAX_RESPONSE_BYTES")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| (16 * 1024 * 1024usize).to_string())
            .parse::<usize>()
            .map_err(|error| {
                format!("CBR_MAX_RESPONSE_BYTES must be a positive integer: {error}")
            })?;
        if cbr_max_response_bytes == 0 {
            return Err("CBR_MAX_RESPONSE_BYTES must be greater than zero".to_owned());
        }

        let spbex_max_response_bytes = get("SPBEX_MAX_RESPONSE_BYTES")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| (16 * 1024 * 1024usize).to_string())
            .parse::<usize>()
            .map_err(|error| {
                format!("SPBEX_MAX_RESPONSE_BYTES must be a positive integer: {error}")
            })?;
        if spbex_max_response_bytes == 0 {
            return Err("SPBEX_MAX_RESPONSE_BYTES must be greater than zero".to_owned());
        }

        let cache_ttl_seconds = get("EXCHANGE_API_HISTORY_CACHE_TTL_SECS")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| "60".to_owned())
            .parse::<u64>()
            .map_err(|error| {
                format!("EXCHANGE_API_HISTORY_CACHE_TTL_SECS must be a positive integer: {error}")
            })?;
        if cache_ttl_seconds == 0 {
            return Err("EXCHANGE_API_HISTORY_CACHE_TTL_SECS must be greater than zero".to_owned());
        }

        let history_cache_max_bytes = get("EXCHANGE_API_HISTORY_CACHE_MAX_BYTES")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| (64 * 1024 * 1024u64).to_string())
            .parse::<u64>()
            .map_err(|error| {
                format!("EXCHANGE_API_HISTORY_CACHE_MAX_BYTES must be a positive integer: {error}")
            })?;
        if history_cache_max_bytes == 0 {
            return Err(
                "EXCHANGE_API_HISTORY_CACHE_MAX_BYTES must be greater than zero".to_owned(),
            );
        }

        let moex_max_response_bytes = get("MOEX_MAX_ISS_RESPONSE_BYTES")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| (4 * 1024 * 1024usize).to_string())
            .parse::<usize>()
            .map_err(|error| {
                format!("MOEX_MAX_ISS_RESPONSE_BYTES must be a positive integer: {error}")
            })?;
        if moex_max_response_bytes == 0 {
            return Err("MOEX_MAX_ISS_RESPONSE_BYTES must be greater than zero".to_owned());
        }

        let moex_max_history_bytes = get("MOEX_MAX_HISTORY_BYTES")
            .and_then(|value| value.into_string().ok())
            .ok_or(std::env::VarError::NotPresent)
            .unwrap_or_else(|_| (64 * 1024 * 1024usize).to_string())
            .parse::<usize>()
            .map_err(|error| {
                format!("MOEX_MAX_HISTORY_BYTES must be a positive integer: {error}")
            })?;
        if moex_max_history_bytes == 0 {
            return Err("MOEX_MAX_HISTORY_BYTES must be greater than zero".to_owned());
        }

        let history_cache_db_path = get("EXCHANGE_API_HISTORY_CACHE_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/exchange-api/history.sqlite3"));

        Ok(Self {
            listen_addr,
            upstream_timeout: Duration::from_secs(timeout_seconds),
            moex_iss_base_url,
            spbex_api_base_url,
            cbr_api_base_url,
            history_cache_ttl: Duration::from_secs(cache_ttl_seconds),
            history_cache_max_bytes,
            moex_max_response_bytes,
            moex_max_history_bytes,
            spbex_max_response_bytes,
            cbr_max_response_bytes,
            history_cache_db_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use std::{collections::HashMap, ffi::OsString, path::PathBuf, time::Duration};

    #[test]
    fn shared_and_provider_settings_have_common_defaults() {
        let config = AppConfig::from_lookup(|_| None).unwrap();
        assert_eq!(config.listen_addr.to_string(), "0.0.0.0:8080");
        assert_eq!(config.upstream_timeout, Duration::from_secs(15));
        assert_eq!(config.history_cache_ttl, Duration::from_secs(60));
        assert_eq!(config.history_cache_max_bytes, 64 * 1024 * 1024);
        assert_eq!(
            config.history_cache_db_path,
            PathBuf::from("/var/lib/exchange-api/history.sqlite3")
        );
        assert!(config.moex_iss_base_url.contains("iss.moex.com"));
        assert!(config.spbex_api_base_url.contains("spbexchange.ru"));
        assert!(config.cbr_api_base_url.contains("cbr.ru"));
    }

    #[test]
    fn canonical_settings_override_defaults_and_legacy_names_are_ignored() {
        let values = HashMap::from([
            ("EXCHANGE_API_LISTEN_ADDR", "127.0.0.1:9090"),
            ("EXCHANGE_API_REQUEST_TIMEOUT_SECS", "7"),
            ("EXCHANGE_API_HISTORY_CACHE_TTL_SECS", "9"),
            ("EXCHANGE_API_HISTORY_CACHE_MAX_BYTES", "1234"),
            ("EXCHANGE_API_HISTORY_CACHE_DB_PATH", "/tmp/cache.sqlite"),
            ("MOEX_ISS_BASE_URL", "https://iss.example/"),
            ("SPBEX_API_BASE_URL", "https://spb.example/api/"),
            ("CBR_API_BASE_URL", "https://cbr.example/"),
            ("MOEX_MAX_ISS_RESPONSE_BYTES", "2048"),
            ("MOEX_MAX_HISTORY_BYTES", "4096"),
            ("SPBEX_MAX_RESPONSE_BYTES", "8192"),
            ("CBR_MAX_RESPONSE_BYTES", "16384"),
            ("LISTEN_ADDR", "127.0.0.1:1111"),
            ("MOEX_REQUEST_TIMEOUT_SECS", "1"),
            ("MOEX_HISTORY_CACHE_TTL_SECS", "1"),
            ("MOEX_HISTORY_CACHE_MAX_BYTES", "1"),
            ("MOEX_HISTORY_CACHE_DB_PATH", "/tmp/legacy.sqlite"),
        ]);
        let config = AppConfig::from_lookup(|key| values.get(key).map(OsString::from)).unwrap();
        assert_eq!(config.listen_addr.to_string(), "127.0.0.1:9090");
        assert_eq!(config.upstream_timeout, Duration::from_secs(7));
        assert_eq!(config.history_cache_ttl, Duration::from_secs(9));
        assert_eq!(config.history_cache_max_bytes, 1234);
        assert_eq!(
            config.history_cache_db_path,
            PathBuf::from("/tmp/cache.sqlite")
        );
        assert_eq!(config.moex_iss_base_url, "https://iss.example/");
        assert_eq!(config.spbex_api_base_url, "https://spb.example/api/");
        assert_eq!(config.cbr_api_base_url, "https://cbr.example/");
        assert_eq!(config.moex_max_response_bytes, 2048);
        assert_eq!(config.moex_max_history_bytes, 4096);
        assert_eq!(config.spbex_max_response_bytes, 8192);
        assert_eq!(config.cbr_max_response_bytes, 16384);
    }
}
