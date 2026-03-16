use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -- Workspace params --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListWorkspacesParams {
    /// Filter by workspace title
    pub title: Option<String>,
    /// Filter by project name (or UUID)
    pub project: Option<String>,
    /// Filter by environment name (or UUID)
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWorkspaceParams {
    /// Workspace title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorkspaceParams {
    /// Workspace title
    pub title: String,
    /// Environment name (or UUID)
    pub environment: String,
    /// Project name (or UUID)
    pub project: String,
    /// Configuration profile
    pub profile: String,
    /// Git branch to use
    pub git_branch: String,
    /// Base git branch (optional)
    pub git_branch_base: Option<String>,
    /// Size (e.g. "Small", "Medium", "Large")
    pub size: Option<String>,
    /// Storage size in GB
    pub storage_size: Option<u32>,
    /// Minutes of inactivity before auto-snooze
    pub auto_snooze_timeout_minutes: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateWorkspaceParams {
    /// Workspace title (for lookup)
    pub current_title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
    /// New title
    pub title: Option<String>,
    /// Switch to a different git branch
    pub git_branch: Option<String>,
    /// New base git branch
    pub git_branch_base: Option<String>,
    /// New profile
    pub profile: Option<String>,
    /// New size
    pub size: Option<String>,
    /// New storage size in GB
    pub storage_size: Option<u32>,
    /// New auto-snooze timeout in minutes
    pub auto_snooze_timeout_minutes: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PauseWorkspaceParams {
    /// Workspace title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResumeWorkspaceParams {
    /// Workspace title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteWorkspaceParams {
    /// Workspace title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

// -- Deployment params --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDeploymentsParams {
    /// Filter by deployment title
    pub title: Option<String>,
    /// Filter by project name (or UUID)
    pub project: Option<String>,
    /// Filter by environment name (or UUID)
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDeploymentParams {
    /// Deployment title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateDeploymentParams {
    /// Deployment title
    pub title: String,
    /// Environment name (or UUID)
    pub environment: String,
    /// Project name (or UUID)
    pub project: String,
    /// Configuration profile
    pub profile: String,
    /// Git branch to use
    pub git_branch: String,
    /// Base git branch (optional)
    pub git_branch_base: Option<String>,
    /// Size (e.g. "Small", "Medium", "Large")
    pub size: Option<String>,
    /// Storage size in GB
    pub storage_size: Option<u32>,
    /// Enable automations (default: true for deployments)
    pub enable_automations: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateDeploymentParams {
    /// Deployment title (for lookup)
    pub current_title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
    /// New title
    pub title: Option<String>,
    /// Switch to a different git branch
    pub git_branch: Option<String>,
    /// New base git branch
    pub git_branch_base: Option<String>,
    /// New profile
    pub profile: Option<String>,
    /// New size
    pub size: Option<String>,
    /// New storage size in GB
    pub storage_size: Option<u32>,
    /// Enable or disable automations
    pub enable_automations: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PauseDeploymentAutomationsParams {
    /// Deployment title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResumeDeploymentAutomationsParams {
    /// Deployment title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteDeploymentParams {
    /// Deployment title
    pub title: String,
    /// Use UUID instead of title (optional override)
    pub uuid: Option<String>,
}

// -- Environment params --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListEnvironmentsParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEnvironmentParams {
    /// Environment title
    pub title: String,
}

// -- Project params --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListProjectsParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetProjectParams {
    /// Project title
    pub title: String,
}

// -- Profile params --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListProfilesParams {
    /// Workspace title (mutually exclusive with deployment/uuid)
    pub workspace: Option<String>,
    /// Deployment title (mutually exclusive with workspace/uuid)
    pub deployment: Option<String>,
    /// UUID (direct override, bypasses title lookup)
    pub uuid: Option<String>,
    /// Project name (or UUID) — use with branch
    pub project: Option<String>,
    /// Git branch (required with project)
    pub branch: Option<String>,
}

// -- Flow params --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFlowsParams {
    /// Workspace title (mutually exclusive with deployment/uuid)
    pub workspace: Option<String>,
    /// Deployment title (mutually exclusive with workspace/uuid)
    pub deployment: Option<String>,
    /// UUID (direct override, bypasses title lookup)
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunFlowParams {
    /// Flow name
    pub flow: String,
    /// Workspace title (mutually exclusive with deployment/uuid)
    pub workspace: Option<String>,
    /// Deployment title (mutually exclusive with workspace/uuid)
    pub deployment: Option<String>,
    /// UUID (direct override, bypasses title lookup)
    pub uuid: Option<String>,
    /// Flow run options. All fields are optional — omit spec entirely to run with defaults.
    pub spec: Option<FlowRunSpec>,
    /// Resume the workspace/deployment if paused before submitting the flow run
    pub resume: Option<bool>,
}

/// Options for a flow run. All fields are optional.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FlowRunSpec {
    /// List of component names to run. If omitted, all components in the flow are run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<String>>,
    /// List of component categories to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_categories: Option<Vec<String>>,
    /// If true, drop all internal data and metadata tables/views and recompute from scratch.
    /// WARNING: This is a destructive operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_refresh: Option<bool>,
    /// Whether to run tests after processing data. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_tests: Option<bool>,
    /// Whether to store test results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_test_results: Option<bool>,
    /// Whether to halt the flow on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halt_flow_on_error: Option<bool>,
    /// Whether to disable optimizers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_optimizers: Option<bool>,
    /// Whether to update component materialization types.
    /// WARNING: If materialization type changes are detected, existing data will be dropped and recomputed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_materialization_type: Option<bool>,
    /// Whether to use deep data pruning for Smart Table component data maintenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_data_pruning: Option<bool>,
    /// Whether to backfill block statistics for existing data blocks without statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backfill_missing_statistics: Option<bool>,
    /// Whether to disable collection of incremental read/transform component metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_incremental_metadata_collection: Option<bool>,
    /// Custom parameters dictionary passed to the flow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Runner configuration overrides for this flow run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner_overrides: Option<serde_json::Value>,
    /// Capture any additional fields for forward compatibility.
    #[serde(flatten)]
    #[schemars(skip)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFlowRunsParams {
    /// Workspace title (mutually exclusive with deployment/uuid)
    pub workspace: Option<String>,
    /// Deployment title (mutually exclusive with workspace/uuid)
    pub deployment: Option<String>,
    /// UUID (direct override, bypasses title lookup)
    pub uuid: Option<String>,
    /// Filter by status (e.g. "running", "succeeded", "failed")
    pub status: Option<String>,
    /// Filter by flow name
    pub flow: Option<String>,
    /// Filter by start time (ISO 8601)
    pub since: Option<String>,
    /// Filter by end time (ISO 8601)
    pub until: Option<String>,
    /// Pagination offset
    pub offset: Option<u64>,
    /// Pagination limit
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetFlowRunParams {
    /// Flow run name
    pub name: String,
    /// Workspace title (mutually exclusive with deployment/uuid)
    pub workspace: Option<String>,
    /// Deployment title (mutually exclusive with workspace/uuid)
    pub deployment: Option<String>,
    /// UUID (direct override, bypasses title lookup)
    pub uuid: Option<String>,
}

// -- Otto params --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OttoParams {
    /// Message to send to Otto
    pub prompt: String,
    /// Workspace title for context (mutually exclusive with deployment/uuid)
    pub workspace: Option<String>,
    /// Deployment title for context (mutually exclusive with workspace/uuid)
    pub deployment: Option<String>,
    /// UUID (direct override, bypasses title lookup)
    pub uuid: Option<String>,
    /// LLM provider (e.g. "openai")
    pub provider: Option<String>,
    /// LLM model (e.g. "gpt-4o")
    pub model: Option<String>,
    /// Thread ID to continue a conversation
    pub thread_id: Option<String>,
}
