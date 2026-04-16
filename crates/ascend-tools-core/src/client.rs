use std::collections::HashMap;
use std::fmt;
use std::io::BufReader;
use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;
use ureq::Agent;

use crate::auth::Auth;
use crate::config::Config;
use crate::error::{Error, JsonResultExt, Result, UreqResultExt};
use crate::models::{
    Conversation, ConversationFilters, ConversationList, Deployment, Environment, Flow, FlowRun,
    FlowRunFilters, FlowRunList, FlowRunTrigger, OttoChatRequest, OttoChatResponse, OttoModel,
    OttoProvider, OttoStreamStatus, OttoStreamUpdate, Project, Runtime, RuntimeCreate,
    RuntimeFilters, RuntimeKind, RuntimeUpdate, StreamEvent, Workspace,
};
use crate::sse::SseReader;

const PATH_SEGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'#').add(b'%').add(b'/').add(b'?');

/// Encode for use in URL query parameter values.
/// Uses NON_ALPHANUMERIC to correctly encode &, =, +, and other reserved characters.
const QUERY_VALUE: &AsciiSet = NON_ALPHANUMERIC;
const FOLLOW_UP_RETRY_TIMEOUT: Duration = Duration::from_secs(30);
const FOLLOW_UP_RETRY_INTERVAL: Duration = Duration::from_millis(500);
const STOP_THREAD_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_THREAD_POLL_INTERVAL_MIN: Duration = Duration::from_millis(100);
const STOP_THREAD_POLL_INTERVAL_MAX: Duration = Duration::from_millis(500);
/// Max retries for initial SSE connection (before any events arrive).
/// Mid-stream reconnection is not attempted because the backend replays
/// all buffered events on new subscriptions, producing duplicates.
const SSE_CONNECT_MAX_RETRIES: u32 = 3;
const SSE_CONNECT_BACKOFF: Duration = Duration::from_millis(500);

#[derive(Debug, serde::Deserialize)]
struct OttoProviderSettingsResponse {
    models: Vec<OttoProviderSettingsModel>,
}

