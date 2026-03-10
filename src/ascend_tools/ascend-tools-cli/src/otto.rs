use std::collections::VecDeque;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{OttoChatRequest, OttoModel};
use clap::Subcommand;

use crate::common::{OutputMode, print_json, print_table, resolve_runtime_target};

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

        /// LLM provider to use
        #[arg(long)]
        provider: Option<String>,

        /// LLM model to use
        #[arg(long)]
        model: Option<String>,

        /// Thread ID to continue a conversation
        #[arg(long)]
        thread: Option<String>,
    },
    /// Manage Otto providers
    Providers {
        #[command(subcommand)]
        command: Option<ProvidersCommands>,
    },
    /// Manage Otto models
    Models {
        #[command(subcommand)]
        command: Option<ModelsCommands>,
    },
    /// Interactive multi-turn conversation with Otto (Ctrl+C to exit)
    Tui {
        /// Workspace to use for context
        #[arg(long)]
        workspace: Option<String>,

        /// Deployment to use for context
        #[arg(long)]
        deployment: Option<String>,

        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,

        /// LLM provider to use
        #[arg(long)]
        provider: Option<String>,

        /// LLM model to use
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum ProvidersCommands {
    /// List available providers
    List,
}

#[derive(Subcommand)]
pub(crate) enum ModelsCommands {
    /// List available models (optionally for a specific provider)
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
        use clap::CommandFactory;
        crate::cli::CliParser::command()
            .find_subcommand_mut("otto")
            .expect("otto subcommand exists")
            .print_help()?;
        return Ok(());
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
            let runtime_id = if workspace.is_some() || deployment.is_some() || uuid.is_some() {
                Some(resolve_runtime_target(
                    client,
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )?)
            } else {
                None
            };

            let otto_model = OttoModel::from_options(provider.as_deref(), model.as_deref());

            let request = OttoChatRequest {
                prompt,
                runtime_id,
                thread_id: thread,
                model: otto_model,
            };

            match output {
                OutputMode::Json => {
                    let response = client.otto_chat(&request)?;
                    print_json(&serde_json::json!({
                        "message": response.message,
                        "thread_id": response.thread_id,
                    }))?;
                }
                OutputMode::Text => {
                    let mut renderer = StreamRenderer::start("");
                    let response = client.otto_chat_streaming(&request, |delta| {
                        renderer.send_delta(delta.to_string());
                    })?;
                    renderer.finish();
                    println!();
                    if let Some(tid) = &response.thread_id {
                        eprintln!("thread: {tid}");
                    }
                }
            }
            Ok(())
        }
        OttoCommands::Providers { command } => {
            let command = command.unwrap_or(ProvidersCommands::List);
            match command {
                ProvidersCommands::List => {
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
                            print_table(&["NAME", "PROVIDER ID", "DEFAULT MODEL", "MODELS"], &rows);
                        }
                    }
                }
            }
            Ok(())
        }
        OttoCommands::Models { command } => {
            let command = command.unwrap_or(ModelsCommands::List { provider: None });
            match command {
                ModelsCommands::List { provider: filter } => {
                    let providers = client.list_otto_providers()?;
                    let filtered: Vec<_> = if let Some(ref id) = filter {
                        providers.into_iter().filter(|p| p.id == *id).collect()
                    } else {
                        providers
                    };
                    if filtered.is_empty() {
                        if let Some(id) = filter {
                            anyhow::bail!("No provider found with id '{id}'");
                        }
                        eprintln!("No providers configured.");
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
                                        vec![m.id.clone(), p.name.clone(), m.name.clone()]
                                    })
                                })
                                .collect();
                            print_table(&["MODEL ID", "PROVIDER", "NAME"], &rows);
                        }
                    }
                }
            }
            Ok(())
        }
        OttoCommands::Tui {
            workspace,
            deployment,
            uuid,
            provider,
            model,
        } => {
            let runtime_id = if workspace.is_some() || deployment.is_some() || uuid.is_some() {
                Some(resolve_runtime_target(
                    client,
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )?)
            } else {
                None
            };
            let otto_model = OttoModel::from_options(provider.as_deref(), model.as_deref());
            let mut thread_id: Option<String> = None;

            // Reset SIGINT to default (SIG_DFL) so Ctrl+C terminates immediately
            // instead of being caught by Python's signal handler (which can't run
            // while we're blocked in Rust code).
            #[cfg(unix)]
            #[allow(unsafe_code)]
            unsafe {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
            }

            eprintln!("Otto chat (Ctrl+C to exit)\n");

            loop {
                eprint!("you> ");
                let mut input = String::new();
                match std::io::stdin().read_line(&mut input) {
                    Ok(0) => break,                                                 // EOF
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => break, // Ctrl+C
                    Err(e) => return Err(e.into()),
                    Ok(_) => {}
                }
                let prompt = input.trim();
                if prompt.is_empty() {
                    continue;
                }

                eprintln!();
                let mut renderer = StreamRenderer::start("otto> ");
                let response = client.otto_chat_streaming(
                    &OttoChatRequest {
                        prompt: prompt.to_string(),
                        runtime_id: runtime_id.clone(),
                        thread_id: thread_id.clone(),
                        model: otto_model.clone(),
                    },
                    |delta| {
                        renderer.send_delta(delta.to_string());
                    },
                )?;
                renderer.finish();
                println!("\n");

                if let Some(tid) = &response.thread_id {
                    thread_id = Some(tid.clone());
                }
            }
            Ok(())
        }
    }
}
