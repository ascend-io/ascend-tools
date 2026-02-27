use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRunTrigger {
    pub event_uuid: String,
    pub event_type: String,
}

/// Filters for listing runtimes.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct RuntimeFilters {
    pub id: Option<String>,
    pub kind: Option<String>,
    pub project_uuid: Option<String>,
    pub environment_uuid: Option<String>,
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