#[derive(Debug, serde::Deserialize)]
struct OttoProviderSettingsModel {
    provider_id: String,
    id: String,
    #[serde(default)]
    thinking_levels: Option<Vec<String>>,
}

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

    // -- Conversations --

    /// List conversations (threads), ordered by most recent first.
    pub fn list_conversations(&self, filters: ConversationFilters) -> Result<ConversationList> {
        let mut qs = QueryString::new();
        qs.push_opt("offset", filters.offset);
        qs.push_opt("limit", filters.limit);
        qs.push_opt("title", filters.title.as_deref());
        self.get(&format!("/api/v1/otto/threads{}", qs.finish()))
    }

    /// Get a conversation by ID, including full message history.
    pub fn get_conversation(&self, id: &str) -> Result<Conversation> {
        self.get(&format!("/api/v1/otto/threads/{}", encode_path(id)))
    }

    /// Get a conversation by title or ID, including full message history.
    ///
    /// Auto-detects whether the input is a conversation ID or title: tries an
    /// ID lookup first, then falls back to title search. See
    /// [`resolve_conversation_id`](Self::resolve_conversation_id) for details.
    pub fn get_conversation_by_title(&self, title_or_id: &str) -> Result<Conversation> {
        let id = self.resolve_conversation_id(title_or_id)?;
        self.get_conversation(&id)
    }

    /// Resolve a conversation title or ID to an ID.
    ///
    /// Tries the input as a conversation ID first (cheap single-item fetch). If
    /// that succeeds, returns the ID immediately. If it 404s, falls back to a
    /// server-side title search. Errors with `AmbiguousTitle` if multiple
    /// conversations share the same title.
    pub fn resolve_conversation_id(&self, title_or_id: &str) -> Result<String> {
        // Try as ID first — common case with --resume or pasted IDs.
        match self.get_conversation(title_or_id) {
            Ok(c) => return Ok(c.id),
            Err(ref e) if e.http_status() == Some(404) => {}
            Err(e) => return Err(e),
        }

        // Not a valid ID — use server-side title filter.
        let list = self.list_conversations(ConversationFilters {
            title: Some(title_or_id.to_string()),
            ..Default::default()
        })?;
        match list.threads.len() {
            0 => Err(Error::NotFound {
                kind: "conversation".to_string(),
                title: title_or_id.to_string(),
            }),
            1 => Ok(list.threads.into_iter().next().unwrap().id),
            _ => Err(Error::AmbiguousTitle {
                kind: "conversation".to_string(),
                title: title_or_id.to_string(),
                matches: list
                    .threads
                    .iter()
                    .map(|c| (c.id.clone(), c.title.clone().unwrap_or_default()))
                    .collect(),
            }),
        }
    }

    /// Get the ID of the most recent conversation.
    pub fn latest_conversation_id(&self) -> Result<String> {
        let list = self.list_conversations(ConversationFilters {
            limit: Some(1),
            ..Default::default()
        })?;
        list.threads
            .into_iter()
            .next()
            .map(|c| c.id)
            .ok_or_else(|| Error::NotFound {
                kind: "conversation".to_string(),
                title: "(most recent)".to_string(),
            })
    }

    /// Resolve a `conversation` or `thread_id` parameter to an optional thread ID.
    ///
    /// If `conversation` is `Some`, resolves it via `resolve_conversation_id`.
    /// Otherwise passes through `thread_id` as-is. Used by SDK bindings to
    /// avoid duplicating the resolution logic.
    pub fn resolve_otto_thread(
        &self,
        conversation: Option<&str>,
        thread_id: Option<String>,
    ) -> Result<Option<String>> {
        if let Some(conv) = conversation {
            Ok(Some(self.resolve_conversation_id(conv)?))
        } else {
            Ok(thread_id)
        }
    }

    // -- Otto --

    /// List available Otto providers and their enabled models.
    pub fn list_otto_providers(&self) -> Result<Vec<OttoProvider>> {
        let mut providers: Vec<OttoProvider> = self.get("/api/v1/otto/providers")?;
        if let Ok(settings) = self.get_otto_provider_settings() {
            merge_otto_provider_thinking_levels(&mut providers, settings);
        }
        Ok(providers)
    }

    /// Resolve a provider name and model name to an [`OttoModel`] with IDs.
    ///
    /// Accepts display names or IDs for both provider and model (case-insensitive).
    /// Returns `None` when `model` is `None`.
    pub fn resolve_otto_model(
        &self,
        provider: Option<&str>,
        model: Option<&str>,
    ) -> Result<Option<OttoModel>> {
        let model = match model {
            Some(m) => m,
            None => return Ok(None),
        };
        let providers = self.list_otto_providers()?;

        if let Some(provider_name) = provider {
            // Resolve provider by name or ID
            let lower = provider_name.to_lowercase();
            let matched: Vec<_> = providers
                .iter()
                .filter(|p| p.name.to_lowercase() == lower || p.id.to_lowercase() == lower)
                .collect();
            let prov = match matched.len() {
                1 => matched[0],
                0 => {
                    return Err(Error::NotFoundWithOptions {
                        kind: "otto provider".to_string(),
                        title: provider_name.to_string(),
                        available: providers
                            .iter()
                            .map(|p| format!("{} ({})", p.name, p.id))
                            .collect(),
                    });
                }
                _ => {
                    return Err(Error::AmbiguousTitle {
                        kind: "otto provider".to_string(),
                        title: provider_name.to_string(),
                        matches: matched
                            .iter()
                            .map(|p| (p.id.clone(), p.name.clone()))
                            .collect(),
                    });
                }
            };
            let model_lower = model.to_lowercase();
            let resolved = prov.models.iter().find(|m| {
                m.id.to_lowercase() == model_lower || m.name.to_lowercase() == model_lower
            });
            match resolved {
                Some(m) => Ok(Some(OttoModel::new(m.id.clone()))),
                None => Err(Error::NotFoundWithOptions {
                    kind: format!("model on provider '{}'", prov.name),
                    title: model.to_string(),
                    available: prov.models.iter().map(|m| m.name.clone()).collect(),
                }),
            }
        } else {
            // No provider specified — search all providers for the model.
            // Collect all matches to detect ambiguity across providers.
            let model_lower = model.to_lowercase();
            let mut matches: Vec<(&OttoProvider, &crate::models::OttoProviderModel)> = Vec::new();
            for prov in &providers {
                if let Some(m) = prov.models.iter().find(|m| {
                    m.id.to_lowercase() == model_lower || m.name.to_lowercase() == model_lower
                }) {
                    matches.push((prov, m));
                }
            }
            match matches.len() {
                1 => Ok(Some(OttoModel::new(matches[0].1.id.clone()))),
                0 => {
                    let available: Vec<String> = providers
                        .iter()
                        .flat_map(|p| p.models.iter().map(|m| m.name.clone()))
                        .collect();
                    Err(Error::NotFoundWithOptions {
                        kind: "otto model".to_string(),
                        title: model.to_string(),
                        available,
                    })
                }
                _ => Err(Error::AmbiguousTitle {
                    kind: "otto model".to_string(),
                    title: model.to_string(),
                    matches: matches
                        .iter()
                        .map(|(p, m)| (m.id.clone(), format!("{} ({})", m.name, p.name)))
                        .collect(),
                }),
            }
        }
    }

    /// Send a message to Otto via the threads API.
    ///
    /// Collects the full response before returning. For real-time streaming,
    /// use [`otto_streaming`] instead.
    pub fn otto(&self, request: &OttoChatRequest) -> Result<OttoChatResponse> {
        let mut full_message = String::new();
        let mut completed_snapshot_message = String::new();
        let response = self.otto_streaming_events(
            request,
            |raw_event| {
                if let Some(StreamEvent::TextDelta(delta)) =
                    raw_otto_stream_update_to_stream_event(&raw_event)
                {
                    full_message.push_str(&delta);
                }
                if raw_event.event_type == "thread.details"
                    && is_completed_thread_details_snapshot(raw_event.data.as_ref())
                {
                    let snapshot_message =
                        extract_completed_thread_details_message(raw_event.data.as_ref());
                    if !snapshot_message.is_empty() {
                        completed_snapshot_message = snapshot_message;
                    }
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )?;
        if response.stream_status != OttoStreamStatus::Completed {
            return Err(Error::OttoStreamEndedUnexpectedly {
                context: response
                    .stream_error
                    .unwrap_or_else(|| "stream did not complete".to_string()),
            });
        }
        Ok(OttoChatResponse {
            message: if !completed_snapshot_message.is_empty() {
                completed_snapshot_message
            } else {
                full_message
            },
            thread_id: response.thread_id,
            stream_status: OttoStreamStatus::Completed,
            stream_error: None,
        })
    }

    /// Send a message to Otto and expose the raw per-thread SSE updates.
    pub fn otto_streaming_events(
        &self,
        request: &OttoChatRequest,
        on_event: impl FnMut(OttoStreamUpdate) -> ControlFlow<()>,
        on_thread_id: impl FnOnce(&str),
    ) -> Result<OttoChatResponse> {
        let thread_id = self.start_otto_request(request)?;
        on_thread_id(&thread_id);
        self.stream_otto_updates(&thread_id, on_event)
    }

    /// Send a message to Otto, streaming events to `on_event` as they arrive.
    ///
    /// `on_thread_id` is called with the thread ID as soon as the thread is
    /// created (before any events arrive). `on_event` receives each stream
    /// event (text deltas, reasoning deltas, tool calls) and returns
    /// `ControlFlow::Continue(())` to keep streaming or
    /// `ControlFlow::Break(())` to cancel early. The returned
    /// `OttoChatResponse` has an empty `message` — the caller is expected to
    /// have accumulated the text via the callback.
    pub fn otto_streaming(
        &self,
        request: &OttoChatRequest,
        mut on_event: impl FnMut(StreamEvent) -> ControlFlow<()>,
        on_thread_id: impl FnOnce(&str),
    ) -> Result<OttoChatResponse> {
        self.otto_streaming_events(
            request,
            |raw_event| match raw_otto_stream_update_to_stream_event(&raw_event) {
                Some(stream_event) => on_event(stream_event),
                None => ControlFlow::Continue(()),
            },
            on_thread_id,
        )
    }

    fn start_otto_request(&self, request: &OttoChatRequest) -> Result<String> {
        let body =
            serde_json::to_value(request).with_json_serialize_context("OttoChatRequest body")?;
        let token = self.auth.get_token()?;

        let (path, context) = if let Some(ref tid) = request.thread_id {
            let encoded = encode_path(tid);
            (
                format!("/api/v1/otto/threads/{encoded}/messages"),
                format!("POST /api/v1/otto/threads/{encoded}/messages"),
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
        let retry_deadline = Instant::now() + FOLLOW_UP_RETRY_TIMEOUT;
        let create_resp: Value = {
            let mut last_err = None;
            let mut resp_val = None;
            let mut sent_stop = false;
            let mut current_token = token;
            loop {
                let resp = self
                    .agent
                    .post(&url)
                    .header("Authorization", &format!("Bearer {current_token}"))
                    .header("Content-Type", "application/json")
                    .send(json_body.as_bytes())
                    .with_request_context(context.clone())?;
                let status = resp.status().as_u16();
                if status == 409 && is_follow_up {
                    let body_str = resp.into_body().read_to_string().unwrap_or_default();
                    last_err = Some(api_error(status, &body_str));
                    if !sent_stop {
                        if let Some(ref tid) = request.thread_id {
                            let _ = self.stop_thread(tid);
                        }
                        sent_stop = true;
                    }
                    if Instant::now() >= retry_deadline {
                        break;
                    }
                    std::thread::sleep(FOLLOW_UP_RETRY_INTERVAL);
                    current_token = self.auth.get_token()?;
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

        create_resp
            .get("thread_id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| Error::ApiError {
                status: 500,
                message: "missing thread_id in response".to_string(),
            })
    }

    fn stream_otto_updates(
        &self,
        thread_id: &str,
        mut on_event: impl FnMut(OttoStreamUpdate) -> ControlFlow<()>,
    ) -> Result<OttoChatResponse> {
        // The backend SSE stream never closes naturally — it sends heartbeat
        // pings every 30s and stays open for future updates on the thread.
        // New subscriptions begin with a `thread.details` snapshot before any
        // live response events, so we only retry the initial connection (before
        // any events arrive). Mid-stream reconnection could duplicate caller-
        // visible updates or re-deliver a completed snapshot.
        let updates_path = format!("/api/v1/otto/threads/{}/updates", encode_path(thread_id));
        let updates_url = format!("{}{updates_path}", self.instance_api_url);
        let updates_context = format!("GET {updates_path}");

        let updates_resp = {
            let mut last_err = None;
            let mut resp = None;
            for attempt in 0..=SSE_CONNECT_MAX_RETRIES {
                if attempt > 0 {
                    let backoff = SSE_CONNECT_BACKOFF * 2u32.pow(attempt - 1);
                    std::thread::sleep(backoff);
                }
                let stream_token = self.auth.get_token()?;
                match self
                    .streaming_agent
                    .get(&updates_url)
                    .header("Authorization", &format!("Bearer {stream_token}"))
                    .header("Accept", "text/event-stream")
                    .call()
                {
                    Ok(r) => {
                        resp = Some(r);
                        break;
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            match resp {
                Some(r) => r,
                None => {
                    return Err(Error::RequestFailed {
                        context: updates_context.clone(),
                        source: last_err.unwrap(),
                    });
                }
            }
        };

        if !(200..300).contains(&updates_resp.status().as_u16()) {
            return check_error_status(updates_resp, &updates_context).map(|()| OttoChatResponse {
                message: String::new(),
                thread_id: Some(thread_id.to_string()),
                stream_status: OttoStreamStatus::Interrupted,
                stream_error: Some(format!("{updates_context} returned non-2xx status")),
            });
        }

        let reader = BufReader::new(updates_resp.into_body().into_reader());

        let mut saw_terminal_event = false;
        let mut cancelled_by_callback = false;
        let mut response_error: Option<String> = None;

        for event_result in SseReader::new(reader) {
            let event = event_result?;
            let raw_event = OttoStreamUpdate {
                event_type: event.event_type.unwrap_or_default(),
                data: parse_otto_stream_update_data(&event.data),
                raw_data: event.data,
            };
            let callback_break = on_event(raw_event.clone()).is_break();

            match classify_otto_stream_terminal(&raw_event) {
                Some(OttoStreamTerminal::Completed) => {
                    saw_terminal_event = true;
                    break;
                }
                Some(OttoStreamTerminal::Interrupted(err)) => {
                    response_error = Some(err);
                    break;
                }
                None if callback_break => {
                    cancelled_by_callback = true;
                    break;
                }
                None => {}
            }
        }

        let (stream_status, stream_error) = if cancelled_by_callback {
            (OttoStreamStatus::Cancelled, None)
        } else if let Some(err) = response_error {
            (OttoStreamStatus::Interrupted, Some(err))
        } else if saw_terminal_event {
            (OttoStreamStatus::Completed, None)
        } else {
            (
                OttoStreamStatus::Interrupted,
                Some(format!("{updates_context} ended before terminal event")),
            )
        };

        Ok(OttoChatResponse {
            message: String::new(),
            thread_id: Some(thread_id.to_string()),
            stream_status,
            stream_error,
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
    /// Starts with 100ms poll interval and backs off to 500ms.
    /// Returns an error if the thread does not stop within ~30 seconds.
    pub fn stop_thread_and_wait(&self, thread_id: &str) -> Result<()> {
        let resp: Value = self.stop_thread(thread_id)?;
        let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "stopping" {
            return Ok(());
        }
        let deadline = Instant::now() + STOP_THREAD_TIMEOUT;
        let mut poll_interval = STOP_THREAD_POLL_INTERVAL_MIN;
        // Poll until the backend confirms the thread is no longer processing.
        while Instant::now() < deadline {
            std::thread::sleep(poll_interval);
            poll_interval = (poll_interval * 2).min(STOP_THREAD_POLL_INTERVAL_MAX);
            let resp: Value = self.stop_thread(thread_id)?;
            let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status != "stopping" {
                return Ok(());
            }
        }
        Err(Error::ApiError {
            status: 408,
            message: format!("thread {thread_id} did not stop within 30 seconds"),
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

    /// Unified request helper for GET, POST, and PATCH.
    fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T> {
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
    }

    fn get_otto_provider_settings(&self) -> Result<OttoProviderSettingsResponse> {
        self.get("/api/v1/otto/provider_settings")
    }
}

enum OttoStreamTerminal {
    Completed,
    Interrupted(String),
}

fn parse_otto_stream_update_data(raw_data: &str) -> Option<Value> {
    if raw_data.is_empty() {
        None
    } else {
        serde_json::from_str(raw_data).ok()
    }
}

fn classify_otto_stream_terminal(event: &OttoStreamUpdate) -> Option<OttoStreamTerminal> {
    match event.event_type.as_str() {
        "thread.done" | "thread.stopped" => Some(OttoStreamTerminal::Completed),
        "thread.details" if is_completed_thread_details_snapshot(event.data.as_ref()) => {
            Some(OttoStreamTerminal::Completed)
        }
        "response.error" => Some(OttoStreamTerminal::Interrupted(extract_otto_stream_error(
            event.data.as_ref(),
        ))),
        _ => None,
    }
}

fn is_completed_thread_details_snapshot(data: Option<&Value>) -> bool {
    data.and_then(|value| value.get("is_processing"))
        .and_then(Value::as_bool)
        == Some(false)
}

fn extract_completed_thread_details_message(data: Option<&Value>) -> String {
    let Some(details) = data else {
        return String::new();
    };
    let Some(messages) = details.get("messages") else {
        return String::new();
    };

    if let (Some(messages_by_id), Some(latest_message_id)) = (
        messages.as_object(),
        details.get("latest_message_id").and_then(Value::as_str),
    ) && let Some(message) = messages_by_id.get(latest_message_id)
        && let Some(text) = extract_assistant_message_text(message)
    {
        return text;
    }

    match messages {
        Value::Array(messages_list) => messages_list
            .iter()
            .rev()
            .find_map(extract_assistant_message_text)
            .unwrap_or_default(),
        Value::Object(messages_by_id) => {
            let mut latest_text = String::new();
            let mut latest_created_at: Option<&str> = None;
            for message in messages_by_id.values() {
                let Some(text) = extract_assistant_message_text(message) else {
                    continue;
                };
                let created_at = message.get("_created_at").and_then(Value::as_str);
                if latest_text.is_empty() || created_at > latest_created_at {
                    latest_created_at = created_at;
                    latest_text = text;
                }
            }
            latest_text
        }
        _ => String::new(),
    }
}

fn extract_assistant_message_text(message: &Value) -> Option<String> {
    if message.get("role").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    let text = Conversation::extract_message_text(message);
    if text.is_empty() { None } else { Some(text) }
}

fn extract_otto_stream_error(data: Option<&Value>) -> String {
    data.and_then(|value| {
        value
            .get("error")
            .or_else(|| value.get("message"))
            .or_else(|| value.get("detail"))
            .and_then(|field| field.as_str())
            .map(String::from)
    })
    .unwrap_or_else(|| "response error".to_string())
}

fn raw_otto_stream_update_to_stream_event(event: &OttoStreamUpdate) -> Option<StreamEvent> {
    let data = event.data.as_ref()?;
    match event.event_type.as_str() {
        "response.output_text.delta" => data
            .get("delta")
            .and_then(|v| v.as_str())
            .map(|d| StreamEvent::TextDelta(d.to_string())),
        "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
            Some(StreamEvent::ReasoningDelta {
                item_id: data
                    .get("item_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                delta: data
                    .get("delta")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
        }
        "response.output_item.added" => {
            let item = data.get("item")?;
            if item.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                Some(StreamEvent::ToolCallStart {
                    item_id: item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
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
        "response.function_call_arguments.delta" => Some(StreamEvent::ToolCallArgsDelta {
            item_id: data
                .get("item_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            delta: data
                .get("delta")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        }),
        "response.run_item_stream_event.tool_call_output_item" => {
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
    }
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

fn merge_otto_provider_thinking_levels(
    providers: &mut [OttoProvider],
    settings: OttoProviderSettingsResponse,
) {
    let mut thinking_levels_by_model: HashMap<(String, String), Vec<String>> = settings
        .models
        .into_iter()
        .filter_map(|model| {
            model
                .thinking_levels
                .map(|thinking_levels| ((model.provider_id, model.id), thinking_levels))
        })
        .collect();

    for provider in providers {
        for model in &mut provider.models {
            model.thinking_levels =
                thinking_levels_by_model.remove(&(provider.id.clone(), model.id.clone()));
        }
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
