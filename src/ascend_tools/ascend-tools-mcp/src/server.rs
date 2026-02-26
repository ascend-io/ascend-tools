use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use ascend_tools::client::AscendClient;
use ascend_tools::models::{FlowRunFilters, RuntimeFilters};

use crate::params::{
    GetFlowRunParams, GetRuntimeParams, ListFlowRunsParams, ListFlowsParams, ListRuntimesParams,
    PauseRuntimeParams, ResumeRuntimeParams, RunFlowParams,
};

fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| McpError::internal_error(format!("JSON serialization error: {e}"), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

#[derive(Clone)]
pub struct AscendMcpServer {
    client: Arc<AscendClient>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AscendMcpServer {
    pub fn new(client: AscendClient) -> Self {
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List Ascend runtimes, optionally filtered by id, kind, project, or environment"
    )]
    async fn list_runtimes(
        &self,
        Parameters(params): Parameters<ListRuntimesParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || {
            client.list_runtimes(RuntimeFilters {
                id: params.id,
                kind: params.kind,
                project_uuid: params.project_uuid,
                environment_uuid: params.environment_uuid,
            })
        })
        .await
        .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
        .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }

    #[tool(description = "Get details of a specific Ascend runtime by UUID")]
    async fn get_runtime(
        &self,
        Parameters(params): Parameters<GetRuntimeParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || client.get_runtime(&params.uuid))
            .await
            .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
            .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }

    #[tool(description = "Resume a paused Ascend runtime")]
    async fn resume_runtime(
        &self,
        Parameters(params): Parameters<ResumeRuntimeParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let result =
            tokio::task::spawn_blocking(move || client.resume_runtime(&params.runtime_uuid))
                .await
                .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
                .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }

    #[tool(description = "Pause a running Ascend runtime")]
    async fn pause_runtime(
        &self,
        Parameters(params): Parameters<PauseRuntimeParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let result =
            tokio::task::spawn_blocking(move || client.pause_runtime(&params.runtime_uuid))
                .await
                .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
                .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }

    #[tool(description = "List flows in an Ascend runtime")]
    async fn list_flows(
        &self,
        Parameters(params): Parameters<ListFlowsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || client.list_flows(&params.runtime_uuid))
            .await
            .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
            .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }

    #[tool(
        description = "Trigger a flow run in an Ascend runtime. Checks runtime health first; use resume=true to resume a paused runtime before running."
    )]
    async fn run_flow(
        &self,
        Parameters(params): Parameters<RunFlowParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let spec = params
            .spec
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| McpError::internal_error(format!("invalid spec: {e}"), None))?;
        let resume = params.resume.unwrap_or(false);
        let result = tokio::task::spawn_blocking(move || {
            client.run_flow(&params.runtime_uuid, &params.flow_name, spec, resume)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
        .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }

    #[tool(
        description = "List flow runs in an Ascend runtime, optionally filtered by status or flow name"
    )]
    async fn list_flow_runs(
        &self,
        Parameters(params): Parameters<ListFlowRunsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || {
            client.list_flow_runs(
                &params.runtime_uuid,
                FlowRunFilters {
                    status: params.status,
                    flow: params.flow_name,
                    since: params.since,
                    until: params.until,
                    offset: params.offset,
                    limit: params.limit,
                },
            )
        })
        .await
        .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
        .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }

    #[tool(description = "Get details of a specific flow run by name")]
    async fn get_flow_run(
        &self,
        Parameters(params): Parameters<GetFlowRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || {
            client.get_flow_run(&params.runtime_uuid, &params.name)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
        .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        json_result(&result)
    }
}

#[tool_handler]
impl ServerHandler for AscendMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Ascend MCP server. Provides tools to manage runtimes, flows, and flow runs."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
