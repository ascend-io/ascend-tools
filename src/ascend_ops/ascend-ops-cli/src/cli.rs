use anyhow::Result;
use ascend_ops::client::AscendClient;
use ascend_ops::config::Config;
use ascend_ops::models::*;
use clap::{Parser, Subcommand, ValueEnum};
use std::ffi::OsString;

#[derive(Parser)]
#[command(name = "ascend-ops", version, about = "CLI for the Ascend REST API")]
struct Cli {
    #[arg(short, long, global = true, value_enum, default_value_t = OutputMode::Text)]
    output: OutputMode,

    #[arg(long, global = true, env = "ASCEND_SERVICE_ACCOUNT_ID")]
    service_account_id: Option<String>,

    #[arg(long, global = true, env = "ASCEND_PRIVATE_KEY")]
    private_key: Option<String>,

    #[arg(long, global = true, env = "ASCEND_CLOUD_API_URL")]
    cloud_api_url: Option<String>,

    #[arg(long, global = true, env = "ASCEND_INSTANCE_API_URL")]
    instance_api_url: Option<String>,

    #[arg(long, global = true, env = "ASCEND_ORG_ID")]
    org_id: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, PartialEq, ValueEnum)]
enum OutputMode {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage runtimes
    Runtime {
        #[command(subcommand)]
        command: Option<RuntimeCommands>,
    },
    /// Manage flows
    Flow {
        #[command(subcommand)]
        command: Option<FlowCommands>,
    },
    /// Manage flow runs
    FlowRun {
        #[command(subcommand)]
        command: Option<FlowRunCommands>,
    },
    /// Manage builds
    Build {
        #[command(subcommand)]
        command: Option<BuildCommands>,
    },
}

#[derive(Subcommand)]
enum RuntimeCommands {
    /// List runtimes
    List {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        project_uuid: Option<String>,
        #[arg(long)]
        environment_uuid: Option<String>,
    },
    /// Get a runtime
    Get {
        /// Runtime UUID
        uuid: String,
    },
}

#[derive(Subcommand)]
enum FlowCommands {
    /// Run a flow
    Run {
        /// Runtime UUID
        runtime_uuid: String,
        /// Flow name
        flow_name: String,
        /// Optional spec as JSON
        #[arg(long)]
        spec: Option<String>,
    },
    /// Backfill a flow
    Backfill {
        /// Runtime UUID
        runtime_uuid: String,
        /// Flow name
        flow_name: String,
        /// Optional spec as JSON
        #[arg(long)]
        spec: Option<String>,
    },
}

#[derive(Subcommand)]
enum FlowRunCommands {
    /// List flow runs
    List {
        #[arg(long, required = true)]
        runtime: String,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        flow: Option<String>,
    },
    /// Get a flow run
    Get {
        /// Flow run name
        name: String,
        #[arg(long, required = true)]
        runtime: String,
    },
}

#[derive(Subcommand)]
enum BuildCommands {
    /// List builds
    List {
        #[arg(long, required = true)]
        runtime: String,
    },
    /// Get a build
    Get {
        /// Build UUID
        uuid: String,
    },
}

pub fn run<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    let Some(command) = cli.command else {
        Cli::parse_from(["ascend-ops", "--help"]);
        unreachable!()
    };

    let config = Config::with_overrides(
        cli.service_account_id.as_deref(),
        cli.private_key.as_deref(),
        cli.cloud_api_url.as_deref(),
        cli.instance_api_url.as_deref(),
        cli.org_id.as_deref(),
    )?;
    let client = AscendClient::new(config)?;

    match command {
        Commands::Runtime { command } => handle_runtime(&client, command, &cli.output),
        Commands::Flow { command } => handle_flow(&client, command, &cli.output),
        Commands::FlowRun { command } => handle_flow_run(&client, command, &cli.output),
        Commands::Build { command } => handle_build(&client, command, &cli.output),
    }
}

