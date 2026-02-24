use anyhow::{Result, bail};
use std::env;

const SA_ID_ENV_VARS: &[&str] = &["ASCEND_SERVICE_ACCOUNT_ID"];
const SA_KEY_ENV_VARS: &[&str] = &["ASCEND_SERVICE_ACCOUNT_KEY", "ASCEND_PRIVATE_KEY"];
const CLOUD_API_URL_ENV_VARS: &[&str] = &["ASCEND_CLOUD_API_URL"];
const CLOUD_API_DOMAIN_ENV_VARS: &[&str] = &["ASCEND_CLOUD_API_DOMAIN"];
const INSTANCE_API_URL_ENV_VARS: &[&str] = &["ASCEND_INSTANCE_API_URL"];

const DEFAULT_CLOUD_API_URL: &str = "https://api.ascend.io";

#[derive(Debug, Clone)]
pub struct Config {
    pub service_account_id: String,
    pub service_account_key: String,
    pub cloud_api_url: String,
    /// Domain used for JWT audience. Defaults to the host from cloud_api_url.
    /// Override via ASCEND_CLOUD_API_DOMAIN for local dev where the proxy domain
    /// differs from the Cloud API's internal CLOUD_API_DOMAIN.
    pub cloud_api_domain: String,
    pub instance_api_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let cloud_api_url = resolve_optional(CLOUD_API_URL_ENV_VARS, None)
            .unwrap_or_else(|| DEFAULT_CLOUD_API_URL.to_string());
        let cloud_api_domain = resolve_optional(CLOUD_API_DOMAIN_ENV_VARS, None)
            .unwrap_or_else(|| domain_from_url(&cloud_api_url));
        Ok(Self {
            service_account_id: resolve_required("service_account_id", SA_ID_ENV_VARS, None)?,
            service_account_key: resolve_required("service_account_key", SA_KEY_ENV_VARS, None)?,
            cloud_api_url,
            cloud_api_domain,
            instance_api_url: resolve_required(
                "instance_api_url",
                INSTANCE_API_URL_ENV_VARS,
                None,
            )?,
        })
    }

    pub fn with_overrides(
        service_account_id: Option<&str>,
        service_account_key: Option<&str>,
        cloud_api_url: Option<&str>,
        instance_api_url: Option<&str>,
    ) -> Result<Self> {
        let cloud_api_url = resolve_optional(CLOUD_API_URL_ENV_VARS, cloud_api_url)
            .unwrap_or_else(|| DEFAULT_CLOUD_API_URL.to_string());
        let cloud_api_domain = resolve_optional(CLOUD_API_DOMAIN_ENV_VARS, None)
            .unwrap_or_else(|| domain_from_url(&cloud_api_url));
        Ok(Self {
            service_account_id: resolve_required(
                "service_account_id",
                SA_ID_ENV_VARS,
                service_account_id,
            )?,
            service_account_key: resolve_required(
                "service_account_key",
                SA_KEY_ENV_VARS,
                service_account_key,
            )?,
            cloud_api_url,
            cloud_api_domain,
            instance_api_url: resolve_required(
                "instance_api_url",
                INSTANCE_API_URL_ENV_VARS,
                instance_api_url,
            )?,
        })
    }

    /// Extract the instance API host (without scheme) for token exchange.
    pub fn instance_api_host(&self) -> String {
        domain_from_url(&self.instance_api_url)
    }
}

/// Extract the host from a URL.
fn domain_from_url(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

fn resolve_required(name: &str, env_vars: &[&str], cli_value: Option<&str>) -> Result<String> {
    if let Some(v) = cli_value {
        if !v.is_empty() {
            return Ok(v.to_string());
        }
    }
    for var in env_vars {
        if let Ok(v) = env::var(var) {
            if !v.is_empty() {
                return Ok(v);
            }
        }
    }
    bail!(
        "{name} is required. Set {} or pass --{}",
        env_vars[0],
        name.replace('_', "-")
    )
}

fn resolve_optional(env_vars: &[&str], cli_value: Option<&str>) -> Option<String> {
    if let Some(v) = cli_value {
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    for var in env_vars {
        if let Ok(v) = env::var(var) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_required_with_cli_value() {
        let result = resolve_required("test", &["NONEXISTENT_VAR"], Some("cli-value"));
        assert_eq!(result.unwrap(), "cli-value");
    }

    #[test]
    fn test_resolve_required_missing() {
        let result = resolve_required("test_field", &["NONEXISTENT_VAR_12345"], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_optional_none() {
        let result = resolve_optional(&["NONEXISTENT_VAR_12345"], None);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_optional_with_cli() {
        let result = resolve_optional(&["NONEXISTENT_VAR_12345"], Some("value"));
        assert_eq!(result.unwrap(), "value");
    }

    #[test]
    fn test_instance_api_host() {
        let config = Config {
            service_account_id: "test".into(),
            service_account_key: "test".into(),
            cloud_api_url: "https://api.ascend.io".into(),
            cloud_api_domain: "api.ascend.io".into(),
            instance_api_url: "https://api.instance.ascend.io".into(),
        };
        assert_eq!(config.instance_api_host(), "api.instance.ascend.io");
    }
}
