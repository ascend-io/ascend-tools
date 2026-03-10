use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{OttoChatRequest, OttoModel};
use clap::Subcommand;

use crate::common::{OutputMode, print_json, print_table};

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

        /// Use UUID instead of title for workspace
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

        /// Use UUID instead of title for workspace
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
            uuid,
            provider,
            model,
            thread,
        } => {
            let runtime_id = if let Some(uuid) = uuid {
                Some(uuid)
            } else if let Some(ws) = workspace {
                Some(client.resolve_runtime_uuid(&ws, "workspace", None)?)
            } else {
                None
            };

            let otto_model = OttoModel::from_options(provider.as_deref(), model.as_deref());

            let response = client.otto_chat(&OttoChatRequest {
                prompt,
                runtime_id,
                thread_id: thread,
                model: otto_model,
            })?;

            match output {
                OutputMode::Json => {
                    print_json(&serde_json::json!({
                        "message": response.message,
                        "thread_id": response.thread_id,
                    }))?;
                }
                OutputMode::Text => {
                    println!("{}", response.message);
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
            uuid,
            provider,
            model,
        } => {
            let runtime_id = if let Some(uuid) = uuid {
                Some(uuid)
            } else if let Some(ws) = workspace {
                Some(client.resolve_runtime_uuid(&ws, "workspace", None)?)
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

                let response = client.otto_chat(&OttoChatRequest {
                    prompt: prompt.to_string(),
                    runtime_id: runtime_id.clone(),
                    thread_id: thread_id.clone(),
                    model: otto_model.clone(),
                })?;

                if let Some(tid) = &response.thread_id {
                    thread_id = Some(tid.clone());
                }

                println!("\notto> {}\n", response.message);
            }
            Ok(())
        }
    }
}
