use std::fmt;
use std::io::BufReader;
use std::ops::ControlFlow;

use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;
use ureq::Agent;

use crate::auth::Auth;
use crate::config::Config;
use crate::error::{Error, JsonResultExt, Result, UreqResultExt};
use crate::models::{
    Deployment, Environment, Flow, FlowRun, FlowRunFilters, FlowRunList, FlowRunTrigger,
    OttoChatRequest, OttoChatResponse, OttoProvider, Project, Runtime, RuntimeCreate,
    RuntimeFilters, RuntimeKind, RuntimeUpdate, StreamEvent, Workspace,
};
use crate::sse::SseReader;

const PATH_SEGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'#').add(b'%').add(b'/').add(b'?');

/// Encode for use in URL query parameter values.
/// Uses NON_ALPHANUMERIC to correctly encode &, =, +, and other reserved characters.
const QUERY_VALUE: &AsciiSet = NON_ALPHANUMERIC;

fn encode_path(s: &str) -> String {
    utf8_percent_encode(s, PATH_SEGMENT).to_string()
}

fn encode_query_value(s: &str) -> String {
    utf8_percent_encode(s, QUERY_VALUE).to_string()
}

/// Builds a URL query string from key-value pairs.
struct QueryString(Vec<String>);

impl QueryString {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, key: &str, value: impl std::fmt::Display) {
        self.0
            .push(format!("{key}={}", encode_query_value(&value.to_string())));
    }

    fn push_opt(&mut self, key: &str, value: Option<impl std::fmt::Display>) {
        if let Some(v) = value {
            self.push(key, v);
        }
    }

    fn finish(self) -> String {
        if self.0.is_empty() {
            String::new()
        } else {
            format!("?{}", self.0.join("&"))
        }
    }
}

/// Client for the Ascend Instance API v1.
pub struct AscendClient {
    agent: Agent,
    streaming_agent: Agent,
    instance_api_url: String,
    auth: Auth,
}

impl AscendClient {
    /// Returns the Instance API URL.
    pub fn instance_api_url(&self) -> &str {
        &self.instance_api_url
    }

    /// Returns the service account ID.
    pub fn service_account_id(&self) -> &str {
        self.auth.service_account_id()
    }

    pub fn new(config: Config) -> Result<Self> {
        let agent = crate::new_agent();
        let streaming_agent = crate::new_streaming_agent();
        let auth = Auth::new(
            config.service_account_id,
            &config.service_account_key,
            config.instance_api_url.clone(),
            agent.clone(),
        )?;
        Ok(Self {
            agent,
            streaming_agent,
            instance_api_url: config.instance_api_url,
            auth,
        })
    }

    // -- Runtimes --

    pub fn list_runtimes(&self, filters: RuntimeFilters) -> Result<Vec<Runtime>> {
        let mut qs = QueryString::new();
        qs.push_opt("id", filters.id.as_deref());
        qs.push_opt("title", filters.title.as_deref());
        qs.push_opt("kind", filters.kind.map(|k| k.as_str()));
        qs.push_opt("project", filters.project.as_deref());
        qs.push_opt("environment", filters.environment.as_deref());
        self.get(&format!("/api/v1/runtimes{}", qs.finish()))
    }

    pub fn get_runtime(&self, uuid: &str) -> Result<Runtime> {
        self.get(&format!("/api/v1/runtimes/{}", encode_path(uuid)))
    }

    pub fn resume_runtime(&self, uuid: &str) -> Result<Runtime> {
        self.post_empty(&format!("/api/v1/runtimes/{}:resume", encode_path(uuid)))
    }

    pub fn pause_runtime(&self, uuid: &str) -> Result<Runtime> {
        self.post_empty(&format!("/api/v1/runtimes/{}:pause", encode_path(uuid)))
    }

    fn create_runtime_at(&self, path: &str, create: &RuntimeCreate) -> Result<Runtime> {
        let body =
            serde_json::to_value(create).with_json_serialize_context("RuntimeCreate body")?;
        self.post_json(path, &body)
    }

    pub fn update_runtime(&self, uuid: &str, update: &RuntimeUpdate) -> Result<Runtime> {
        let body =
            serde_json::to_value(update).with_json_serialize_context("RuntimeUpdate body")?;
        self.patch_json(&format!("/api/v1/runtimes/{}", encode_path(uuid)), &body)
    }

