use std::collections::VecDeque;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{OttoChatRequest, OttoModel, StreamEvent};
use clap::Subcommand;

use crate::common::{OutputMode, print_json, print_subcommand_help, print_table};

// ---------------------------------------------------------------------------
// StreamRenderer — spinner + smoothed character-by-character output
// ---------------------------------------------------------------------------

enum RenderMsg {
    Delta(String),
    Done,
}

/// Renders Otto's streaming response with a spinner while waiting and
/// smoothed character-by-character output once text starts flowing.
struct StreamRenderer {
    tx: Option<mpsc::Sender<RenderMsg>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Target characters per second for smoothed output (~200 cps).
/// Fast enough to not feel laggy, slow enough to look smooth.
const CHAR_DELAY: Duration = Duration::from_millis(5);

impl StreamRenderer {
    /// Start the renderer. `prefix` is printed once before the first character
    /// (e.g. `"otto> "` in TUI mode, `""` in run mode).
    fn start(prefix: &str) -> Self {
        let (tx, rx) = mpsc::channel::<RenderMsg>();
        let prefix = prefix.to_string();
        let handle = std::thread::spawn(move || {
            Self::render_loop(rx, &prefix);
        });
        Self {
            tx: Some(tx),
            handle: Some(handle),
        }
    }

    fn send_delta(&self, text: String) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(RenderMsg::Delta(text));
        }
    }

    fn finish(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(RenderMsg::Done);
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    fn render_loop(rx: mpsc::Receiver<RenderMsg>, prefix: &str) {
        let mut stderr = std::io::stderr();
        let mut stdout = std::io::stdout();

        // Phase 1: Spinner — animate until the first Delta arrives
        let mut buf: VecDeque<char> = VecDeque::new();
        let mut frame = 0usize;
        loop {
            match rx.recv_timeout(Duration::from_millis(80)) {
                Ok(RenderMsg::Delta(text)) => {
                    // Clear spinner, print prefix, buffer the text
                    let _ = write!(stderr, "\r\x1b[2K{prefix}");
                    let _ = stderr.flush();
                    buf.extend(text.chars());
                    break;
                }
                Ok(RenderMsg::Done) => {
                    let _ = write!(stderr, "\r\x1b[2K");
                    let _ = stderr.flush();
                    return;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let _ = write!(
                        stderr,
                        "\r{} Ascending...",
                        SPINNER_FRAMES[frame % SPINNER_FRAMES.len()]
                    );
                    let _ = stderr.flush();
                    frame += 1;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = write!(stderr, "\r\x1b[2K");
                    let _ = stderr.flush();
                    return;
                }
            }
        }

        // Phase 2: Smoothed character output
        loop {
            // Drain any pending deltas into buf
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    RenderMsg::Delta(text) => buf.extend(text.chars()),
                    RenderMsg::Done => {
                        // Flush remaining buffer immediately
                        for ch in &buf {
                            let _ = write!(stdout, "{ch}");
                        }
                        let _ = stdout.flush();
                        return;
                    }
                }
            }

            if buf.is_empty() {
                // Buffer empty — block until next message
                match rx.recv() {
                    Ok(RenderMsg::Delta(text)) => buf.extend(text.chars()),
                    Ok(RenderMsg::Done) | Err(_) => return,
                }
                continue;
            }

            // Print one character and sleep
            let ch = buf.pop_front().unwrap();
            let _ = write!(stdout, "{ch}");
            let _ = stdout.flush();

            // Adaptive speed: if buffer is large, go faster
            if buf.len() > 200 {
                // Way behind — flush in bulk
                let n = buf.len().min(100);
                let bulk: String = buf.drain(..n).collect();
                let _ = write!(stdout, "{bulk}");
                let _ = stdout.flush();
            } else if buf.len() > 50 {
                // Behind — skip delay
            } else {
                std::thread::sleep(CHAR_DELAY);
            }
        }
    }
}

impl Drop for StreamRenderer {
    fn drop(&mut self) {
        self.finish();
    }
}

