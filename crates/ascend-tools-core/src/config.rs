use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

/// Parsed config file: (default_instance_name, instances).
type ConfigFileData = (Option<String>, BTreeMap<String, InstanceEntry>);

use crate::error::{Error, Result};

const SA_ID_ENV: &str = "ASCEND_SERVICE_ACCOUNT_ID";
const SA_KEY_ENV: &str = "ASCEND_SERVICE_ACCOUNT_KEY";
const INSTANCE_API_URL_ENV: &str = "ASCEND_INSTANCE_API_URL";
const INSTANCE_ENV: &str = "ASCEND_INSTANCE";

const CONFIG_DIR: &str = ".ascend-tools";
const CONFIG_FILE: &str = "config.toml";

#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    pub(crate) service_account_id: String,
    pub(crate) service_account_key: String,
    pub(crate) instance_api_url: String,
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

/// A single instance entry from the config file.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct InstanceEntry {
    pub service_account_id: String,
    pub instance_api_url: String,
    pub service_account_key_env: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::with_overrides_and_instance(None, None, None, None)
    }

    pub fn with_overrides(
        service_account_id: Option<&str>,
        service_account_key: Option<&str>,
        instance_api_url: Option<&str>,
    ) -> Result<Self> {
        Self::with_overrides_and_instance(
            service_account_id,
            service_account_key,
            instance_api_url,
            None,
        )
    }

    pub fn with_overrides_and_instance(
        service_account_id: Option<&str>,
        service_account_key: Option<&str>,
        instance_api_url: Option<&str>,
        instance: Option<&str>,
    ) -> Result<Self> {
        // Determine instance name: explicit > ASCEND_INSTANCE env var
        let instance_env = env::var(INSTANCE_ENV).ok();
        let instance_name = instance.or(instance_env.as_deref());

        // Load instance config from file
        let entry = load_instance_entry(instance_name)?;

        // Resolve the key from the instance's key_env if available
        let inst_key = match &entry {
            Some((name, e)) => {
                let val = env::var(&e.service_account_key_env).ok();
                if val.as_deref().is_none_or(|v| v.is_empty()) {
                    // Only error if the user explicitly selected this instance
                    // or the config file exists (meaning they opted in)
                    if instance_name.is_some() {
                        return Err(Error::KeyEnvNotSet {
                            env_var: e.service_account_key_env.clone(),
                            instance: name.clone(),
                        });
                    }
                    None
                } else {
                    val
                }
            }
            None => None,
        };

        Ok(Self {
            service_account_id: resolve(
                "service_account_id",
                SA_ID_ENV,
                service_account_id,
                entry.as_ref().map(|(_, e)| e.service_account_id.as_str()),
                env::var(SA_ID_ENV).ok().as_deref(),
            )?,
            service_account_key: resolve(
                "service_account_key",
                SA_KEY_ENV,
                service_account_key,
                inst_key.as_deref(),
                env::var(SA_KEY_ENV).ok().as_deref(),
            )?,
            instance_api_url: resolve(
                "instance_api_url",
                INSTANCE_API_URL_ENV,
                instance_api_url,
                entry.as_ref().map(|(_, e)| e.instance_api_url.as_str()),
                env::var(INSTANCE_API_URL_ENV).ok().as_deref(),
            )?,
        })
    }
}

/// Load the appropriate instance entry from the config file.
///
/// - If `instance_name` is Some, load that specific instance (error if not found).
/// - If `instance_name` is None and a config file exists, load the default instance.
/// - If no config file exists, return None (backward compat with env vars only).
fn load_instance_entry(instance_name: Option<&str>) -> Result<Option<(String, InstanceEntry)>> {
    let (default_name, mut instances) = match load_config_file()? {
        Some(v) => v,
        None => {
            if let Some(name) = instance_name {
                return Err(Error::ConfigFileRead {
                    path: config_file_path().unwrap_or_default(),
                    reason: format!(
                        "instance '{name}' requested but no config file found, run `ascend-tools instance add`"
                    ),
                });
            }
            return Ok(None);
        }
    };

    let target = instance_name.unwrap_or_else(|| default_name.as_deref().unwrap_or("default"));

    match instances.remove_entry(target) {
        Some((name, entry)) => Ok(Some((name, entry))),
        None if instance_name.is_some() => {
            // User explicitly asked for an instance that doesn't exist
            let available: Vec<String> = instances.into_keys().collect();
            Err(Error::InstanceNotFound {
                name: target.to_string(),
                available,
            })
        }
        None => {
            // No explicit instance requested and default doesn't exist — fall through to env vars
            Ok(None)
        }
    }
}

