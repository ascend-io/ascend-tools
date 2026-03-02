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

    #[tool(
        description = "List Ascend runtimes, optionally filtered by id, kind, project, or environment"
    )]
    async fn list_runtimes(
        &self,
        Parameters(params): Parameters<ListRuntimesParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut filters = RuntimeFilters::default();
            filters.id = params.id;
            filters.kind = params.kind;
            filters.project_uuid = params.project_uuid;
            filters.environment_uuid = params.environment_uuid;
            c.list_runtimes(filters)
        })
        .await
    }

    #[tool(description = "Get details of a specific Ascend runtime by UUID")]
    async fn get_runtime(
        &self,
        Parameters(params): Parameters<GetRuntimeParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| c.get_runtime(&params.uuid)).await
    }

    #[tool(description = "Resume a paused Ascend runtime")]
    async fn resume_runtime(
        &self,
        Parameters(params): Parameters<ResumeRuntimeParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| c.resume_runtime(&params.runtime_uuid)).await
    }

    #[tool(description = "Pause a running Ascend runtime")]
    async fn pause_runtime(
        &self,
        Parameters(params): Parameters<PauseRuntimeParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| c.pause_runtime(&params.runtime_uuid)).await
    }

    #[tool(description = "List flows in an Ascend runtime")]
    async fn list_flows(
        &self,
        Parameters(params): Parameters<ListFlowsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| c.list_flows(&params.runtime_uuid)).await
    }

    #[tool(
        description = "Trigger a flow run in an Ascend runtime. Checks runtime health first; use resume=true to resume a paused runtime before running."
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
        blocking(client, move |c| {
            c.run_flow(&params.runtime_uuid, &params.flow_name, spec, resume)
        })
        .await
    }

    #[tool(
        description = "List flow runs in an Ascend runtime, optionally filtered by status or flow name"
    )]
    async fn list_flow_runs(
        &self,
        Parameters(params): Parameters<ListFlowRunsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.client()?;
        blocking(client, move |c| {
            let mut filters = FlowRunFilters::default();
            filters.status = params.status;
            filters.flow = params.flow_name;
            filters.since = params.since;
            filters.until = params.until;
            filters.offset = params.offset;
            filters.limit = params.limit;
            c.list_flow_runs(&params.runtime_uuid, filters)
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
            c.get_flow_run(&params.runtime_uuid, &params.name)
        })
        .await
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

#[cfg(test)]
mod tests {
    use ascend_tools::{client::AscendClient, config::Config};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use mockito::Server;
    use rmcp::handler::server::wrapper::Parameters;

    use super::*;
    use crate::params::{
        GetFlowRunParams, GetRuntimeParams, ListFlowRunsParams, ListFlowsParams,
        ListRuntimesParams, PauseRuntimeParams, ResumeRuntimeParams, RunFlowParams,
    };

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

    fn tool_result_json(result: CallToolResult) -> serde_json::Value {
        let text = serde_json::to_value(result).unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        serde_json::from_str(&text).unwrap()
    }

    #[tokio::test]
    async fn all_tools_succeed_with_expected_json_shapes() {
        let mut server = Server::new_async().await;
        mock_auth(&mut server);

        let list_runtimes = server
            .mock("GET", "/api/v1/runtimes")
            .match_header("authorization", "Bearer mcp-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!([{
                    "uuid": "rt-1",
                    "id": "runtime-1",
                    "title": "Runtime 1",
                    "kind": "deployment",
                    "project_uuid": "p-1",
                    "environment_uuid": "e-1",
                    "build_uuid": null,
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                    "health": "running",
                    "paused": false
                }])
                .to_string(),
            )
            .expect(1)
            .create();

        let get_runtime = server
            .mock("GET", "/api/v1/runtimes/rt-1")
            .match_header("authorization", "Bearer mcp-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "uuid": "rt-1",
                    "id": "runtime-1",
                    "title": "Runtime 1",
                    "kind": "deployment",
                    "project_uuid": "p-1",
                    "environment_uuid": "e-1",
                    "build_uuid": null,
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                    "health": "running",
                    "paused": false
                })
                .to_string(),
            )
            .expect(2)
            .create();

        let resume_runtime = server
            .mock("POST", "/api/v1/runtimes/rt-1:resume")
            .match_header("authorization", "Bearer mcp-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "uuid": "rt-1",
                    "id": "runtime-1",
                    "title": "Runtime 1",
                    "kind": "deployment",
                    "project_uuid": "p-1",
                    "environment_uuid": "e-1",
                    "build_uuid": null,
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                    "health": "running",
                    "paused": false
                })
                .to_string(),
            )
            .expect(1)
            .create();

        let pause_runtime = server
            .mock("POST", "/api/v1/runtimes/rt-1:pause")
            .match_header("authorization", "Bearer mcp-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "uuid": "rt-1",
                    "id": "runtime-1",
                    "title": "Runtime 1",
                    "kind": "deployment",
                    "project_uuid": "p-1",
                    "environment_uuid": "e-1",
                    "build_uuid": null,
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                    "health": "running",
                    "paused": true
                })
                .to_string(),
            )
            .expect(1)
            .create();

        let list_flows = server
            .mock("GET", "/api/v1/runtimes/rt-1/flows")
            .match_header("authorization", "Bearer mcp-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"name":"sales"}]"#)
            .expect(1)
            .create();

        let run_flow = server
            .mock("POST", "/api/v1/runtimes/rt-1/flows/sales:run")
            .match_header("authorization", "Bearer mcp-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"event_uuid":"ev-1","event_type":"flow_run_requested"}"#)
            .expect(1)
            .create();

        let list_flow_runs = server
            .mock("GET", "/api/v1/flow-runs")
            .match_header("authorization", "Bearer mcp-token")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("runtime_uuid".into(), "rt-1".into()),
                mockito::Matcher::UrlEncoded("status".into(), "running".into()),
                mockito::Matcher::UrlEncoded("flow".into(), "sales".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "items": [{
                        "name": "fr-1",
                        "flow": "sales",
                        "build_uuid": "b-1",
                        "runtime_uuid": "rt-1",
                        "status": "running",
                        "created_at": "2026-01-01T00:00:00Z",
                        "error": null
                    }],
                    "truncated": false
                })
                .to_string(),
            )
            .expect(1)
            .create();

        let get_flow_run = server
            .mock("GET", "/api/v1/flow-runs/fr-1")
            .match_header("authorization", "Bearer mcp-token")
            .match_query(mockito::Matcher::UrlEncoded(
                "runtime_uuid".into(),
                "rt-1".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "name": "fr-1",
                    "flow": "sales",
                    "build_uuid": "b-1",
                    "runtime_uuid": "rt-1",
                    "status": "running",
                    "created_at": "2026-01-01T00:00:00Z",
                    "error": null
                })
                .to_string(),
            )
            .expect(1)
            .create();

        let mcp = test_server(&server);

        let runtimes = mcp
            .list_runtimes(Parameters(ListRuntimesParams {
                id: None,
                kind: None,
                project_uuid: None,
                environment_uuid: None,
            }))
            .await
            .unwrap();
        assert!(tool_result_json(runtimes).is_array());

        let runtime = mcp
            .get_runtime(Parameters(GetRuntimeParams {
                uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(tool_result_json(runtime)["uuid"], "rt-1");

        let resumed = mcp
            .resume_runtime(Parameters(ResumeRuntimeParams {
                runtime_uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(tool_result_json(resumed)["paused"], false);

        let paused = mcp
            .pause_runtime(Parameters(PauseRuntimeParams {
                runtime_uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(tool_result_json(paused)["paused"], true);

        let flows = mcp
            .list_flows(Parameters(ListFlowsParams {
                runtime_uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(tool_result_json(flows)[0]["name"], "sales");

        let trigger = mcp
            .run_flow(Parameters(RunFlowParams {
                runtime_uuid: "rt-1".to_string(),
                flow_name: "sales".to_string(),
                spec: None,
                resume: None,
            }))
            .await
            .unwrap();
        assert_eq!(tool_result_json(trigger)["event_uuid"], "ev-1");

        let runs = mcp
            .list_flow_runs(Parameters(ListFlowRunsParams {
                runtime_uuid: "rt-1".to_string(),
                status: Some("running".to_string()),
                flow_name: Some("sales".to_string()),
                since: None,
                until: None,
                offset: None,
                limit: None,
            }))
            .await
            .unwrap();
        let runs_json = tool_result_json(runs);
        assert_eq!(runs_json["items"][0]["name"], "fr-1");
        assert_eq!(runs_json["truncated"], false);

        let run = mcp
            .get_flow_run(Parameters(GetFlowRunParams {
                runtime_uuid: "rt-1".to_string(),
                name: "fr-1".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(tool_result_json(run)["status"], "running");

        list_runtimes.assert();
        get_runtime.assert();
        resume_runtime.assert();
        pause_runtime.assert();
        list_flows.assert();
        run_flow.assert();
        list_flow_runs.assert();
        get_flow_run.assert();
    }

    #[tokio::test]
    async fn all_tools_fail_when_client_is_unconfigured() {
        let mcp = AscendMcpServer::with_client_init_error("missing env vars");

        let mut errors = Vec::new();

        errors.push(
            mcp.list_runtimes(Parameters(ListRuntimesParams {
                id: None,
                kind: None,
                project_uuid: None,
                environment_uuid: None,
            }))
            .await
            .unwrap_err()
            .to_string(),
        );
        errors.push(
            mcp.get_runtime(Parameters(GetRuntimeParams {
                uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap_err()
            .to_string(),
        );
        errors.push(
            mcp.resume_runtime(Parameters(ResumeRuntimeParams {
                runtime_uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap_err()
            .to_string(),
        );
        errors.push(
            mcp.pause_runtime(Parameters(PauseRuntimeParams {
                runtime_uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap_err()
            .to_string(),
        );
        errors.push(
            mcp.list_flows(Parameters(ListFlowsParams {
                runtime_uuid: "rt-1".to_string(),
            }))
            .await
            .unwrap_err()
            .to_string(),
        );
        errors.push(
            mcp.run_flow(Parameters(RunFlowParams {
                runtime_uuid: "rt-1".to_string(),
                flow_name: "sales".to_string(),
                spec: None,
                resume: None,
            }))
            .await
            .unwrap_err()
            .to_string(),
        );
        errors.push(
            mcp.list_flow_runs(Parameters(ListFlowRunsParams {
                runtime_uuid: "rt-1".to_string(),
                status: None,
                flow_name: None,
                since: None,
                until: None,
                offset: None,
                limit: None,
            }))
            .await
            .unwrap_err()
            .to_string(),
        );
        errors.push(
            mcp.get_flow_run(Parameters(GetFlowRunParams {
                runtime_uuid: "rt-1".to_string(),
                name: "fr-1".to_string(),
            }))
            .await
            .unwrap_err()
            .to_string(),
        );

        for err in errors {
            assert!(err.contains("Ascend client is not configured"));
            assert!(err.contains("ASCEND_SERVICE_ACCOUNT_ID"));
        }
    }
}
