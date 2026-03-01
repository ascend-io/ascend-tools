use std::env;

use crate::error::{Error, Result};

const SA_ID_ENV: &str = "ASCEND_SERVICE_ACCOUNT_ID";
const SA_KEY_ENV: &str = "ASCEND_SERVICE_ACCOUNT_KEY";
const INSTANCE_API_URL_ENV: &str = "ASCEND_INSTANCE_API_URL";

#[derive(Clone)]
pub struct Config {
    pub service_account_id: String,
    pub service_account_key: String,
    pub instance_api_url: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("service_account_id", &self.service_account_id)
            .field("service_account_key", &"[REDACTED]")
            .field("instance_api_url", &self.instance_api_url)
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            service_account_id: resolve_required("service_account_id", SA_ID_ENV, None)?,
            service_account_key: resolve_required("service_account_key", SA_KEY_ENV, None)?,
            instance_api_url: resolve_required("instance_api_url", INSTANCE_API_URL_ENV, None)?,
        })
    }

    pub fn with_overrides(
        service_account_id: Option<&str>,
        service_account_key: Option<&str>,
        instance_api_url: Option<&str>,
    ) -> Result<Self> {
        Ok(Self {
            service_account_id: resolve_required(
                "service_account_id",
                SA_ID_ENV,
                service_account_id,
            )?,
            service_account_key: resolve_required(
                "service_account_key",
                SA_KEY_ENV,
                service_account_key,
            )?,
            instance_api_url: resolve_required(
                "instance_api_url",
                INSTANCE_API_URL_ENV,
                instance_api_url,
            )?,
        })
    }
}

fn resolve_required(name: &str, env_var: &str, cli_value: Option<&str>) -> Result<String> {
    resolve(name, env_var, cli_value, env::var(env_var).ok().as_deref())
}

fn resolve(
    name: &str,
    env_var: &str,
    cli_value: Option<&str>,
    env_value: Option<&str>,
) -> Result<String> {
    if let Some(v) = cli_value {
        if !v.is_empty() {
            return Ok(v.to_string());
        }
    }
    if let Some(v) = env_value {
        if !v.is_empty() {
            return Ok(v.to_string());
        }
    }
    Err(Error::MissingConfig {
        field: name.to_string(),
        env_var: env_var.to_string(),
        flag: name.replace('_', "-"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_with_cli_value() {
        let result = resolve("test", "TEST_VAR", Some("cli-value"), None);
        assert_eq!(result.unwrap(), "cli-value");
    }

    #[test]
    fn test_resolve_missing() {
        let result = resolve("test_field", "TEST_VAR", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_cli_overrides_env() {
        let result = resolve("test", "TEST_VAR", Some("from-cli"), Some("from-env"));
        assert_eq!(result.unwrap(), "from-cli");
    }

    #[test]
    fn test_resolve_falls_back_to_env() {
        let result = resolve("test", "TEST_VAR", None, Some("from-env"));
        assert_eq!(result.unwrap(), "from-env");
    }

    #[test]
    fn test_resolve_empty_cli_falls_back_to_env() {
        let result = resolve("test", "TEST_VAR", Some(""), Some("from-env"));
        assert_eq!(result.unwrap(), "from-env");
    }

    #[test]
    fn test_resolve_empty_env_errors() {
        let result = resolve("test_field", "TEST_VAR", None, Some(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_error_message_format() {
        let err = resolve("instance_api_url", "ASCEND_INSTANCE_API_URL", None, None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("ASCEND_INSTANCE_API_URL"));
        assert!(err.contains("--instance-api-url"));
    }
}