    pub fn delete_runtime(&self, uuid: &str) -> Result<()> {
        self.delete_empty(&format!("/api/v1/runtimes/{}", encode_path(uuid)))
    }

    /// Resolve a runtime by its title and kind. Returns exactly one match.
    ///
    /// Errors if zero or multiple runtimes match.
    /// Prefer `get_workspace`/`get_deployment` for most use cases.
    pub fn resolve_runtime_by_title(&self, title: &str, kind: RuntimeKind) -> Result<Runtime> {
        let runtimes = self.list_runtimes(RuntimeFilters {
            title: Some(title.to_string()),
            kind: Some(kind),
            ..Default::default()
        })?;
        resolve_one(runtimes, kind, title, |r| (&r.uuid, &r.title))
    }

    /// Resolve a runtime UUID from a title or UUID string.
    ///
    /// If `uuid_override` is `Some`, uses it directly. Otherwise resolves by title+kind.
    /// Prefer `get_workspace`/`get_deployment` for most use cases.
    pub fn resolve_runtime_uuid(
        &self,
        title: &str,
        kind: RuntimeKind,
        uuid_override: Option<&str>,
    ) -> Result<String> {
        if let Some(uuid) = uuid_override {
            return Ok(uuid.to_string());
        }
        self.resolve_runtime_by_title(title, kind).map(|r| r.uuid)
    }

    // -- Workspaces (convenience wrappers that set kind=workspace) --

    pub fn list_workspaces(&self, mut filters: RuntimeFilters) -> Result<Vec<Workspace>> {
        filters.kind = Some(RuntimeKind::Workspace);
        self.list_runtimes(filters)
            .map(|v| v.into_iter().map(Workspace).collect())
    }

    /// Get a workspace by title. If `uuid` is provided, it is used directly
    /// and `title` is ignored.
    pub fn get_workspace(&self, title: &str, uuid: Option<&str>) -> Result<Workspace> {
        if let Some(uuid) = uuid {
            self.get_runtime(uuid).map(Workspace)
        } else {
            self.resolve_runtime_by_title(title, RuntimeKind::Workspace)
                .map(Workspace)
        }
    }

    pub fn create_workspace(&self, create: &RuntimeCreate) -> Result<Workspace> {
        self.create_runtime_at("/api/v1/workspaces", create)
            .map(Workspace)
    }