fn resolve(
    name: &str,
    env_var: &str,
    cli_value: Option<&str>,
    instance_value: Option<&str>,
    env_value: Option<&str>,
) -> Result<String> {
    if let Some(v) = cli_value
        && !v.is_empty()
    {
        return Ok(v.to_string());
    }
    if let Some(v) = instance_value
        && !v.is_empty()
    {
        return Ok(v.to_string());
    }
    if let Some(v) = env_value
        && !v.is_empty()
    {
        return Ok(v.to_string());
    }
    Err(Error::MissingConfig {
        field: name.to_string(),
        env_var: env_var.to_string(),
        flag: name.replace('_', "-"),
    })
}

/// Returns the config file path: `~/.ascend-tools/config.toml`.
pub fn config_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(CONFIG_DIR).join(CONFIG_FILE))
}

/// Returns the config directory path: `~/.ascend-tools/`.
pub fn config_dir_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(CONFIG_DIR))
}

/// Load and parse the config file, returning (default_instance, instances).
fn load_config_file() -> Result<Option<ConfigFileData>> {
    load_config_file_from_path(config_file_path())
}

fn load_config_file_from_path(path: Option<PathBuf>) -> Result<Option<ConfigFileData>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(Error::ConfigFileRead {
                path,
                reason: e.to_string(),
            });
        }
    };
    parse_config_toml(&contents, &path)
}

