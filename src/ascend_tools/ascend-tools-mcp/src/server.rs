use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use ascend_tools::client::AscendClient;
use ascend_tools::models::{
    FlowRunFilters, RuntimeCreate, RuntimeFilters, RuntimeKind, RuntimeUpdate,
};

use crate::params::{
    CreateDeploymentParams, CreateWorkspaceParams, DeleteDeploymentParams, DeleteWorkspaceParams,
    GetDeploymentParams, GetEnvironmentParams, GetFlowRunParams, GetProjectParams,
    GetWorkspaceParams, ListDeploymentsParams, ListEnvironmentsParams, ListFlowRunsParams,
    ListFlowsParams, ListProfilesParams, ListProjectsParams, ListWorkspacesParams,
    PauseDeploymentAutomationsParams, PauseWorkspaceParams, ResumeDeploymentAutomationsParams,
    ResumeWorkspaceParams, RunFlowParams, UpdateDeploymentParams, UpdateWorkspaceParams,
};

/// Run a blocking SDK call on a spawn_blocking task and serialize the result as JSON.
async fn blocking<T: serde::Serialize + Send + 'static>(
    client: &Arc<AscendClient>,
    f: impl FnOnce(&AscendClient) -> ascend_tools::Result<T> + Send + 'static,
) -> Result<CallToolResult, McpError> {
    let client = client.clone();
    let result = tokio::task::spawn_blocking(move || f(&client))
        .await
        .map_err(|e| McpError::internal_error(format!("task join error: {e}"), None))?
        .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;
    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::internal_error(format!("JSON serialization error: {e}"), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Resolve target UUID from workspace_title/deployment_title/uuid params.
fn resolve_flow_target(
    client: &AscendClient,
    workspace: Option<String>,
    deployment: Option<String>,
    uuid: Option<String>,
) -> ascend_tools::Result<String> {
    client.resolve_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref())
}

#[derive(Clone)]
pub struct AscendMcpServer {
    client: Option<Arc<AscendClient>>,
    client_init_error: Option<String>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AscendMcpServer {
    pub fn new(client: AscendClient) -> Self {
        Self {
            client: Some(Arc::new(client)),
            client_init_error: None,
            tool_router: Self::tool_router(),
        }
    }

    pub fn with_client_init_error(error: impl Into<String>) -> Self {
        Self {
            client: None,
            client_init_error: Some(error.into()),
            tool_router: Self::tool_router(),
        }
    }

    fn client(&self) -> Result<&Arc<AscendClient>, McpError> {
        self.client.as_ref().ok_or_else(|| {
            let detail = self
                .client_init_error
                .as_deref()
                .unwrap_or("unknown initialization error");
            McpError::internal_error(
                format!(
                    "Ascend client is not configured: {detail}. Set ASCEND_SERVICE_ACCOUNT_ID, ASCEND_SERVICE_ACCOUNT_KEY, and ASCEND_INSTANCE_API_URL in the MCP server environment."
                ),
                None,
            )
        })
    }

    // -- Workspace tools --

    #[tool(description = "List workspaces, optionally filtered by title, project, or environment")]
    async fn list_workspaces(
        &self,
        Parameters(params): Parameters<ListWorkspacesParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut filters = RuntimeFilters::default();
            filters.title = params.title;
            filters.project = params.project;
            filters.environment = params.environment;
            c.list_workspaces(filters)
        })
        .await
    }

    #[tool(description = "Get a workspace by title (or UUID)")]
    async fn get_workspace(
        &self,
        Parameters(params): Parameters<GetWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.get_workspace(&params.title, params.uuid.as_deref())
        })
        .await
    }

    #[tool(description = "Create a workspace")]
    async fn create_workspace(
        &self,
        Parameters(params): Parameters<CreateWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut create = RuntimeCreate::new(
                params.title,
                params.environment,
                params.project,
                params.profile,
                params.git_branch,
            );
            create.git_branch_base = params.git_branch_base;
            create.size = params.size;
            create.storage_size = params.storage_size;
            create.auto_snooze_timeout_minutes = params.auto_snooze_timeout_minutes;
            c.create_workspace(&create)
        })
        .await
    }

    #[tool(description = "Update a workspace (only provided fields are changed)")]
    async fn update_workspace(
        &self,
        Parameters(params): Parameters<UpdateWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut update = RuntimeUpdate::default();
            update.title = params.title;
            update.git_branch = params.git_branch;
            update.git_branch_base = params.git_branch_base;
            update.profile = params.profile;
            update.size = params.size;
            update.storage_size = params.storage_size;
            update.auto_snooze_timeout_minutes = params.auto_snooze_timeout_minutes;
            c.update_workspace(&params.current_title, params.uuid.as_deref(), &update)
        })
        .await
    }

    #[tool(description = "Pause a workspace")]
    async fn pause_workspace(
        &self,
        Parameters(params): Parameters<PauseWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.pause_workspace(&params.title, params.uuid.as_deref())
        })
        .await
    }

    #[tool(description = "Resume a paused workspace")]
    async fn resume_workspace(
        &self,
        Parameters(params): Parameters<ResumeWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.resume_workspace(&params.title, params.uuid.as_deref())
        })
        .await
    }

    #[tool(description = "Delete a workspace")]
    async fn delete_workspace(
        &self,
        Parameters(params): Parameters<DeleteWorkspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.delete_workspace(&params.title, params.uuid.as_deref())
        })
        .await
        .map(|_| CallToolResult::success(vec![Content::text("Workspace deleted")]))
    }

    // -- Deployment tools --

    #[tool(description = "List deployments, optionally filtered by title, project, or environment")]
    async fn list_deployments(
        &self,
        Parameters(params): Parameters<ListDeploymentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut filters = RuntimeFilters::default();
            filters.title = params.title;
            filters.project = params.project;
            filters.environment = params.environment;
            c.list_deployments(filters)
        })
        .await
    }

    #[tool(description = "Get a deployment by title (or UUID)")]
    async fn get_deployment(
        &self,
        Parameters(params): Parameters<GetDeploymentParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.get_deployment(&params.title, params.uuid.as_deref())
        })
        .await
    }

    #[tool(description = "Create a deployment")]
    async fn create_deployment(
        &self,
        Parameters(params): Parameters<CreateDeploymentParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut create = RuntimeCreate::new(
                params.title,
                params.environment,
                params.project,
                params.profile,
                params.git_branch,
            );
            create.git_branch_base = params.git_branch_base;
            create.size = params.size;
            create.storage_size = params.storage_size;
            create.enable_automations = params.enable_automations;
            c.create_deployment(&create)
        })
        .await
    }

    #[tool(description = "Update a deployment (only provided fields are changed)")]
    async fn update_deployment(
        &self,
        Parameters(params): Parameters<UpdateDeploymentParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut update = RuntimeUpdate::default();
            update.title = params.title;
            update.git_branch = params.git_branch;
            update.git_branch_base = params.git_branch_base;
            update.profile = params.profile;
            update.size = params.size;
            update.storage_size = params.storage_size;
            update.enable_automations = params.enable_automations;
            c.update_deployment(&params.current_title, params.uuid.as_deref(), &update)
        })
        .await
    }

    #[tool(description = "Pause automations on a deployment")]
    async fn pause_deployment_automations(
        &self,
        Parameters(params): Parameters<PauseDeploymentAutomationsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.pause_deployment_automations(&params.title, params.uuid.as_deref())
        })
        .await
    }

    #[tool(description = "Resume automations on a deployment")]
    async fn resume_deployment_automations(
        &self,
        Parameters(params): Parameters<ResumeDeploymentAutomationsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.resume_deployment_automations(&params.title, params.uuid.as_deref())
        })
        .await
    }

    #[tool(description = "Delete a deployment")]
    async fn delete_deployment(
        &self,
        Parameters(params): Parameters<DeleteDeploymentParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            c.delete_deployment(&params.title, params.uuid.as_deref())
        })
        .await
        .map(|_| CallToolResult::success(vec![Content::text("Deployment deleted")]))
    }

    // -- Environment tools --

    #[tool(description = "List environments")]
    async fn list_environments(
        &self,
        Parameters(_params): Parameters<ListEnvironmentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, |c| c.list_environments()).await
    }

    #[tool(description = "Get an environment by title")]
    async fn get_environment(
        &self,
        Parameters(params): Parameters<GetEnvironmentParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| c.get_environment(&params.title)).await
    }

    // -- Project tools --

    #[tool(description = "List projects")]
    async fn list_projects(
        &self,
        Parameters(_params): Parameters<ListProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, |c| c.list_projects()).await
    }

    #[tool(description = "Get a project by title")]
    async fn get_project(
        &self,
        Parameters(params): Parameters<GetProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| c.get_project(&params.title)).await
    }

    // -- Profile tools --

    #[tool(description = "List available profiles for a workspace, deployment, or project+branch")]
    async fn list_profiles(
        &self,
        Parameters(params): Parameters<ListProfilesParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let (runtime_uuid, project, branch) = if let Some(uuid) = params.uuid {
                (Some(uuid), None, None)
            } else if let Some(ws) = params.workspace {
                let rt = c.resolve_runtime_by_title(&ws, RuntimeKind::Workspace)?;
                (Some(rt.uuid), None, None)
            } else if let Some(dep) = params.deployment {
                let rt = c.resolve_runtime_by_title(&dep, RuntimeKind::Deployment)?;
                (Some(rt.uuid), None, None)
            } else if let Some(proj) = params.project {
                (None, Some(proj), params.branch)
            } else {
                return Err(ascend_tools::Error::MissingField {
                    context: "list_profiles",
                    field: "workspace, deployment, project, or uuid",
                });
            };
            c.list_profiles(
                runtime_uuid.as_deref(),
                project.as_deref(),
                branch.as_deref(),
            )
        })
        .await
    }

    // -- Flow tools --

    #[tool(description = "List flows in a workspace or deployment")]
    async fn list_flows(
        &self,
        Parameters(params): Parameters<ListFlowsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let uuid = resolve_flow_target(c, params.workspace, params.deployment, params.uuid)?;
            c.list_flows(&uuid)
        })
        .await
    }

    #[tool(
        description = "Trigger a flow run (checks health first; use resume=true to resume a paused workspace/deployment)"
    )]
    async fn run_flow(
        &self,
        Parameters(params): Parameters<RunFlowParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        let spec = params
            .spec
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| McpError::internal_error(format!("invalid spec: {e}"), None))?;
        let resume = params.resume.unwrap_or(false);
        let flow = params.flow;
        blocking(client, move |c| {
            let uuid = resolve_flow_target(c, params.workspace, params.deployment, params.uuid)?;
            c.run_flow(&uuid, &flow, spec, resume)
        })
        .await
    }

    #[tool(
        description = "List flow runs in a workspace or deployment, optionally filtered by status or flow name"
    )]
    async fn list_flow_runs(
        &self,
        Parameters(params): Parameters<ListFlowRunsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let uuid = resolve_flow_target(c, params.workspace, params.deployment, params.uuid)?;
            let mut filters = FlowRunFilters::default();
            filters.status = params.status;
            filters.flow = params.flow;
            filters.since = params.since;
            filters.until = params.until;
            filters.offset = params.offset;
            filters.limit = params.limit;
            c.list_flow_runs(&uuid, filters)
        })
        .await
    }

    #[tool(description = "Get details of a specific flow run by name")]
    async fn get_flow_run(
        &self,
        Parameters(params): Parameters<GetFlowRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let uuid = resolve_flow_target(c, params.workspace, params.deployment, params.uuid)?;
            c.get_flow_run(&uuid, &params.name)
        })
        .await
    }
}

