mod params;
mod server;

use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use server::AscendMcpServer;
use tracing_subscriber::EnvFilter;

/// Reset SIGINT to the default handler. When embedded in Python (via PyO3),
/// Python's SIGINT handler swallows the signal instead of propagating it,
/// which prevents tokio's ctrl_c() from ever firing.
fn reset_sigint() {
    // SAFETY: Setting SIGINT to the default disposition (SIG_DFL) is always safe.
    // This is needed because Python's SIGINT handler (installed by PyO3) swallows
    // the signal, preventing tokio's ctrl_c() from firing.
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_DFL);
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}

pub async fn run_stdio(config: Result<Config>) -> Result<()> {
    reset_sigint();
    init_tracing();
    tracing::info!("Starting Ascend MCP server (stdio)");

    let server = match config {
        Ok(config) => match AscendClient::new(config) {
            Ok(client) => AscendMcpServer::new(client),
            Err(e) => {
                tracing::error!("Ascend client initialization failed: {e:#}");
                AscendMcpServer::with_client_init_error(format!("{e:#}"))
            }
        },
        Err(e) => {
            tracing::error!("Ascend config resolution failed: {e:#}");
            AscendMcpServer::with_client_init_error(format!("{e:#}"))
        }
    };

    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {e:?}"))?;

    tokio::select! {
        result = service.waiting() => { result?; }
        _ = tokio::signal::ctrl_c() => {}
    }
    Ok(())
}

pub async fn run_http(config: Result<Config>, bind_addr: &str) -> Result<()> {
    reset_sigint();
    init_tracing();
    tracing::info!("Starting Ascend MCP server (HTTP) on {bind_addr}");

    let ct = tokio_util::sync::CancellationToken::new();
    let (config, config_error) = match config {
        Ok(config) => (Some(config), None),
        Err(e) => {
            let message = format!("{e:#}");
            tracing::error!("Ascend config resolution failed: {message}");
            (None, Some(message))
        }
    };
    let client_init_error: std::sync::Arc<std::sync::Mutex<Option<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));

    let service = StreamableHttpService::new(
        {
            let client_init_error = client_init_error.clone();
            let config_error = config_error.clone();
            move || {
                let Some(config) = config.clone() else {
                    return Ok(AscendMcpServer::with_client_init_error(
                        config_error
                            .clone()
                            .unwrap_or_else(|| "missing config".to_string()),
                    ));
                };
                match AscendClient::new(config) {
                    Ok(client) => Ok(AscendMcpServer::new(client)),
                    Err(e) => {
                        let message = format!("{e:#}");
                        if let Ok(mut guard) = client_init_error.lock() {
                            if guard.as_deref() != Some(message.as_str()) {
                                tracing::error!("Ascend client initialization failed: {message}");
                                *guard = Some(message.clone());
                            }
                        }
                        Ok(AscendMcpServer::with_client_init_error(message))
                    }
                }
            }
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            cancellation_token: ct.child_token(),
            ..Default::default()
        },
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let tcp_listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!("Listening on {bind_addr}");

    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            ct.cancel();
        })
        .await?;

    Ok(())
}