fn parse_config_toml(contents: &str, path: &Path) -> Result<Option<ConfigFileData>> {
    let table: toml::Value = toml::from_str(contents).map_err(|e| Error::ConfigFileParse {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let toml::Value::Table(root) = table else {
        return Err(Error::ConfigFileParse {
            path: path.to_path_buf(),
            reason: "expected a TOML table at root".to_string(),
        });
    };

    let default_instance = root
        .get("default_instance")
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut instances = BTreeMap::new();
    for (key, value) in &root {
        if key == "default_instance" {
            continue;
        }
        if let toml::Value::Table(_) = value {
            let entry: InstanceEntry =
                value
                    .clone()
                    .try_into()
                    .map_err(|e: toml::de::Error| Error::ConfigFileParse {
                        path: path.to_path_buf(),
                        reason: format!("instance '{key}': {e}"),
                    })?;
            instances.insert(key.clone(), entry);
        }
    }

    if instances.is_empty() {
        return Ok(None);
    }

    Ok(Some((default_instance, instances)))
}

/// Validate an instance name: must be non-empty, alphanumeric/hyphens/underscores,
/// and not collide with the reserved `default_instance` key.
fn validate_instance_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidInstanceName {
            name: name.to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if name == "default_instance" {
        return Err(Error::InvalidInstanceName {
            name: name.to_string(),
            reason: "reserved name".to_string(),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::InvalidInstanceName {
            name: name.to_string(),
            reason: "must contain only alphanumeric characters, hyphens, or underscores"
                .to_string(),
        });
    }
    Ok(())
}

/// Public API for managing the instance config file.
pub mod instance_config {
    use super::*;

    /// Add or update an instance in the config file.
    pub fn add(name: &str, sa_id: &str, url: &str, key_env: &str) -> Result<()> {
        validate_instance_name(name)?;

        let path = config_file_path().ok_or_else(|| Error::ConfigFileWrite {
            path: PathBuf::from("~/.ascend-tools/config.toml"),
            reason: "could not determine home directory".to_string(),
        })?;

        // Ensure directory exists
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::ConfigFileWrite {
                path: path.clone(),
                reason: e.to_string(),
            })?;
        }

        // Load existing or create new document
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return Err(Error::ConfigFileRead {
                    path,
                    reason: e.to_string(),
                });
            }
        };

        let mut doc: toml_edit::DocumentMut =
            contents
                .parse()
                .map_err(|e: toml_edit::TomlError| Error::ConfigFileParse {
                    path: path.clone(),
                    reason: e.to_string(),
                })?;

        // Auto-set default_instance when no default exists yet
        if doc.get("default_instance").is_none() {
            doc["default_instance"] = toml_edit::value(name);
        }

        // Create or update the instance table
        let table = doc[name].or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
        table["service_account_id"] = toml_edit::value(sa_id);
        table["instance_api_url"] = toml_edit::value(url);
        table["service_account_key_env"] = toml_edit::value(key_env);

        std::fs::write(&path, doc.to_string()).map_err(|e| Error::ConfigFileWrite {
            path,
            reason: e.to_string(),
        })
    }

    /// Remove an instance from the config file.
    /// If the removed instance was the default, the `default_instance` key is also removed.
    pub fn remove(name: &str) -> Result<()> {
        let path = config_file_path().ok_or_else(|| Error::ConfigFileWrite {
            path: PathBuf::from("~/.ascend-tools/config.toml"),
            reason: "could not determine home directory".to_string(),
        })?;

        let contents = std::fs::read_to_string(&path).map_err(|e| Error::ConfigFileRead {
            path: path.clone(),
            reason: e.to_string(),
        })?;

        let mut doc: toml_edit::DocumentMut =
            contents
                .parse()
                .map_err(|e: toml_edit::TomlError| Error::ConfigFileParse {
                    path: path.clone(),
                    reason: e.to_string(),
                })?;

        if doc.remove(name).is_none() {
            let available: Vec<String> = doc
                .iter()
                .filter(|(k, v)| *k != "default_instance" && v.is_table())
                .map(|(k, _)| k.to_string())
                .collect();
            return Err(Error::InstanceNotFound {
                name: name.to_string(),
                available,
            });
        }

        // Clear default_instance if it pointed to the removed instance
        let is_default = doc
            .get("default_instance")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v == name);
        if is_default {
            doc.remove("default_instance");
        }

        std::fs::write(&path, doc.to_string()).map_err(|e| Error::ConfigFileWrite {
            path,
            reason: e.to_string(),
        })
    }

    /// List all configured instances. Returns (name, entry) pairs and the default instance name.
    pub fn list() -> Result<(String, Vec<(String, InstanceEntry)>)> {
        let (default_name, instances) = load_config_file()?.unwrap_or_default();
        let default = default_name.unwrap_or_else(|| "default".to_string());
        let entries: Vec<(String, InstanceEntry)> = instances.into_iter().collect();
        Ok((default, entries))
    }

    /// Set the default instance.
    pub fn set_default(name: &str) -> Result<()> {
        let path = config_file_path().ok_or_else(|| Error::ConfigFileWrite {
            path: PathBuf::from("~/.ascend-tools/config.toml"),
            reason: "could not determine home directory".to_string(),
        })?;

        let contents = std::fs::read_to_string(&path).map_err(|e| Error::ConfigFileRead {
            path: path.clone(),
            reason: e.to_string(),
        })?;

        let mut doc: toml_edit::DocumentMut =
            contents
                .parse()
                .map_err(|e: toml_edit::TomlError| Error::ConfigFileParse {
                    path: path.clone(),
                    reason: e.to_string(),
                })?;

        // Verify the instance exists
        if !doc.contains_key(name) || !doc[name].is_table() {
            let available: Vec<String> = doc
                .iter()
                .filter(|(k, v)| *k != "default_instance" && v.is_table())
                .map(|(k, _)| k.to_string())
                .collect();
            return Err(Error::InstanceNotFound {
                name: name.to_string(),
                available,
            });
        }

        doc["default_instance"] = toml_edit::value(name);

        std::fs::write(&path, doc.to_string()).map_err(|e| Error::ConfigFileWrite {
            path,
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: parse TOML content directly for testing
    fn parse_test_config(toml: &str) -> Result<Option<ConfigFileData>> {
        parse_config_toml(toml, &PathBuf::from("test.toml"))
    }

    #[test]
    fn test_resolve_with_cli_value() {
        let result = resolve("test", "TEST_VAR", Some("cli-value"), None, None);
        assert_eq!(result.unwrap(), "cli-value");
    }

    #[test]
    fn test_resolve_missing() {
        let result = resolve("test_field", "TEST_VAR", None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_cli_overrides_instance_and_env() {
        let result = resolve(
            "test",
            "TEST_VAR",
            Some("from-cli"),
            Some("from-instance"),
            Some("from-env"),
        );
        assert_eq!(result.unwrap(), "from-cli");
    }

    #[test]
    fn test_resolve_instance_overrides_env() {
        let result = resolve(
            "test",
            "TEST_VAR",
            None,
            Some("from-instance"),
            Some("from-env"),
        );
        assert_eq!(result.unwrap(), "from-instance");
    }

    #[test]
    fn test_resolve_falls_back_to_env() {
        let result = resolve("test", "TEST_VAR", None, None, Some("from-env"));
        assert_eq!(result.unwrap(), "from-env");
    }

    #[test]
    fn test_resolve_empty_cli_falls_back_to_instance() {
        let result = resolve("test", "TEST_VAR", Some(""), Some("from-instance"), None);
        assert_eq!(result.unwrap(), "from-instance");
    }

    #[test]
    fn test_resolve_empty_instance_falls_back_to_env() {
        let result = resolve("test", "TEST_VAR", None, Some(""), Some("from-env"));
        assert_eq!(result.unwrap(), "from-env");
    }

    #[test]
    fn test_resolve_empty_env_errors() {
        let result = resolve("test_field", "TEST_VAR", None, None, Some(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_error_message_format() {
        let err = resolve(
            "instance_api_url",
            "ASCEND_INSTANCE_API_URL",
            None,
            None,
            None,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("ASCEND_INSTANCE_API_URL"));
        assert!(err.contains("--instance-api-url"));
    }

    #[test]
    fn test_parse_config_basic() {
        let toml = r#"
[default]
service_account_id = "asc-sa-abc"
instance_api_url = "https://api.test.ascend.io"
service_account_key_env = "MY_KEY"
"#;
        let result = parse_test_config(toml).unwrap().unwrap();
        assert!(result.0.is_none()); // no default_instance key
        assert_eq!(result.1.len(), 1);
        let entry = &result.1["default"];
        assert_eq!(entry.service_account_id, "asc-sa-abc");
        assert_eq!(entry.instance_api_url, "https://api.test.ascend.io");
        assert_eq!(entry.service_account_key_env, "MY_KEY");
    }

    #[test]
    fn test_parse_config_multiple_instances() {
        let toml = r#"
default_instance = "staging"

[default]
service_account_id = "asc-sa-abc"
instance_api_url = "https://api.default.ascend.io"
service_account_key_env = "DEFAULT_KEY"

[staging]
service_account_id = "asc-sa-def"
instance_api_url = "https://api.staging.ascend.io"
service_account_key_env = "STAGING_KEY"
"#;
        let result = parse_test_config(toml).unwrap().unwrap();
        assert_eq!(result.0.as_deref(), Some("staging"));
        assert_eq!(result.1.len(), 2);
        assert!(result.1.contains_key("default"));
        assert!(result.1.contains_key("staging"));
    }

    #[test]
    fn test_parse_config_empty() {
        let toml = "";
        let result = parse_test_config(toml).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_config_invalid_toml() {
        let toml = "not valid toml {{{";
        let result = parse_test_config(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_config_missing_field() {
        let toml = r#"
[broken]
service_account_id = "asc-sa-abc"
"#;
        // Missing instance_api_url and service_account_key_env
        let result = parse_test_config(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_instance_name_valid() {
        assert!(validate_instance_name("default").is_ok());
        assert!(validate_instance_name("my-instance").is_ok());
        assert!(validate_instance_name("prod_01").is_ok());
        assert!(validate_instance_name("a").is_ok());
    }

    #[test]
    fn test_validate_instance_name_empty() {
        let err = validate_instance_name("").unwrap_err().to_string();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn test_validate_instance_name_reserved() {
        let err = validate_instance_name("default_instance")
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"));
    }

    #[test]
    fn test_validate_instance_name_invalid_chars() {
        let err = validate_instance_name("my instance")
            .unwrap_err()
            .to_string();
        assert!(err.contains("alphanumeric"));

        assert!(validate_instance_name("foo.bar").is_err());
        assert!(validate_instance_name("foo[0]").is_err());
    }
}
