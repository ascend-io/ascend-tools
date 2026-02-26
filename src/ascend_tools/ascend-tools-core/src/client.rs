use anyhow::{Context, Result, bail};
use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;
use ureq::Agent;

use crate::auth::Auth;
use crate::config::Config;
use crate::models::*;

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
        let auth = Auth::new(
            config.service_account_id,
            &config.service_account_key,
            config.instance_api_url.clone(),
        )?;
        Ok(Self {
            agent: crate::new_agent(),
            instance_api_url: config.instance_api_url,
            auth,
        })
    }

    // -- Runtimes --

    pub fn list_runtimes(&self, filters: RuntimeFilters) -> Result<Vec<Runtime>> {
        let mut qs = QueryString::new();
        qs.push_opt("id", filters.id.as_deref());
        qs.push_opt("kind", filters.kind.as_deref());
        qs.push_opt("project_uuid", filters.project_uuid.as_deref());
        qs.push_opt("environment_uuid", filters.environment_uuid.as_deref());
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
        wakeup: bool,
    ) -> Result<FlowRunTrigger> {
        if wakeup {
            self.resume_runtime(runtime_uuid)?;
        } else {
            let runtime = self.get_runtime(runtime_uuid)?;
            if runtime.paused {
                bail!(
                    "Runtime is paused. Use --resume (CLI) or resume=True (SDK) to resume before running."
                );
            }
            match runtime.health.as_deref() {
                Some("running") => {}
                Some("starting") => {
                    bail!("Runtime is starting, not yet ready to accept flow runs.")
                }
                Some("error") => bail!("Runtime is in error state and cannot run flows."),
                Some(other) => bail!("Runtime health is '{other}', expected 'running'."),
                None => bail!("Runtime has no health status. It may be initializing."),
            }
        }
        let body = serde_json::json!({ "spec": spec });
        self.post_json(
            &format!(
                "/api/v1/runtimes/{}/flows/{}:run",
                encode_path(runtime_uuid),
                encode_path(flow_name)
            ),
            &body,
        )
    }

    // -- Flow runs --

    pub fn list_flow_runs(
        &self,
        runtime_uuid: &str,
        filters: FlowRunFilters,
    ) -> Result<Vec<FlowRun>> {
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

    // -- HTTP helpers --

    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let resp = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .call()
            .context(format!("GET {path}"))?;
        handle_response(resp)
    }

    fn post_empty<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let resp = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .send_empty()
            .context(format!("POST {path}"))?;
        handle_response(resp)
    }

    fn post_json<T: serde::de::DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let json_body = serde_json::to_string(body)?;
        let resp = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .send(json_body.as_bytes())
            .context(format!("POST {path}"))?;
        handle_response(resp)
    }
}

fn handle_response<T: serde::de::DeserializeOwned>(
    mut resp: ureq::http::Response<ureq::Body>,
) -> Result<T> {
    let status = resp.status().as_u16();
    let body: String = resp.body_mut().read_to_string()?;

    if !(200..300).contains(&status) {
        // Try to extract error message from JSON response
        if let Ok(json) = serde_json::from_str::<Value>(&body) {
            if let Some(detail) = json.get("detail").and_then(|v| v.as_str()) {
                bail!("API error (HTTP {status}): {detail}");
            }
        }
        bail!("API error (HTTP {status}): {body}");
    }

    serde_json::from_str(&body).context("failed to parse API response")
}

impl std::fmt::Debug for AscendClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AscendClient")
            .field("instance_api_url", &self.instance_api_url)
            .field("auth", &self.auth)
            .finish()
    }
}
