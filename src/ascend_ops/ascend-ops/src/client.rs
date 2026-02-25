use anyhow::{Context, Result, bail};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde_json::Value;
use std::time::Duration;
use ureq::Agent;

use crate::auth::Auth;
use crate::config::Config;
use crate::models::*;

const TIMEOUT: Duration = Duration::from_secs(30);

const PATH_SEGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'#').add(b'%').add(b'/').add(b'?');

fn encode_path(s: &str) -> String {
    utf8_percent_encode(s, PATH_SEGMENT).to_string()
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
        let agent = Agent::new_with_config(
            ureq::config::Config::builder()
                .tls_config(
                    ureq::tls::TlsConfig::builder()
                        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                        .build(),
                )
                .http_status_as_error(false)
                .timeout_global(Some(TIMEOUT))
                .user_agent(concat!("ascend-ops/", env!("CARGO_PKG_VERSION")))
                .build(),
        );
        Ok(Self {
            agent,
            instance_api_url: config.instance_api_url,
            auth,
        })
    }

    // -- Runtimes --

    pub fn list_runtimes(&self, filters: RuntimeFilters) -> Result<Vec<Runtime>> {
        let mut params = Vec::new();
        if let Some(ref id) = filters.id {
            params.push(format!("id={}", encode_path(id)));
        }
        if let Some(ref kind) = filters.kind {
            params.push(format!("kind={}", encode_path(kind)));
        }
        if let Some(ref p) = filters.project_uuid {
            params.push(format!("project_uuid={}", encode_path(p)));
        }
        if let Some(ref e) = filters.environment_uuid {
            params.push(format!("environment_uuid={}", encode_path(e)));
        }
        let qs = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };
        self.get(&format!("/api/v1/runtimes{qs}"))
    }

    pub fn get_runtime(&self, uuid: &str) -> Result<Runtime> {
        self.get(&format!("/api/v1/runtimes/{}", encode_path(uuid)))
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
    ) -> Result<FlowRunTrigger> {
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
        let mut params = vec![format!("runtime_uuid={}", encode_path(runtime_uuid))];
        if let Some(ref s) = filters.status {
            params.push(format!("status={}", encode_path(s)));
        }
        if let Some(ref f) = filters.flow {
            params.push(format!("flow={}", encode_path(f)));
        }
        if let Some(ref s) = filters.since {
            params.push(format!("since={}", encode_path(s)));
        }
        if let Some(ref u) = filters.until {
            params.push(format!("until={}", encode_path(u)));
        }
        if let Some(o) = filters.offset {
            params.push(format!("offset={o}"));
        }
        if let Some(l) = filters.limit {
            params.push(format!("limit={l}"));
        }
        let qs = format!("?{}", params.join("&"));
        self.get(&format!("/api/v1/flow-runs{qs}"))
    }

    pub fn get_flow_run(&self, runtime_uuid: &str, name: &str) -> Result<FlowRun> {
        self.get(&format!(
            "/api/v1/flow-runs/{}?runtime_uuid={}",
            encode_path(name),
            encode_path(runtime_uuid)
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

    fn post_json<T: serde::de::DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        let token = self.auth.get_token()?;
        let url = format!("{}{path}", self.instance_api_url);
        let resp = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .send(serde_json::to_string(body)?.as_bytes())
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
