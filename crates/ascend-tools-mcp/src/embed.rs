//! Embeddable MCP server: use per-request Bearer token and expose a request handler for FastAPI.

use std::cell::RefCell;
use std::sync::OnceLock;

use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::Result as CoreResult;
use axum::body::Body;
use axum::http::Request;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use server::AscendMcpServer;

thread_local! {
    /// Bearer token for the current request (set by handle_request, read by the session factory).
    static REQUEST_BEARER_TOKEN: RefCell<Option<String>> = RefCell::new(None);
}

/// Set the Bearer token for the current request (used when creating a new MCP session).
pub(crate) fn set_request_bearer_token(token: Option<String>) {
    REQUEST_BEARER_TOKEN.with(|cell| *cell.borrow_mut() = token);
}

/// Read and take the Bearer token for the current request.
pub(crate) fn take_request_bearer_token() -> Option<String> {
    REQUEST_BEARER_TOKEN.with(|cell| cell.borrow_mut().take())
}

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static ROUTER: OnceLock<std::sync::Mutex<axum::Router>> = OnceLock::new();

/// Initialize the embeddable MCP handler. Call once before handle_mcp_request (e.g. at app startup).
pub fn init_mcp_embed(instance_api_url: String) {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().expect("create tokio runtime for MCP embed")
    });
    let _ = ROUTER.set(std::sync::Mutex::new(mcp_router(instance_api_url, None)));
}

/// Handle one MCP HTTP request. Authorization header is used as the Bearer token for this request.
/// Returns (status_code, headers_vec, body_bytes). Call init_mcp_embed first.
pub fn handle_mcp_request(
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(u16, Vec<(String, String)>, Vec<u8>), String> {
    let rt = RUNTIME.get().ok_or("MCP embed not initialized (call init_mcp_embed first)")?;
    let router_guard = ROUTER
        .get()
        .ok_or("MCP embed not initialized")?
        .lock()
        .map_err(|e| e.to_string())?;
    let router = router_guard.clone();
    drop(router_guard);

    let token = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .and_then(|(_, v)| v.strip_prefix("Bearer ").map(String::from));
    set_request_bearer_token(token);

    let mut req_builder = Request::builder().method(method).uri(path);
    for (k, v) in headers {
        if let (Ok(name), Ok(value)) = (
            axum::http::header::HeaderName::try_from(k.as_str()),
            axum::http::header::HeaderValue::try_from(v.as_str()),
        ) {
            req_builder = req_builder.header(name, value);
        }
    }
    let req = req_builder
        .body(Body::from(body.to_vec()))
        .map_err(|e| e.to_string())?;

    let response = rt.block_on(router.call(req)).map_err(|e| e.to_string())?;

    let status = response.status().as_u16();
    let resp_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();
    let body_future = axum::body::to_bytes(response.into_body());
    let body_bytes = rt
        .block_on(body_future)
        .map_err(|e| e.to_string())?
        .to_vec();

    Ok((status, resp_headers, body_bytes))
}

/// Build the StreamableHttpService with a factory that uses the current request's Bearer token.
fn streamable_http_service(
    instance_api_url: String,
    fallback_config: Option<Config>,
) -> StreamableHttpService<
    impl Fn() -> std::result::Result<AscendMcpServer, rmcp::ErrorData> + Send + Clone + 'static,
    LocalSessionManager,
> {
    let factory = move || {
        let token = take_request_bearer_token();
        if let Some(t) = token {
            match AscendClient::from_instance_token(instance_api_url.clone(), t) {
                Ok(client) => Ok(AscendMcpServer::new(client)),
                Err(e) => Ok(AscendMcpServer::with_client_init_error(format!("{e:#}"))),
            }
        } else if let Some(ref config) = fallback_config {
            match AscendClient::new(config.clone()) {
                Ok(client) => Ok(AscendMcpServer::new(client)),
                Err(e) => Ok(AscendMcpServer::with_client_init_error(format!("{e:#}"))),
            }
        } else {
            Ok(AscendMcpServer::with_client_init_error(
                "no Authorization Bearer token and no fallback config".to_string(),
            ))
        }
    };

    StreamableHttpService::new(
        factory,
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    )
}

/// Creates an axum Router that serves MCP at `/mcp` and uses the request's Bearer token per session.
/// Can be mounted on an existing FastAPI/axum app. Optional `fallback_config` is used when no
/// Authorization header is present (e.g. legacy standalone mode).
pub fn mcp_router(
    instance_api_url: String,
    fallback_config: Option<CoreResult<Config>>,
) -> axum::Router {
    let (instance_api_url, fallback) = match fallback_config {
        Some(Ok(c)) => (instance_api_url, Some(c)),
        Some(Err(_)) | None => (instance_api_url, None),
    };
    let service = streamable_http_service(instance_api_url, fallback);
    axum::Router::new().nest_service("/mcp", service)
}
