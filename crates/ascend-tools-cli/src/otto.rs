use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{OttoChatRequest, OttoStreamStatus, StreamEvent};
use clap::{Subcommand, ValueEnum};
use serde::Serialize;

use crate::common::{OutputMode, print_json, print_subcommand_help, print_table};

// ---------------------------------------------------------------------------
// StreamRenderer — spinner + smoothed character-by-character output
// ---------------------------------------------------------------------------

enum RenderMsg {
    Delta(String),
    Done,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum ThinkingLevelArg {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Max,
}

impl ThinkingLevelArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }
}

#[derive(Serialize)]
struct OttoJsonlRequestRecord {
    record_type: &'static str,
    base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_thread_id: Option<String>,
    request_body: serde_json::Value,
}

#[derive(Serialize)]
struct OttoJsonlEventRecord {
    record_type: &'static str,
    thread_id: String,
    sequence: u64,
    event_type: String,
    raw_data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct OttoJsonlTerminalRecord {
    record_type: &'static str,
    thread_id: String,
    stream_status: OttoStreamStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_error: Option<String>,
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

        /// LLM provider name (requires --model)
        #[arg(long, requires = "model")]
        provider: Option<String>,

        /// LLM model to use
        #[arg(long)]
        model: Option<String>,

        /// Explicit thinking level for reasoning-capable models
        #[arg(long, value_enum)]
        thinking: Option<ThinkingLevelArg>,

        /// Print reasoning and tool stream events to stderr
        #[arg(long)]
        debug_stream: bool,

        /// Emit request provenance and raw Otto stream updates as JSONL
        #[arg(long)]
        jsonl: bool,

        /// Thread ID to continue a conversation (hidden, use --conversation instead)
        #[arg(long, hide = true)]
        thread: Option<String>,

        /// Continue an existing conversation (by title or ID)
        #[arg(long, conflicts_with_all = ["thread", "resume"])]
        conversation: Option<String>,

        /// Resume the most recent conversation
        #[arg(long, conflicts_with_all = ["thread", "conversation"])]
        resume: bool,
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
    /// Manage Otto conversations
    #[command(long_about = "Manage Otto conversations.\n\n\
            Examples:\n  \
            ascend-tools otto conversation list\n  \
            ascend-tools otto conversation list --limit 10\n  \
            ascend-tools otto conversation get \"My conversation title\"\n  \
            ascend-tools otto conversation get abc123 --id")]
    Conversation {
        #[command(subcommand)]
        command: Option<crate::conversation::ConversationCommands>,
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

        /// LLM provider to use (requires --model)
        #[arg(long, requires = "model")]
        provider: Option<String>,

        /// LLM model to use
        #[arg(long)]
        model: Option<String>,

        /// Continue an existing conversation (by title or ID)
        #[arg(long, conflicts_with = "resume")]
        conversation: Option<String>,

        /// Resume the most recent conversation
        #[arg(long, conflicts_with = "conversation")]
        resume: bool,
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
        /// Filter by provider name (e.g. "OpenAI", "Ascend Managed Bedrock")
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
            thinking,
            debug_stream,
            jsonl,
            thread,
            conversation,
            resume,
        } => {
            if jsonl && *output != OutputMode::Text {
                anyhow::bail!("--jsonl cannot be combined with -o json");
            }
            let runtime_uuid = client.resolve_optional_runtime_target(
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;

            let thread_id = crate::conversation::resolve_conversation_flag(
                client,
                thread,
                conversation,
                resume,
            )?;

            let request = OttoChatRequest {
                prompt,
                runtime_uuid,
                thread_id,
                model: client.resolve_otto_model(provider.as_deref(), model.as_deref())?,
                thinking: thinking.map(|level| level.as_str().to_string()),
            };

            if jsonl {
                let request_body = serde_json::to_value(&request)?;
                let request_record = OttoJsonlRequestRecord {
                    record_type: "request",
                    base_url: client.instance_api_url().to_string(),
                    binary_path: std::env::current_exe()
                        .ok()
                        .map(|path| path.display().to_string()),
                    provider: provider.clone(),
                    model: request.model.as_ref().map(|model| model.id().to_string()),
                    request_thread_id: request.thread_id.clone(),
                    request_body,
                };
                println!(
                    "{}",
                    serde_json::to_string(&request_record)
                        .expect("otto jsonl request record should always serialize")
                );

                let thread_id = RefCell::new(None::<String>);
                let mut sequence = 0u64;
                let response = client.otto_streaming_events(
                    &request,
                    |event| {
                        sequence += 1;
                        let record = OttoJsonlEventRecord {
                            record_type: "event",
                            thread_id: thread_id
                                .borrow()
                                .clone()
                                .unwrap_or_else(|| "<unknown>".to_string()),
                            sequence,
                            event_type: event.event_type,
                            raw_data: event.raw_data,
                            data: event.data,
                        };
                        println!(
                            "{}",
                            serde_json::to_string(&record)
                                .expect("otto jsonl event record should always serialize")
                        );
                        std::ops::ControlFlow::Continue(())
                    },
                    |tid| {
                        *thread_id.borrow_mut() = Some(tid.to_string());
                    },
                )?;
                let terminal_record = OttoJsonlTerminalRecord {
                    record_type: "terminal",
                    thread_id: response
                        .thread_id
                        .or_else(|| thread_id.borrow().clone())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    stream_status: response.stream_status,
                    stream_error: response.stream_error,
                };
                println!(
                    "{}",
                    serde_json::to_string(&terminal_record)
                        .expect("otto jsonl terminal record should always serialize")
                );
                return Ok(());
            }

            match output {
                OutputMode::Json => {
                    let response = client.otto(&request)?;
                    print_json(&serde_json::json!({
                        "message": response.message,
                        "thread_id": response.thread_id,
                    }))?;
                }
                OutputMode::Text => {
                    let mut renderer = (!debug_stream).then(|| StreamRenderer::start(""));
                    let mut tool_names = HashMap::new();
                    let mut tool_item_names = HashMap::new();
                    let mut thread_id = None;
                    client.otto_streaming(
                        &request,
                        |event| {
                            match event {
                                StreamEvent::TextDelta(delta) => {
                                    if let Some(renderer) = renderer.as_ref() {
                                        renderer.send_delta(delta);
                                    } else {
                                        print!("{delta}");
                                        let _ = std::io::stdout().flush();
                                    }
                                }
                                StreamEvent::ReasoningDelta { item_id: _, delta } => {
                                    if debug_stream {
                                        eprintln!("[reasoning] {delta}");
                                    }
                                }
                                StreamEvent::ToolCallStart {
                                    item_id,
                                    call_id,
                                    name,
                                    arguments,
                                } => {
                                    tool_names.insert(call_id, name.clone());
                                    tool_item_names.insert(item_id, name.clone());
                                    if debug_stream {
                                        eprintln!("[tool start] {name} {arguments}");
                                    }
                                }
                                StreamEvent::ToolCallArgsDelta { item_id, delta } => {
                                    if debug_stream {
                                        let name = tool_item_names
                                            .get(&item_id)
                                            .map(String::as_str)
                                            .unwrap_or("tool");
                                        eprintln!("[tool args] {name} {delta}");
                                    }
                                }
                                StreamEvent::ToolCallOutput { call_id, output } => {
                                    if debug_stream {
                                        let name = tool_names
                                            .get(&call_id)
                                            .map(String::as_str)
                                            .unwrap_or("tool");
                                        eprintln!("[tool output] {name} {output}");
                                    }
                                }
                            }
                            std::ops::ControlFlow::Continue(())
                        },
                        |tid| {
                            thread_id = Some(tid.to_string());
                            if debug_stream {
                                eprintln!("thread: {tid}");
                            }
                        },
                    )?;
                    if let Some(renderer) = renderer.as_mut() {
                        renderer.finish();
                    }
                    println!();
                    if !debug_stream && let Some(tid) = &thread_id {
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
                    let filtered: Vec<_> = if let Some(ref name) = filter {
                        let lower = name.to_lowercase();
                        providers
                            .into_iter()
                            .filter(|p| {
                                p.name.to_lowercase() == lower || p.id.to_lowercase() == lower
                            })
                            .collect()
                    } else {
                        providers
                    };
                    if filtered.is_empty() {
                        if let Some(name) = filter {
                            anyhow::bail!("no provider found matching '{name}'");
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
                                        vec![m.id.clone(), p.name.clone(), m.name.clone()]
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
        OttoCommands::Conversation { command } => {
            crate::conversation::handle_conversation(client, command, output)
        }
        OttoCommands::Tui {
            workspace,
            deployment,
            uuid,
            provider,
            model,
            conversation,
            resume,
        } => {
            let runtime_uuid = client.resolve_optional_runtime_target(
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;
            let otto_model = client.resolve_otto_model(provider.as_deref(), model.as_deref())?;
            let context_label = workspace
                .as_deref()
                .map(|w| format!("workspace:{w}"))
                .or(deployment.as_deref().map(|d| format!("deployment:{d}")));
            let thread_id =
                crate::conversation::resolve_conversation_flag(client, None, conversation, resume)?;
            ascend_tools_tui::run_tui(client, runtime_uuid, otto_model, context_label, thread_id)
        }
    }
}
