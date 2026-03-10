use serde::{Deserialize, Serialize};

/// Runtime kind constants.
pub const KIND_WORKSPACE: &str = "workspace";
pub const KIND_DEPLOYMENT: &str = "deployment";

/// An Ascend environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub uuid: String,
    pub id: String,
    pub title: String,
}

/// An Ascend project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub uuid: String,
    pub id: String,
    pub title: String,
    pub path: Option<String>,
    pub repository_uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    pub uuid: String,
    pub id: String,
    pub title: String,
    pub kind: String,
    pub project_uuid: String,
    pub environment_uuid: String,
    pub build_uuid: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub health: Option<String>,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub profile_name: Option<String>,
    #[serde(default)]
    pub base_git_branch: Option<String>,
    #[serde(default)]
    pub working_git_branch: Option<String>,
    #[serde(default)]
    pub enable_automations: Option<bool>,
    #[serde(default)]
    pub auto_snooze_timeout_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRun {
    pub name: String,
    pub flow: String,
    pub build_uuid: String,
    pub runtime_uuid: String,
    pub status: String,
    pub created_at: String,
    pub error: Option<serde_json::Value>,
}

/// Wrapper returned by the list flow runs endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRunList {
    pub items: Vec<FlowRun>,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRunTrigger {
    pub event_uuid: String,
    pub event_type: String,
}

/// Filters for listing runtimes.
///
/// `project` and `environment` accept either a title or UUID — the backend resolves them.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct RuntimeFilters {
    pub id: Option<String>,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub project: Option<String>,
    pub environment: Option<String>,
}

/// Filters for listing flow runs.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct FlowRunFilters {
    pub status: Option<String>,
    pub flow: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

/// Request body for creating a workspace or deployment.
///
/// The `environment` and `project` fields accept either a title or UUID — the backend resolves them.
/// Kind is determined by the endpoint (`POST /workspaces` vs `POST /deployments`).
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeCreate {
    pub title: String,
    pub environment: String,
    pub project: String,
    pub profile_name: String,
    pub working_git_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_automations: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_snooze_timeout_minutes: Option<u32>,
}

/// Request body for updating a runtime (PATCH semantics — only set fields are sent).
#[derive(Debug, Clone, Default, Serialize)]
pub struct RuntimeUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_automations: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_snooze_timeout_minutes: Option<u32>,
}

/// Request for Otto chat.
#[derive(Debug, Clone, Serialize)]
pub struct OttoChatRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<OttoModel>,
}

/// Model specification for Otto — either a plain model name or a provider+model pair.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum OttoModel {
    /// Use default provider for this model name.
    Name(String),
    /// Use a specific provider and model.
    ProviderModel {
        provider_id: String,
        model_id: String,
    },
}

impl OttoModel {
    /// Build an `OttoModel` from optional provider and model strings.
    ///
    /// Returns `None` if `model` is `None`.
    pub fn from_options(provider: Option<&str>, model: Option<&str>) -> Option<Self> {
        match (provider, model) {
            (Some(p), Some(m)) => Some(Self::ProviderModel {
                provider_id: p.to_string(),
                model_id: m.to_string(),
            }),
            (None, Some(m)) => Some(Self::Name(m.to_string())),
            _ => None,
        }
    }
}

/// Response from Otto chat.
#[derive(Debug, Clone)]
pub struct OttoChatResponse {
    pub message: String,
    pub thread_id: Option<String>,
}

/// An Otto provider with its enabled models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OttoProvider {
    pub id: String,
    pub name: String,
    pub default_model: String,
    pub models: Vec<OttoProviderModel>,
}

/// A model available on an Otto provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OttoProviderModel {
    pub id: String,
    pub name: String,
}
