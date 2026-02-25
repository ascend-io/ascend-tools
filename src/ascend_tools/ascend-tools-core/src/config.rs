use anyhow::{Result, bail};
use std::env;

const SA_ID_ENV_VARS: &[&str] = &["ASCEND_SERVICE_ACCOUNT_ID"];
const SA_KEY_ENV_VARS: &[&str] = &["ASCEND_SERVICE_ACCOUNT_KEY"];
const INSTANCE_API_URL_ENV_VARS: &[&str] = &["ASCEND_INSTANCE_API_URL"];

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
            service_account_id: resolve_required("service_account_id", SA_ID_ENV_VARS, None)?,
            service_account_key: resolve_required("service_account_key", SA_KEY_ENV_VARS, None)?,
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
        instance_api_url: Option<&str>,
    ) -> Result<Self> {
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
            instance_api_url: resolve_required(
                "instance_api_url",
                INSTANCE_API_URL_ENV_VARS,
                instance_api_url,
            )?,
        })
    }
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
}
