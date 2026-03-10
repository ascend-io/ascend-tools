use std::io::BufReader;

use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;
use ureq::Agent;

use crate::auth::Auth;
use crate::config::Config;
use crate::error::{Error, JsonResultExt, Result, UreqResultExt};
use crate::models::{
    Environment, Flow, FlowRun, FlowRunFilters, FlowRunList, FlowRunTrigger, KIND_DEPLOYMENT,
    KIND_WORKSPACE, OttoChatRequest, OttoChatResponse, OttoProvider, Project, Runtime,
    RuntimeCreate, RuntimeFilters, RuntimeUpdate,
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
    instance_api_url: String,
    auth: Auth,
}

impl AscendClient {
    pub fn new(config: Config) -> Result<Self> {
        let agent = crate::new_agent();
        let auth = Auth::new(
            config.service_account_id,
            &config.service_account_key,
            config.instance_api_url.clone(),
            agent.clone(),
        )?;
        Ok(Self {
            agent,
            instance_api_url: config.instance_api_url,
            auth,
        })
    }

    // -- Runtimes --

    pub fn list_runtimes(&self, filters: RuntimeFilters) -> Result<Vec<Runtime>> {
        let mut qs = QueryString::new();
        qs.push_opt("id", filters.id.as_deref());
        qs.push_opt("title", filters.title.as_deref());
        qs.push_opt("kind", filters.kind.as_deref());
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
        let body = serde_json::to_value(create)
            .with_json_serialize_context("RuntimeCreate body".to_string())?;
        self.post_json(path, &body)
    }

    pub fn update_runtime(&self, uuid: &str, update: &RuntimeUpdate) -> Result<Runtime> {
        let body = serde_json::to_value(update)
            .with_json_serialize_context("RuntimeUpdate body".to_string())?;
        self.patch_json(&format!("/api/v1/runtimes/{}", encode_path(uuid)), &body)
    }

    pub fn delete_runtime(&self, uuid: &str) -> Result<()> {
        self.delete_empty(&format!("/api/v1/runtimes/{}", encode_path(uuid)))
    }

    /// Resolve a runtime by its title and kind. Returns exactly one match.
    ///
    /// Errors if zero or multiple runtimes match.
    pub fn resolve_runtime_by_title(&self, title: &str, kind: &str) -> Result<Runtime> {
        let runtimes = self.list_runtimes(RuntimeFilters {
            title: Some(title.to_string()),
            kind: Some(kind.to_string()),
            ..Default::default()
        })?;
        resolve_one(runtimes, kind, title, |r| (&r.uuid, &r.title))
    }

    /// Resolve a runtime UUID from a title or UUID string.
    ///
    /// If `uuid_override` is `Some`, uses it directly. Otherwise resolves by title+kind.
    pub fn resolve_runtime_uuid(
        &self,
        title: &str,
        kind: &str,
        uuid_override: Option<&str>,
    ) -> Result<String> {
        if let Some(uuid) = uuid_override {
            return Ok(uuid.to_string());
        }
        self.resolve_runtime_by_title(title, kind).map(|r| r.uuid)
    }

    // -- Workspaces (convenience wrappers that set kind=workspace) --

    pub fn list_workspaces(&self, filters: RuntimeFilters) -> Result<Vec<Runtime>> {
        let mut filters = filters;
        filters.kind = Some(KIND_WORKSPACE.into());
        self.list_runtimes(filters)
    }

    pub fn get_workspace(&self, title: &str, uuid: Option<&str>) -> Result<Runtime> {
        if let Some(uuid) = uuid {
            self.get_runtime(uuid)
        } else {
            self.resolve_runtime_by_title(title, KIND_WORKSPACE)
        }
    }

    pub fn create_workspace(&self, create: &RuntimeCreate) -> Result<Runtime> {
        self.create_runtime_at("/api/v1/workspaces", create)
    }

    pub fn update_workspace(
        &self,
        title: &str,
        uuid: Option<&str>,
        update: &RuntimeUpdate,
    ) -> Result<Runtime> {
        let uuid = self.resolve_runtime_uuid(title, KIND_WORKSPACE, uuid)?;
        self.update_runtime(&uuid, update)
    }

    pub fn pause_workspace(&self, title: &str, uuid: Option<&str>) -> Result<Runtime> {
        let uuid = self.resolve_runtime_uuid(title, KIND_WORKSPACE, uuid)?;
        self.pause_runtime(&uuid)
    }

    pub fn resume_workspace(&self, title: &str, uuid: Option<&str>) -> Result<Runtime> {
        let uuid = self.resolve_runtime_uuid(title, KIND_WORKSPACE, uuid)?;
        self.resume_runtime(&uuid)
    }

    pub fn delete_workspace(&self, title: &str, uuid: Option<&str>) -> Result<()> {
        let uuid = self.resolve_runtime_uuid(title, KIND_WORKSPACE, uuid)?;
        self.delete_runtime(&uuid)
    }

    // -- Deployments (convenience wrappers that set kind=deployment) --

    pub fn list_deployments(&self, filters: RuntimeFilters) -> Result<Vec<Runtime>> {
        let mut filters = filters;
        filters.kind = Some(KIND_DEPLOYMENT.into());
        self.list_runtimes(filters)
    }

    pub fn get_deployment(&self, title: &str, uuid: Option<&str>) -> Result<Runtime> {
        if let Some(uuid) = uuid {
            self.get_runtime(uuid)
        } else {
            self.resolve_runtime_by_title(title, KIND_DEPLOYMENT)
        }
    }

    pub fn create_deployment(&self, create: &RuntimeCreate) -> Result<Runtime> {
        self.create_runtime_at("/api/v1/deployments", create)
    }

