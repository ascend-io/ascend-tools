use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::models::*;
use clap::{Parser, Subcommand, ValueEnum};
use std::ffi::OsString;

#[derive(Parser)]
#[command(name = "ascend-tools", version, about = "CLI for the Ascend REST API")]
struct Cli {
    #[arg(short, long, global = true, value_enum, default_value_t = OutputMode::Text)]
    output: OutputMode,

    #[arg(
        long,
        global = true,
        env = "ASCEND_SERVICE_ACCOUNT_ID",
        hide_env_values = true
    )]
    service_account_id: Option<String>,

    #[arg(
        long,
        global = true,
        env = "ASCEND_SERVICE_ACCOUNT_KEY",
        hide_env_values = true
    )]
    service_account_key: Option<String>,

    #[arg(long, global = true, env = "ASCEND_INSTANCE_API_URL")]
    instance_api_url: Option<String>,

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
    /// Manage flows and flow runs
    Flow {
        #[command(subcommand)]
        command: Option<FlowCommands>,
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
    /// List flows in a runtime
    List {
        #[arg(short, long, required = true)]
        runtime: String,
    },
    /// Run a flow
    Run {
        /// Flow name
        flow_name: String,
        /// Runtime UUID
        #[arg(short, long, required = true)]
        runtime: String,
        /// Optional spec as JSON
        #[arg(long)]
        spec: Option<String>,
    },
    /// List flow runs
    ListRuns {
        #[arg(short, long, required = true)]
        runtime: String,
        #[arg(long)]
        status: Option<String>,
        #[arg(short, long)]
        flow_name: Option<String>,
    },
    /// Get a flow run
    GetRun {
        /// Flow run name
        name: String,
        #[arg(short, long, required = true)]
        runtime: String,
    },
}

pub fn run<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    let Some(command) = cli.command else {
        Cli::parse_from(["ascend-tools", "--help"]);
        unreachable!()
    };

    let config = Config::with_overrides(
        cli.service_account_id.as_deref(),
        cli.service_account_key.as_deref(),
        cli.instance_api_url.as_deref(),
    )?;
    let client = AscendClient::new(config)?;

    match command {
        Commands::Runtime { command } => handle_runtime(&client, command, &cli.output),
        Commands::Flow { command } => handle_flow(&client, command, &cli.output),
    }
}

fn handle_runtime(
    client: &AscendClient,
    cmd: Option<RuntimeCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        Cli::parse_from(["ascend-tools", "runtime", "--help"]);
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
        Cli::parse_from(["ascend-tools", "flow", "--help"]);
        unreachable!()
    };
    match cmd {
        FlowCommands::List { runtime } => {
            let flows = client.list_flows(&runtime)?;
            match output {
                OutputMode::Json => print_json(&flows)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> =
                        flows.iter().map(|f| vec![f.name.clone()]).collect();
                    print_table(&["NAME"], &rows);
                }
            }
        }
        FlowCommands::Run {
            runtime,
            flow_name,
            spec,
        } => {
            let spec_value = parse_spec(spec)?;
            let trigger = client.run_flow(&runtime, &flow_name, spec_value)?;
            match output {
                OutputMode::Json => print_json(&trigger)?,
                OutputMode::Text => println!("{}", trigger.event_uuid),
            }
        }
        FlowCommands::ListRuns {
            runtime,
            status,
            flow_name,
        } => {
            let runs = client.list_flow_runs(
                &runtime,
                FlowRunFilters {
                    status,
                    flow: flow_name,
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
        FlowCommands::GetRun { name, runtime } => {
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

// -- output helpers --

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Print rows as a fixed-width table with a header.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        eprintln!("No results.");
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
        let cli = Cli::parse_from(["ascend-tools", "runtime", "list"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Runtime {
                command: Some(RuntimeCommands::List { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_flow_run() {
        let cli = Cli::parse_from([
            "ascend-tools",
            "flow",
            "run",
            "my-flow",
            "--runtime",
            "uuid-123",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::Flow {
                command: Some(FlowCommands::Run { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_flow_list_runs() {
        let cli = Cli::parse_from([
            "ascend-tools",
            "flow",
            "list-runs",
            "--runtime",
            "uuid-123",
            "--status",
            "running",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::Flow {
                command: Some(FlowCommands::ListRuns { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_flow_get_run() {
        let cli = Cli::parse_from([
            "ascend-tools",
            "flow",
            "get-run",
            "fr-abc123",
            "--runtime",
            "uuid-123",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::Flow {
                command: Some(FlowCommands::GetRun { .. })
            })
        ));
    }

    #[test]
    fn test_cli_no_subcommand_is_none() {
        let cli = Cli::parse_from(["ascend-tools"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_flow_no_subcommand_is_none() {
        let cli = Cli::parse_from(["ascend-tools", "flow"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Flow { command: None })
        ));
    }

    #[test]
    fn test_cli_parses_output_json() {
        let cli = Cli::parse_from(["ascend-tools", "-o", "json", "runtime", "list"]);
        assert!(matches!(cli.output, OutputMode::Json));
    }

    #[test]
    fn test_cli_default_output_is_text() {
        let cli = Cli::parse_from(["ascend-tools", "runtime", "list"]);
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