#[tool_handler]
impl ServerHandler for AscendMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Ascend MCP server. Provides tools to manage workspaces, deployments, and flows."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use ascend_tools::{client::AscendClient, config::Config};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use mockito::Server;
    use rmcp::handler::server::wrapper::Parameters;

    use super::*;
    use crate::params::{GetWorkspaceParams, ListDeploymentsParams, ListWorkspacesParams};

    fn test_server(server: &Server) -> AscendMcpServer {
        let key = URL_SAFE_NO_PAD.encode([7u8; 32]);
        let config =
            Config::with_overrides(Some("asc-sa-test"), Some(&key), Some(server.url().as_str()))
                .unwrap();
        let client = AscendClient::new(config).unwrap();
        AscendMcpServer::new(client)
    }

    fn mock_auth(server: &mut Server) {
        server
            .mock("GET", "/api/v1/auth/config")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"cloud_api_domain":"api.cloud.ascend.io"}"#)
            .expect(1)
            .create();

        server
            .mock("POST", "/api/v1/auth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"mcp-token","expiration":4102444800}"#)
            .expect(1)
            .create();
    }

    fn runtime_json(kind: &str) -> serde_json::Value {
        serde_json::json!({
            "uuid": "rt-1",
            "id": "runtime-1",
            "title": "My Runtime",
            "kind": kind,
            "project_uuid": "p-1",
            "environment_uuid": "e-1",
            "build_uuid": null,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "health": "running",
            "paused": false,
            "profile_name": "default",
            "working_git_branch": "main"
        })
    }

    fn tool_result_json(result: CallToolResult) -> serde_json::Value {
        let text = serde_json::to_value(result).unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        serde_json::from_str(&text).unwrap()
    }

    #[tokio::test]
    async fn workspace_tools_succeed() {
        let mut server = Server::new_async().await;
        mock_auth(&mut server);

        let list_mock = server
            .mock("GET", "/api/v1/runtimes")
            .match_query(mockito::Matcher::UrlEncoded(
                "kind".into(),
                "workspace".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::json!([runtime_json("workspace")]).to_string())
            .expect(1)
            .create();

        // For get_workspace: first resolves title via list, then gets by UUID
        let resolve_mock = server
            .mock("GET", "/api/v1/runtimes")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("title".into(), "My Runtime".into()),
                mockito::Matcher::UrlEncoded("kind".into(), "workspace".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::json!([runtime_json("workspace")]).to_string())
            .expect(1)
            .create();

        let mcp = test_server(&server);

        let workspaces = mcp
            .list_workspaces(Parameters(ListWorkspacesParams {
                title: None,
                project: None,
                environment: None,
            }))
            .await
            .unwrap();
        assert!(tool_result_json(workspaces).is_array());

        // get_workspace resolves by title via list (no separate GET by UUID)
        let workspace = mcp
            .get_workspace(Parameters(GetWorkspaceParams {
                title: "My Runtime".to_string(),
                uuid: None,
            }))
            .await
            .unwrap();
        assert_eq!(tool_result_json(workspace)["uuid"], "rt-1");

        list_mock.assert();
        resolve_mock.assert();
    }

    #[tokio::test]
    async fn deployment_list_filters_by_kind() {
        let mut server = Server::new_async().await;
        mock_auth(&mut server);

        let mock = server
            .mock("GET", "/api/v1/runtimes")
            .match_query(mockito::Matcher::UrlEncoded(
                "kind".into(),
                "deployment".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::json!([runtime_json("deployment")]).to_string())
            .expect(1)
            .create();

        let mcp = test_server(&server);

        let deployments = mcp
            .list_deployments(Parameters(ListDeploymentsParams {
                title: None,
                project: None,
                environment: None,
            }))
            .await
            .unwrap();
        assert!(tool_result_json(deployments).is_array());
        mock.assert();
    }

    #[tokio::test]
    async fn all_tools_fail_when_client_is_unconfigured() {
        let mcp = AscendMcpServer::with_client_init_error("missing env vars");

        let err = mcp
            .list_workspaces(Parameters(ListWorkspacesParams {
                title: None,
                project: None,
                environment: None,
            }))
            .await
            .unwrap_err()
            .to_string();

        assert!(err.contains("Ascend client is not configured"));
        assert!(err.contains("ASCEND_SERVICE_ACCOUNT_ID"));
    }
}
