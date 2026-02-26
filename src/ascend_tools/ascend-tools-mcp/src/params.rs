use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRuntimesParams {
    /// Filter by runtime ID
    pub id: Option<String>,
    /// Filter by runtime kind
    pub kind: Option<String>,
    /// Filter by project UUID
    pub project_uuid: Option<String>,
    /// Filter by environment UUID
    pub environment_uuid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRuntimeParams {
    /// Runtime UUID
    pub uuid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFlowsParams {
    /// Runtime UUID
    pub runtime_uuid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResumeRuntimeParams {
    /// Runtime UUID
    pub runtime_uuid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PauseRuntimeParams {
    /// Runtime UUID
    pub runtime_uuid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunFlowParams {
    /// Runtime UUID
    pub runtime_uuid: String,
    /// Flow name
    pub flow_name: String,
    /// Flow run options. All fields are optional — omit spec entirely to run with defaults.
    pub spec: Option<FlowRunSpec>,
    /// Resume the runtime if paused before submitting the flow run.
    pub resume: Option<bool>,
}

/// Options for a flow run. All fields are optional.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FlowRunSpec {
    /// List of component names to run. If omitted, all components in the flow are run.
    pub components: Option<Vec<String>>,
    /// List of component categories to run.
    pub component_categories: Option<Vec<String>>,
    /// If true, drop all internal data and metadata tables/views and recompute from scratch.
    /// WARNING: This is a destructive operation.
    pub full_refresh: Option<bool>,
    /// Whether to run tests after processing data. Defaults to true.
    pub run_tests: Option<bool>,
    /// Whether to store test results.
    pub store_test_results: Option<bool>,
    /// Whether to halt the flow on error.
    pub halt_flow_on_error: Option<bool>,
    /// Whether to disable optimizers.
    pub disable_optimizers: Option<bool>,
    /// Whether to update component materialization types (e.g. between simple, view, incremental, smart).
    /// WARNING: If materialization type changes are detected, existing data will be dropped and recomputed.
    pub update_materialization_type: Option<bool>,
    /// Whether to use deep data pruning for Smart Table component data maintenance (slower but full table scan).
    pub deep_data_pruning: Option<bool>,
    /// Whether to backfill block statistics for existing data blocks without statistics.
    pub backfill_missing_statistics: Option<bool>,
    /// Whether to disable collection of incremental read/transform component metadata.
    pub disable_incremental_metadata_collection: Option<bool>,
    /// Custom parameters dictionary passed to the flow.
    pub parameters: Option<serde_json::Value>,
    /// Runner configuration overrides for this flow run (e.g. {"size": "Medium"} or {"size": {"cpu": "8", "memory": "32Gi"}}).
    pub runner_overrides: Option<serde_json::Value>,
    /// Capture any additional fields for forward compatibility.
    #[serde(flatten)]
    #[schemars(skip)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFlowRunsParams {
    /// Runtime UUID
    pub runtime_uuid: String,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by flow name
    pub flow_name: Option<String>,
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
    /// Runtime UUID
    pub runtime_uuid: String,
    /// Flow run name
    pub name: String,
}