fn handle_runtime(
    client: &AscendClient,
    cmd: Option<RuntimeCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        Cli::parse_from(["ascend-ops", "runtime", "--help"]);
        unreachable!()
    };
    match cmd {
        RuntimeCommands::List {
            id,
            kind,
            project_uuid,
            environment_uuid,
        } => {
            let runtimes = client.list_runtimes(RuntimeFilters {
                id,
                kind,
                project_uuid,
                environment_uuid,
            })?;
            match output {
                OutputMode::Json => print_json(&runtimes)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = runtimes
                        .iter()
                        .map(|r| {
                            vec![
                                r.uuid.clone(),
                                r.id.clone(),
                                r.title.clone(),
                                r.kind.clone(),
                                r.health.clone().unwrap_or_else(|| "-".into()),
                            ]
                        })
                        .collect();
                    print_table(&["UUID", "ID", "TITLE", "KIND", "HEALTH"], &rows);
                }
            }
        }
        RuntimeCommands::Get { uuid } => {
            let r = client.get_runtime(&uuid)?;
            match output {
                OutputMode::Json => print_json(&r)?,
                OutputMode::Text => {
                    println!("UUID:         {}", r.uuid);
                    println!("ID:           {}", r.id);
                    println!("Title:        {}", r.title);
                    println!("Kind:         {}", r.kind);
                    println!("Health:       {}", r.health.unwrap_or_else(|| "-".into()));
                    println!("Project:      {}", r.project_uuid);
                    println!("Environment:  {}", r.environment_uuid);
                    println!(
                        "Build:        {}",
                        r.build_uuid.unwrap_or_else(|| "-".into())
                    );
                    println!("Created:      {}", r.created_at);
                    println!("Updated:      {}", r.updated_at);
                }
            }
        }
    }
    Ok(())
}

fn handle_flow(
    client: &AscendClient,
    cmd: Option<FlowCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        Cli::parse_from(["ascend-ops", "flow", "--help"]);
        unreachable!()
    };
    match cmd {
        FlowCommands::Run {
            runtime_uuid,
            flow_name,
            spec,
        } => {
            let spec_value = parse_spec(spec)?;
            let trigger = client.run_flow(&runtime_uuid, &flow_name, spec_value)?;
            match output {
                OutputMode::Json => print_json(&trigger)?,
                OutputMode::Text => {
                    println!("{}", trigger.event_uuid);
                }
            }
        }
        FlowCommands::Backfill {
            runtime_uuid,
            flow_name,
            spec,
        } => {
            let spec_value = parse_spec(spec)?;
            let trigger = client.backfill_flow(&runtime_uuid, &flow_name, spec_value)?;
            match output {
                OutputMode::Json => print_json(&trigger)?,
                OutputMode::Text => {
                    println!("{}", trigger.event_uuid);
                }
            }
        }
    }
    Ok(())
}