// ---------------------------------------------------------------------------
// CLI commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub(crate) enum OttoCommands {
    /// Send a message to Otto
    #[command(arg_required_else_help = true)]
    Run {
        /// Message to send to Otto
        prompt: String,

        /// Workspace to use for context
        #[arg(long)]
        workspace: Option<String>,

        /// Deployment to use for context
        #[arg(long)]
        deployment: Option<String>,

        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,

        /// LLM provider to use (requires --model)
        #[arg(long, requires = "model")]
        provider: Option<String>,

        /// LLM model to use
        #[arg(long)]
        model: Option<String>,

        /// Thread ID to continue a conversation
        #[arg(long)]
        thread: Option<String>,
    },
    /// Manage Otto providers
    Provider {
        #[command(subcommand)]
        command: Option<ProviderCommands>,
    },
    /// Manage Otto models
    Model {
        #[command(subcommand)]
        command: Option<ModelCommands>,
    },
}

#[derive(Subcommand)]
pub(crate) enum ProviderCommands {
    /// List available providers
    List,
}

#[derive(Subcommand)]
pub(crate) enum ModelCommands {
    /// List available models
    List {
        /// Filter by provider ID (e.g. ascend_managed_bedrock, openai)
        #[arg(long)]
        provider: Option<String>,
    },
}

pub(crate) fn handle_otto_cmd(
    client: &AscendClient,
    cmd: Option<OttoCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        return print_subcommand_help("otto");
    };
    match cmd {
        OttoCommands::Run {
            prompt,
            workspace,
            deployment,
            uuid,
            provider,
            model,
            thread,
        } => {
            let runtime_uuid = client.resolve_optional_runtime_target(
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;

            let request = OttoChatRequest {
                prompt,
                runtime_uuid,
                thread_id: thread,
                model: OttoModel::from_options(provider.as_deref(), model.as_deref()),
            };

            match output {
                OutputMode::Json => {
                    let response = client.otto(&request)?;
                    print_json(&serde_json::json!({
                        "message": response.message,
                        "thread_id": response.thread_id,
                    }))?;
                }
                OutputMode::Text => {
                    let mut renderer = StreamRenderer::start("");
                    let mut thread_id = None;
                    client.otto_streaming(
                        &request,
                        |event| {
                            if let StreamEvent::TextDelta(delta) = event {
                                renderer.send_delta(delta);
                            }
                            std::ops::ControlFlow::Continue(())
                        },
                        |tid| {
                            thread_id = Some(tid.to_string());
                        },
                    )?;
                    renderer.finish();
                    println!();
                    if let Some(tid) = &thread_id {
                        eprintln!("thread: {tid}");
                    }
                }
            }
            Ok(())
        }
        OttoCommands::Provider { command } => {
            let Some(command) = command else {
                return print_subcommand_help("otto provider");
            };
            match command {
                ProviderCommands::List => {
                    let providers = client.list_otto_providers()?;
                    match output {
                        OutputMode::Json => print_json(&providers)?,
                        OutputMode::Text => {
                            let rows: Vec<Vec<String>> = providers
                                .iter()
                                .map(|p| {
                                    vec![
                                        p.name.clone(),
                                        p.id.clone(),
                                        p.default_model.clone(),
                                        p.models.len().to_string(),
                                    ]
                                })
                                .collect();
                            print_table(&["NAME", "ID", "DEFAULT MODEL", "MODELS"], &rows);
                        }
                    }
                }
            }
            Ok(())
        }
        OttoCommands::Model { command } => {
            let Some(command) = command else {
                return print_subcommand_help("otto model");
            };
            match command {
                ModelCommands::List { provider: filter } => {
                    let providers = client.list_otto_providers()?;
                    let filtered: Vec<_> = if let Some(ref id) = filter {
                        providers.into_iter().filter(|p| p.id == *id).collect()
                    } else {
                        providers
                    };
                    if filtered.is_empty() {
                        if let Some(id) = filter {
                            anyhow::bail!("no provider found with id '{id}'");
                        }
                        eprintln!("No results.");
                        return Ok(());
                    }
                    match output {
                        OutputMode::Json => {
                            let models: Vec<_> = filtered
                                .iter()
                                .flat_map(|p| {
                                    p.models.iter().map(move |m| {
                                        serde_json::json!({
                                            "provider": p.id,
                                            "model": m.id,
                                            "name": m.name,
                                        })
                                    })
                                })
                                .collect();
                            print_json(&models)?;
                        }
                        OutputMode::Text => {
                            let rows: Vec<Vec<String>> = filtered
                                .iter()
                                .flat_map(|p| {
                                    p.models.iter().map(move |m| {
                                        vec![m.id.clone(), p.id.clone(), m.name.clone()]
                                    })
                                })
                                .collect();
                            print_table(&["ID", "PROVIDER", "NAME"], &rows);
                        }
                    }
                }
            }
            Ok(())
        }
    }
}
