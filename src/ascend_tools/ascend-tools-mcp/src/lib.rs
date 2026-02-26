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

pub async fn run_stdio(config: Config) -> Result<()> {
    reset_sigint();
    init_tracing();
    tracing::info!("Starting Ascend MCP server (stdio)");

    let client = AscendClient::new(config)?;
    let server = AscendMcpServer::new(client);

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

pub async fn run_http(config: Config, bind_addr: &str) -> Result<()> {
    reset_sigint();
    init_tracing();
    tracing::info!("Starting Ascend MCP server (HTTP) on {bind_addr}");

    let ct = tokio_util::sync::CancellationToken::new();

    let service = StreamableHttpService::new(
        move || {
            let client = AscendClient::new(config.clone())
                .map_err(|e| std::io::Error::other(format!("{e:#}")))?;
            Ok(AscendMcpServer::new(client))
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