fn handle_flow_run(
    client: &AscendClient,
    cmd: Option<FlowRunCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        Cli::parse_from(["ascend-ops", "flow-run", "--help"]);
        unreachable!()
    };
    match cmd {
        FlowRunCommands::List {
            runtime,
            status,
            flow,
        } => {
            let runs = client.list_flow_runs(
                &runtime,
                FlowRunFilters {
                    status,
                    flow,
                    ..Default::default()
                },
            )?;
            match output {
                OutputMode::Json => print_json(&runs)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = runs
                        .iter()
                        .map(|r| {
                            vec![
                                r.name.clone(),
                                r.flow.clone(),
                                r.status.clone(),
                                r.created_at.clone(),
                            ]
                        })
                        .collect();
                    print_table(&["NAME", "FLOW", "STATUS", "CREATED"], &rows);
                }
            }
        }
        FlowRunCommands::Get { name, runtime } => {
            let r = client.get_flow_run(&runtime, &name)?;
            match output {
                OutputMode::Json => print_json(&r)?,
                OutputMode::Text => {
                    println!("Name:     {}", r.name);
                    println!("Flow:     {}", r.flow);
                    println!("Status:   {}", r.status);
                    println!("Runtime:  {}", r.runtime_uuid);
                    println!("Build:    {}", r.build_uuid);
                    println!("Created:  {}", r.created_at);
                    if let Some(error) = &r.error {
                        println!("Error:    {}", error);
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_build(
    client: &AscendClient,
    cmd: Option<BuildCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        Cli::parse_from(["ascend-ops", "build", "--help"]);
        unreachable!()
    };
    match cmd {
        BuildCommands::List { runtime } => {
            let builds = client.list_builds(&runtime)?;
            match output {
                OutputMode::Json => print_json(&builds)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = builds
                        .iter()
                        .map(|b| {
                            vec![
                                b.uuid.clone(),
                                b.state.clone().unwrap_or_else(|| "-".into()),
                                b.created_at.clone(),
                            ]
                        })
                        .collect();
                    print_table(&["UUID", "STATE", "CREATED"], &rows);
                }
            }
        }
        BuildCommands::Get { uuid } => {
            let b = client.get_build(&uuid)?;
            match output {
                OutputMode::Json => print_json(&b)?,
                OutputMode::Text => {
                    println!("UUID:     {}", b.uuid);
                    println!("Runtime:  {}", b.runtime_uuid);
                    println!("Git SHA:  {}", b.git_sha);
                    println!("State:    {}", b.state.unwrap_or_else(|| "-".into()));
                    println!("Created:  {}", b.created_at);
                    println!("Updated:  {}", b.updated_at);
                    if let Some(error) = &b.error_details {
                        println!("Error:    {}", error);
                    }
                }
            }
        }
    }
    Ok(())
}

// -- output helpers --

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Print rows as a fixed-width table with a header.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        return;
    }

    let widths: Vec<usize> = (0..headers.len())
        .map(|i| {
            let header_w = headers[i].len();
            let max_row_w = rows
                .iter()
                .map(|r| r.get(i).map_or(0, |s| s.len()))
                .max()
                .unwrap_or(0);
            header_w.max(max_row_w)
        })
        .collect();

    let last = headers.len() - 1;

    // Header
    for (i, h) in headers.iter().enumerate() {
        if i < last {
            print!("{:<width$}  ", h, width = widths[i]);
        } else {
            println!("{h}");
        }
    }

    // Rows
    for row in rows {
        for (i, val) in row.iter().enumerate() {
            if i < last {
                print!("{:<width$}  ", val, width = widths[i]);
            } else {
                println!("{val}");
            }
        }
    }
}

fn parse_spec(spec: Option<String>) -> Result<Option<serde_json::Value>> {
    match spec {
        Some(s) => {
            let v: serde_json::Value =
                serde_json::from_str(&s).map_err(|e| anyhow::anyhow!("invalid JSON spec: {e}"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parses_runtime_list() {
        let cli = Cli::parse_from(["ascend-ops", "runtime", "list"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Runtime {
                command: Some(RuntimeCommands::List { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_flow_run() {
        let cli = Cli::parse_from(["ascend-ops", "flow", "run", "uuid-123", "my-flow"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Flow {
                command: Some(FlowCommands::Run { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_flow_run_list() {
        let cli = Cli::parse_from([
            "ascend-ops",
            "flow-run",
            "list",
            "--runtime",
            "uuid-123",
            "--status",
            "running",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::FlowRun {
                command: Some(FlowRunCommands::List { .. })
            })
        ));
    }

    #[test]
    fn test_cli_no_subcommand_is_none() {
        let cli = Cli::parse_from(["ascend-ops"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_runtime_no_subcommand_is_none() {
        let cli = Cli::parse_from(["ascend-ops", "runtime"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Runtime { command: None })
        ));
    }

    #[test]
    fn test_cli_parses_output_json() {
        let cli = Cli::parse_from(["ascend-ops", "-o", "json", "runtime", "list"]);
        assert!(matches!(cli.output, OutputMode::Json));
    }

    #[test]
    fn test_cli_default_output_is_text() {
        let cli = Cli::parse_from(["ascend-ops", "runtime", "list"]);
        assert!(cli.output == OutputMode::Text);
    }

    #[test]
    fn test_parse_spec_valid() {
        let result = parse_spec(Some(r#"{"key": "value"}"#.to_string()));
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_parse_spec_none() {
        let result = parse_spec(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_spec_invalid() {
        let result = parse_spec(Some("not json".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_print_table_empty() {
        print_table(&["A", "B"], &[]);
    }

    #[test]
    fn test_print_table_rows() {
        print_table(
            &["ID", "NAME"],
            &[
                vec!["1".into(), "alice".into()],
                vec!["1000".into(), "b".into()],
            ],
        );
    }
}
