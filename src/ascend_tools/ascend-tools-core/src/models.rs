use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// The kind of runtime (workspace or deployment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    Workspace,
    Deployment,
}

impl RuntimeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Deployment => "deployment",
        }
    }
}

impl fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

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

/// A workspace or deployment. Use the [`Workspace`] or [`Deployment`] type aliases
/// for clarity when the kind is known.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    pub uuid: String,
    pub id: String,
    pub title: String,
    pub kind: RuntimeKind,
    pub project_uuid: String,
    pub environment_uuid: String,
    pub build_uuid: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub health: Option<String>,
    #[serde(default)]
    pub paused: bool,
    #[serde(default, alias = "profile_name")]
    pub profile: Option<String>,
    #[serde(default, alias = "base_git_branch")]
    pub git_branch_base: Option<String>,
    #[serde(default, alias = "working_git_branch")]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub enable_automations: Option<bool>,
    #[serde(default)]
    pub auto_snooze_timeout_minutes: Option<u32>,
}

/// A workspace runtime. Wraps [`Runtime`] with `kind == Workspace`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Workspace(pub Runtime);

impl Deref for Workspace {
    type Target = Runtime;
    fn deref(&self) -> &Runtime {
        &self.0
    }
}

impl From<Runtime> for Workspace {
    fn from(r: Runtime) -> Self {
        Self(r)
    }
}

/// A deployment runtime. Wraps [`Runtime`] with `kind == Deployment`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Deployment(pub Runtime);

impl Deref for Deployment {
    type Target = Runtime;
    fn deref(&self) -> &Runtime {
        &self.0
    }
}

impl From<Runtime> for Deployment {
    fn from(r: Runtime) -> Self {
        Self(r)
    }
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
    pub kind: Option<RuntimeKind>,
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
#[non_exhaustive]
pub struct RuntimeCreate {
    pub title: String,
    pub environment: String,
    pub project: String,
    #[serde(rename = "profile_name")]
    pub profile: String,
    #[serde(rename = "working_git_branch")]
    pub git_branch: String,
    #[serde(rename = "base_git_branch", skip_serializing_if = "Option::is_none")]
    pub git_branch_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_automations: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_snooze_timeout_minutes: Option<u32>,
}

impl RuntimeCreate {
    /// Create a new request with the required fields. Optional fields default to `None`.
    pub fn new(
        title: impl Into<String>,
        environment: impl Into<String>,
        project: impl Into<String>,
        profile: impl Into<String>,
        git_branch: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            environment: environment.into(),
            project: project.into(),
            profile: profile.into(),
            git_branch: git_branch.into(),
            git_branch_base: None,
            size: None,
            storage_size: None,
            enable_automations: None,
            auto_snooze_timeout_minutes: None,
        }
    }
}

/// Request body for updating a workspace or deployment (PATCH semantics — only set fields are sent).
#[derive(Debug, Clone, Default, Serialize)]
#[non_exhaustive]
pub struct RuntimeUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "working_git_branch", skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(rename = "base_git_branch", skip_serializing_if = "Option::is_none")]
    pub git_branch_base: Option<String>,
    #[serde(rename = "profile_name", skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_automations: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_snooze_timeout_minutes: Option<u32>,
}

impl RuntimeUpdate {
    /// Returns `true` if no fields are set (i.e., the update is a no-op).
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.git_branch.is_none()
            && self.git_branch_base.is_none()
            && self.profile.is_none()
            && self.size.is_none()
            && self.storage_size.is_none()
            && self.enable_automations.is_none()
            && self.auto_snooze_timeout_minutes.is_none()
    }
}

/// Request for Otto chat.
#[derive(Debug, Clone, Serialize)]
pub struct OttoChatRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_uuid: Option<String>,
    #[serde(skip)]
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
#[derive(Debug, Clone, Serialize)]
pub struct OttoChatResponse {
    pub message: String,
    pub thread_id: Option<String>,
}

/// A streaming event from Otto.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A text delta from Otto's response.
    TextDelta(String),
    /// A tool call has started.
    ToolCallStart {
        call_id: String,
        name: String,
        arguments: String,
    },
    /// A tool call has completed with output.
    ToolCallOutput { call_id: String, output: String },
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