    pub fn update_deployment(
        &self,
        title: &str,
        uuid: Option<&str>,
        update: &RuntimeUpdate,
    ) -> Result<Runtime> {
        let uuid = self.resolve_runtime_uuid(title, KIND_DEPLOYMENT, uuid)?;
        self.update_runtime(&uuid, update)
    }

    pub fn delete_deployment(&self, title: &str, uuid: Option<&str>) -> Result<()> {
        let uuid = self.resolve_runtime_uuid(title, KIND_DEPLOYMENT, uuid)?;
        self.delete_runtime(&uuid)
    }

    // -- Environments --

    pub fn list_environments(&self) -> Result<Vec<Environment>> {
        self.get("/api/v1/environments")
    }

    pub fn get_environment(&self, title: &str) -> Result<Environment> {
        let envs: Vec<Environment> = self.get(&format!(
            "/api/v1/environments?title={}",
            encode_query_value(title)
        ))?;
        resolve_one(envs, "environment", title, |e| (&e.uuid, &e.title))
    }

    // -- Projects --

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        self.get("/api/v1/projects")
    }

    pub fn get_project(&self, title: &str) -> Result<Project> {
        let projects: Vec<Project> = self.get(&format!(
            "/api/v1/projects?title={}",
            encode_query_value(title)
        ))?;
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

    /// Send a chat message to Otto and return the assistant's response.
    ///
    /// List available Otto providers and their enabled models.
    pub fn list_otto_providers(&self) -> Result<Vec<OttoProvider>> {
        self.get("/api/v1/otto/providers")
    }

    /// Consumes the SSE stream from `/api/v1/otto/chat` and extracts the last
    /// assistant message text.
    pub fn otto_chat(&self, request: &OttoChatRequest) -> Result<OttoChatResponse> {
        let body = serde_json::to_value(request)
            .with_json_serialize_context("OttoChatRequest body".to_string())?;
        let token = self.auth.get_token()?;
        let url = format!("{}/api/v1/otto/chat", self.instance_api_url);
        let context = "POST /api/v1/otto/chat";
        let json_body = serde_json::to_string(&body)
            .with_json_serialize_context(format!("{context} request body"))?;
        let resp = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .send(json_body.as_bytes())
            .with_request_context(context.to_string())?;

        if !(200..300).contains(&resp.status().as_u16()) {
            return check_error_status(resp, context).map(|()| OttoChatResponse {
                message: String::new(),
                thread_id: None,
            });
        }

        // Parse SSE stream to extract assistant message and thread_id
        let reader = BufReader::new(resp.into_body().into_reader());
        let mut last_message = String::new();
        let mut thread_id = None;

        for event_result in SseReader::new(reader) {
            let event = event_result?;
            // Try to parse the data as JSON to extract message content
            if let Ok(data) = serde_json::from_str::<Value>(&event.data) {
                // Look for thread_id in thread-related events
                if let Some(tid) = data.get("thread_id").and_then(|v| v.as_str()) {
                    thread_id = Some(tid.to_string());
                }
                // Look for output text content in response events
                if let Some(text) = extract_assistant_text(&data) {
                    last_message = text;
                }
            }
        }

        Ok(OttoChatResponse {
            message: last_message,
            thread_id,
        })
    }

    // -- HTTP helpers --

    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let context = format!("GET {path}");
        let resp = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .call()
            .with_request_context(context.clone())?;
        handle_response(resp, &context)
    }

    fn post_empty<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let context = format!("POST {path}");
        let resp = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .send_empty()
            .with_request_context(context.clone())?;
        handle_response(resp, &context)
    }

    fn post_json<T: serde::de::DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let json_body = serde_json::to_string(body)
            .with_json_serialize_context(format!("POST {path} request body"))?;
        let context = format!("POST {path}");
        let resp = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .send(json_body.as_bytes())
            .with_request_context(context.clone())?;
        handle_response(resp, &context)
    }

    fn patch_json<T: serde::de::DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let json_body = serde_json::to_string(body)
            .with_json_serialize_context(format!("PATCH {path} request body"))?;
        let context = format!("PATCH {path}");
        let resp = self
            .agent
            .patch(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .send(json_body.as_bytes())
            .with_request_context(context.clone())?;
        handle_response(resp, &context)
    }

    fn delete_empty(&self, path: &str) -> Result<()> {
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
    }
}

/// Try to extract assistant text from an SSE data payload.
///
/// Otto uses the OpenAI Responses API format. Look for output items
/// with type "message" / role "assistant" containing text content.
fn extract_assistant_text(data: &Value) -> Option<String> {
    // response.output_item.done events contain the full output item
    let item = data.get("item").or(Some(data))?;
    if item.get("type").and_then(|v| v.as_str()) == Some("message")
        && item.get("role").and_then(|v| v.as_str()) == Some("assistant")
        && let Some(content) = item.get("content").and_then(|v| v.as_array())
    {
        let texts: Vec<&str> = content
            .iter()
            .filter(|c| c.get("type").and_then(|v| v.as_str()) == Some("output_text"))
            .filter_map(|c| c.get("text").and_then(|v| v.as_str()))
            .collect();
        if !texts.is_empty() {
            return Some(texts.join(""));
        }
    }
    None
}

/// Resolve exactly one item from a list, returning its UUID.
/// Errors with `NotFound` or `AmbiguousTitle` for 0 or >1 matches.
fn resolve_one<T>(
    items: Vec<T>,
    kind: &str,
    title: &str,
    extract: impl Fn(&T) -> (&str, &str),
) -> Result<T> {
    match items.len() {
        0 => Err(Error::NotFound {
            kind: kind.to_string(),
            title: title.to_string(),
        }),
        1 => Ok(items.into_iter().next().unwrap()),
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
            .finish()
    }
}