    pub fn update_workspace(
        &self,
        title: &str,
        uuid: Option<&str>,
        update: &RuntimeUpdate,
    ) -> Result<Workspace> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Workspace, uuid)?;
        self.update_runtime(&uuid, update).map(Workspace)
    }

    pub fn pause_workspace(&self, title: &str, uuid: Option<&str>) -> Result<Workspace> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Workspace, uuid)?;
        self.pause_runtime(&uuid).map(Workspace)
    }

    pub fn resume_workspace(&self, title: &str, uuid: Option<&str>) -> Result<Workspace> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Workspace, uuid)?;
        self.resume_runtime(&uuid).map(Workspace)
    }

    pub fn delete_workspace(&self, title: &str, uuid: Option<&str>) -> Result<()> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Workspace, uuid)?;
        self.delete_runtime(&uuid)
    }

    // -- Deployments (convenience wrappers that set kind=deployment) --

    pub fn list_deployments(&self, mut filters: RuntimeFilters) -> Result<Vec<Deployment>> {
        filters.kind = Some(RuntimeKind::Deployment);
        self.list_runtimes(filters)
            .map(|v| v.into_iter().map(Deployment).collect())
    }

    /// Get a deployment by title. If `uuid` is provided, it is used directly
    /// and `title` is ignored.
    pub fn get_deployment(&self, title: &str, uuid: Option<&str>) -> Result<Deployment> {
        if let Some(uuid) = uuid {
            self.get_runtime(uuid).map(Deployment)
        } else {
            self.resolve_runtime_by_title(title, RuntimeKind::Deployment)
                .map(Deployment)
        }
    }

    pub fn create_deployment(&self, create: &RuntimeCreate) -> Result<Deployment> {
        self.create_runtime_at("/api/v1/deployments", create)
            .map(Deployment)
    }

    pub fn update_deployment(
        &self,
        title: &str,
        uuid: Option<&str>,
        update: &RuntimeUpdate,
    ) -> Result<Deployment> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Deployment, uuid)?;
        self.update_runtime(&uuid, update).map(Deployment)
    }

    pub fn pause_deployment_automations(
        &self,
        title: &str,
        uuid: Option<&str>,
    ) -> Result<Deployment> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Deployment, uuid)?;
        self.update_runtime(
            &uuid,
            &RuntimeUpdate {
                enable_automations: Some(false),
                ..Default::default()
            },
        )
        .map(Deployment)
    }

    pub fn resume_deployment_automations(
        &self,
        title: &str,
        uuid: Option<&str>,
    ) -> Result<Deployment> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Deployment, uuid)?;
        self.update_runtime(
            &uuid,
            &RuntimeUpdate {
                enable_automations: Some(true),
                ..Default::default()
            },
        )
        .map(Deployment)
    }

    pub fn delete_deployment(&self, title: &str, uuid: Option<&str>) -> Result<()> {
        let uuid = self.resolve_runtime_uuid(title, RuntimeKind::Deployment, uuid)?;
        self.delete_runtime(&uuid)
    }

    // -- Cross-kind resolution --

    /// Resolve a runtime UUID from workspace title, deployment title, or UUID.
    ///
    /// Exactly one of the three must be provided. Returns `MissingField` if none are set.
    pub fn resolve_runtime_target(
        &self,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
    ) -> Result<String> {
        if let Some(uuid) = uuid {
            return Ok(uuid.to_string());
        }
        if let Some(ws) = workspace {
            return self.resolve_runtime_uuid(ws, RuntimeKind::Workspace, None);
        }
        if let Some(dep) = deployment {
            return self.resolve_runtime_uuid(dep, RuntimeKind::Deployment, None);
        }
        Err(Error::MissingField {
            context: "target",
            field: "workspace, deployment, or uuid",
        })
    }

    /// Like `resolve_runtime_target` but returns `None` when no target is specified.
    pub fn resolve_optional_runtime_target(
        &self,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
    ) -> Result<Option<String>> {
        if uuid.is_none() && workspace.is_none() && deployment.is_none() {
            return Ok(None);
        }
        self.resolve_runtime_target(workspace, deployment, uuid)
            .map(Some)
    }

    // -- Environments --

    pub fn list_environments(&self) -> Result<Vec<Environment>> {
        self.get("/api/v1/environments")
    }

    pub fn get_environment(&self, title: &str) -> Result<Environment> {
        let mut qs = QueryString::new();
        qs.push("title", title);
        let envs: Vec<Environment> = self.get(&format!("/api/v1/environments{}", qs.finish()))?;
        resolve_one(envs, "environment", title, |e| (&e.uuid, &e.title))
    }

    // -- Projects --

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        self.get("/api/v1/projects")
    }

    pub fn get_project(&self, title: &str) -> Result<Project> {
        let mut qs = QueryString::new();
        qs.push("title", title);
        let projects: Vec<Project> = self.get(&format!("/api/v1/projects{}", qs.finish()))?;
        resolve_one(projects, "project", title, |p| (&p.uuid, &p.title))
    }

    // -- Profiles --

    pub fn list_profiles(
        &self,
        runtime_uuid: Option<&str>,
        project: Option<&str>,
        branch: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut qs = QueryString::new();
        qs.push_opt("runtime_uuid", runtime_uuid);
        qs.push_opt("project", project);
        qs.push_opt("branch", branch);
        self.get(&format!("/api/v1/profiles{}", qs.finish()))
    }

    // -- Flows --

    pub fn list_flows(&self, runtime_uuid: &str) -> Result<Vec<Flow>> {
        self.get(&format!(
            "/api/v1/runtimes/{}/flows",
            encode_path(runtime_uuid)
        ))
    }

    pub fn run_flow(
        &self,
        runtime_uuid: &str,
        flow_name: &str,
        spec: Option<Value>,
        resume: bool,
    ) -> Result<FlowRunTrigger> {
        let runtime = self.get_runtime(runtime_uuid)?;
        if runtime.paused {
            if resume {
                self.resume_runtime(runtime_uuid)?;
            } else {
                return Err(Error::RuntimePaused);
            }
        } else {
            match runtime.health.as_deref() {
                Some("running") => {}
                Some("starting") => return Err(Error::RuntimeStarting),
                Some("error") => return Err(Error::RuntimeInErrorState),
                Some(other) => {
                    return Err(Error::RuntimeUnexpectedHealth {
                        health: other.to_string(),
                    });
                }
                None => return Err(Error::RuntimeHealthMissing),
            }
        }
        let path = format!(
            "/api/v1/runtimes/{}/flows/{}:run",
            encode_path(runtime_uuid),
            encode_path(flow_name)
        );
        match spec {
            Some(spec) => self.post_json(&path, &serde_json::json!({ "spec": spec })),
            None => self.post_empty(&path),
        }
    }

    // -- Flow runs --

    pub fn list_flow_runs(
        &self,
        runtime_uuid: &str,
        filters: FlowRunFilters,
    ) -> Result<FlowRunList> {
        let mut qs = QueryString::new();
        qs.push("runtime_uuid", runtime_uuid);
        qs.push_opt("status", filters.status.as_deref());
        qs.push_opt("flow", filters.flow.as_deref());
        qs.push_opt("since", filters.since.as_deref());
        qs.push_opt("until", filters.until.as_deref());
        qs.push_opt("offset", filters.offset);
        qs.push_opt("limit", filters.limit);
        self.get(&format!("/api/v1/flow-runs{}", qs.finish()))
    }

    pub fn get_flow_run(&self, runtime_uuid: &str, name: &str) -> Result<FlowRun> {
        self.get(&format!(
            "/api/v1/flow-runs/{}?runtime_uuid={}",
            encode_path(name),
            encode_query_value(runtime_uuid)
        ))
    }

    // -- Otto --

    /// List available Otto providers and their enabled models.
    pub fn list_otto_providers(&self) -> Result<Vec<OttoProvider>> {
        self.get("/api/v1/otto/providers")
    }

    /// Send a message to Otto via the threads API.
    ///
    /// Collects the full response before returning. For real-time streaming,
    /// use [`otto_streaming`] instead.
    pub fn otto(&self, request: &OttoChatRequest) -> Result<OttoChatResponse> {
        let mut full_message = String::new();
        let response = self.otto_streaming(
            request,
            |event| {
                if let StreamEvent::TextDelta(delta) = event {
                    full_message.push_str(&delta);
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )?;
        Ok(OttoChatResponse {
            message: full_message,
            thread_id: response.thread_id,
        })
    }

    /// Send a message to Otto, streaming events to `on_event` as they arrive.
    ///
    /// `on_thread_id` is called with the thread ID as soon as the thread is
    /// created (before any events arrive). `on_event` receives each stream
    /// event (text deltas, tool calls) and returns `ControlFlow::Continue(())`
    /// to keep streaming or `ControlFlow::Break(())` to cancel early. The
    /// returned `OttoChatResponse` has an empty `message` — the caller is
    /// expected to have accumulated the text via the callback.
    pub fn otto_streaming(
        &self,
        request: &OttoChatRequest,
        mut on_event: impl FnMut(StreamEvent) -> ControlFlow<()>,
        on_thread_id: impl FnOnce(&str),
    ) -> Result<OttoChatResponse> {
        let body =
            serde_json::to_value(request).with_json_serialize_context("OttoChatRequest body")?;
        let token = self.auth.get_token()?;

        // Step 1: Create thread or send message to existing thread
        let (path, context) = if let Some(ref tid) = request.thread_id {
            (
                format!("/api/v1/otto/threads/{tid}/messages"),
                format!("POST /api/v1/otto/threads/{tid}/messages"),
            )
        } else {
            (
                "/api/v1/otto/threads".to_string(),
                "POST /api/v1/otto/threads".to_string(),
            )
        };

        let url = format!("{}{path}", self.instance_api_url);
        let json_body = serde_json::to_string(&body)
            .with_json_serialize_context(format!("{context} request body"))?;
        let is_follow_up = request.thread_id.is_some();
        let create_resp: Value = {
            let mut last_err = None;
            let mut resp_val = None;
            // Retry on 409 (thread still processing from a cancelled run).
            // The stop request is async, so the thread may not have fully
            // wound down yet when the user sends a follow-up message.
            let attempts = if is_follow_up { 30 } else { 1 };
            for attempt in 0..attempts {
                if attempt > 0 {
                    // Call stop to nudge the backend, then wait before retrying
                    if let Some(ref tid) = request.thread_id {
                        let _ = self.stop_thread(tid);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                let resp = self
                    .agent
                    .post(&url)
                    .header("Authorization", &format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .send(json_body.as_bytes())
                    .with_request_context(context.clone())?;
                let status = resp.status().as_u16();
                if status == 409 && is_follow_up {
                    let body_str = resp.into_body().read_to_string().unwrap_or_default();
                    last_err = Some(api_error(status, &body_str));
                    continue;
                }
                resp_val = Some(handle_response(resp, &context)?);
                break;
            }
            match resp_val {
                Some(v) => v,
                None => return Err(last_err.unwrap()),
            }
        };

        let thread_id = create_resp
            .get("thread_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ApiError {
                status: 500,
                message: "missing thread_id in response".to_string(),
            })?
            .to_string();

        on_thread_id(&thread_id);

        // Step 2: Stream SSE events from the thread updates endpoint (no global timeout)
        let updates_path = format!("/api/v1/otto/threads/{thread_id}/updates");
        let updates_url = format!("{}{updates_path}", self.instance_api_url);
        let updates_context = format!("GET {updates_path}");
        let updates_resp = self
            .streaming_agent
            .get(&updates_url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Accept", "text/event-stream")
            .call()
            .with_request_context(updates_context.clone())?;

        if !(200..300).contains(&updates_resp.status().as_u16()) {
            return check_error_status(updates_resp, &updates_context).map(|()| OttoChatResponse {
                message: String::new(),
                thread_id: Some(thread_id),
            });
        }

        // Parse SSE stream, dispatching events to the caller
        let reader = BufReader::new(updates_resp.into_body().into_reader());

        for event_result in SseReader::new(reader) {
            let event = event_result?;
            let event_type = event.event_type.as_deref();

            if event_type == Some("thread.done") || event_type == Some("thread.stopped") {
                break;
            }

            let Ok(data) = serde_json::from_str::<Value>(&event.data) else {
                continue;
            };

            let stream_event = match event_type {
                Some("response.output_text.delta") => data
                    .get("delta")
                    .and_then(|v| v.as_str())
                    .map(|d| StreamEvent::TextDelta(d.to_string())),
                Some("response.output_item.added") => {
                    let Some(item) = data.get("item") else {
                        continue;
                    };
                    if item.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                        Some(StreamEvent::ToolCallStart {
                            call_id: item
                                .get("call_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            name: item
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            arguments: item
                                .get("arguments")
                                .and_then(|v| v.as_str())
                                .unwrap_or("{}")
                                .to_string(),
                        })
                    } else {
                        None
                    }
                }
                Some("response.run_item_stream_event.tool_call_output_item") => {
                    Some(StreamEvent::ToolCallOutput {
                        call_id: data
                            .get("call_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        output: data
                            .get("output")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                }
                _ => None,
            };

            if let Some(se) = stream_event
                && on_event(se).is_break()
            {
                break;
            }
        }

        Ok(OttoChatResponse {
            message: String::new(),
            thread_id: Some(thread_id),
        })
    }

    /// Stop a running Otto thread. Returns the thread ID and status
    /// ("stopping", "not_processing", or "not_found").
    pub fn stop_thread(&self, thread_id: &str) -> Result<Value> {
        self.post_empty(&format!(
            "/api/v1/otto/threads/{}/stop",
            encode_path(thread_id)
        ))
    }

    /// Stop a running Otto thread and wait until processing has fully stopped.
    /// Polls the stop endpoint until the backend reports "not_processing".
    /// Returns an error if the thread does not stop within ~5 seconds.
    pub fn stop_thread_and_wait(&self, thread_id: &str) -> Result<()> {
        let resp: Value = self.stop_thread(thread_id)?;
        let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "stopping" {
            return Ok(());
        }
        // Poll until the backend confirms the thread is no longer processing
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let resp: Value = self.stop_thread(thread_id)?;
            let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status != "stopping" {
                return Ok(());
            }
        }
        Err(Error::ApiError {
            status: 408,
            message: format!("thread {thread_id} did not stop within 5 seconds"),
        })
    }

    // -- HTTP helpers --

    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request("GET", path, None)
    }

    fn post_empty<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request("POST", path, None)
    }

    fn post_json<T: serde::de::DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request("POST", path, Some(body))
    }

    fn patch_json<T: serde::de::DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request("PATCH", path, Some(body))
    }

    fn delete_empty(&self, path: &str) -> Result<()> {
        with_retry(|| {
            let token = self.auth.get_token()?;
            let url = format!("{}{path}", self.instance_api_url);
            let context = format!("DELETE {path}");
            let resp = self
                .agent
                .delete(&url)
                .header("Authorization", &format!("Bearer {token}"))
                .call()
                .with_request_context(context.clone())?;
            check_error_status(resp, &context)
        })
    }

    /// Unified request helper for GET, POST, and PATCH.
    fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T> {
        with_retry(|| {
            let token = self.auth.get_token()?;
            let url = format!("{}{path}", self.instance_api_url);
            let context = format!("{method} {path}");

            let resp = match (method, body) {
                ("GET", _) => self
                    .agent
                    .get(&url)
                    .header("Authorization", &format!("Bearer {token}"))
                    .call()
                    .with_request_context(context.clone())?,
                (m, Some(body)) => {
                    let json_body = serde_json::to_string(body)
                        .with_json_serialize_context(format!("{context} request body"))?;
                    let req = match m {
                        "POST" => self.agent.post(&url),
                        "PATCH" => self.agent.patch(&url),
                        _ => unreachable!("unsupported method with body: {m}"),
                    };
                    req.header("Authorization", &format!("Bearer {token}"))
                        .header("Content-Type", "application/json")
                        .send(json_body.as_bytes())
                        .with_request_context(context.clone())?
                }
                ("POST", None) => self
                    .agent
                    .post(&url)
                    .header("Authorization", &format!("Bearer {token}"))
                    .send_empty()
                    .with_request_context(context.clone())?,
                _ => unreachable!("unsupported method without body: {method}"),
            };

            handle_response(resp, &context)
        })
    }
}

/// Retry a request on transient errors (429, 502, 503, 504, or network failures).
/// Uses exponential backoff: 500ms, 1s, 2s. Max 3 retries.
fn with_retry<T>(mut f: impl FnMut() -> Result<T>) -> Result<T> {
    let delays = [
        std::time::Duration::from_millis(500),
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(2),
    ];
    let mut last_err = None;
    for attempt in 0..=delays.len() {
        match f() {
            Ok(val) => return Ok(val),
            Err(e) if e.is_retryable() => {
                if attempt < delays.len() {
                    std::thread::sleep(delays[attempt]);
                }
                last_err = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.expect("at least one attempt"))
}

/// Resolve exactly one item from a list, returning its UUID.
/// Errors with `NotFound` or `AmbiguousTitle` for 0 or >1 matches.
fn resolve_one<T>(
    items: Vec<T>,
    kind: impl fmt::Display,
    title: &str,
    extract: impl Fn(&T) -> (&str, &str),
) -> Result<T> {
    match items.len() {
        0 => Err(Error::NotFound {
            kind: kind.to_string(),
            title: title.to_string(),
        }),
        1 => Ok(items.into_iter().next().unwrap_or_else(|| unreachable!())),
        _ => Err(Error::AmbiguousTitle {
            kind: kind.to_string(),
            title: title.to_string(),
            matches: items
                .iter()
                .map(|item| {
                    let (uuid, title) = extract(item);
                    (uuid.to_string(), title.to_string())
                })
                .collect(),
        }),
    }
}

/// Check HTTP status and return an error for non-2xx responses. Discards the body on success.
fn check_error_status(mut resp: ureq::http::Response<ureq::Body>, context: &str) -> Result<()> {
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let body: String = resp
            .body_mut()
            .read_to_string()
            .with_response_read_context(context.to_string())?;
        return Err(api_error(status, &body));
    }
    Ok(())
}

fn handle_response<T: serde::de::DeserializeOwned>(
    mut resp: ureq::http::Response<ureq::Body>,
    context: &str,
) -> Result<T> {
    let status = resp.status().as_u16();
    let body: String = resp
        .body_mut()
        .read_to_string()
        .with_response_read_context(context.to_string())?;

    if !(200..300).contains(&status) {
        return Err(api_error(status, &body));
    }

    serde_json::from_str(&body).with_json_parse_context(format!("{context} response"))
}

/// Build an ApiError, preferring the `detail` field from JSON responses.
fn api_error(status: u16, body: &str) -> Error {
    if let Ok(json) = serde_json::from_str::<Value>(body)
        && let Some(detail) = json.get("detail").and_then(|v| v.as_str())
    {
        return Error::ApiError {
            status,
            message: detail.to_string(),
        };
    }
    Error::ApiError {
        status,
        message: body.to_string(),
    }
}

impl std::fmt::Debug for AscendClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AscendClient")
            .field("instance_api_url", &self.instance_api_url)
            .field("auth", &self.auth)
            .finish_non_exhaustive()
    }
}
