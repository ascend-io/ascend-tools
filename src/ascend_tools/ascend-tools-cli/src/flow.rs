use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::FlowRunFilters;
use clap::Subcommand;

use crate::common::{OutputMode, parse_spec, print_json, print_table, resolve_runtime_target};

#[derive(Subcommand)]
pub(crate) enum FlowCommands {
    /// List flows
    #[command(arg_required_else_help = true)]
    List {
        /// Workspace title
        #[arg(long, group = "target")]
        workspace: Option<String>,
        /// Deployment title
        #[arg(long, group = "target")]
        deployment: Option<String>,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
    /// Run a flow
    #[command(arg_required_else_help = true)]
    Run {
        /// Flow name
        flow_name: String,
        /// Workspace title
        #[arg(long, group = "target")]
        workspace: Option<String>,
        /// Deployment title
        #[arg(long, group = "target")]
        deployment: Option<String>,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
        /// Optional spec as JSON
        #[arg(long)]
        spec: Option<String>,
        /// Resume the runtime if paused before submitting
        #[arg(long)]
        resume: bool,
    },
    /// List flow runs
    #[command(arg_required_else_help = true)]
    ListRuns {
        /// Workspace title
        #[arg(long, group = "target")]
        workspace: Option<String>,
        /// Deployment title
        #[arg(long, group = "target")]
        deployment: Option<String>,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
        #[arg(long)]
        status: Option<String>,
        /// Filter by flow name
        #[arg(short, long)]
        flow: Option<String>,
        /// Filter by start time (ISO 8601)
        #[arg(long)]
        since: Option<String>,
        /// Filter by end time (ISO 8601)
        #[arg(long)]
        until: Option<String>,
        /// Pagination offset
        #[arg(long)]
        offset: Option<u64>,
        /// Pagination limit
        #[arg(long)]
        limit: Option<u64>,
    },
    /// Get a flow run
    #[command(arg_required_else_help = true)]
    GetRun {
        /// Flow run name
        name: String,
        /// Workspace title
        #[arg(long, group = "target")]
        workspace: Option<String>,
        /// Deployment title
        #[arg(long, group = "target")]
        deployment: Option<String>,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
}

pub(crate) fn handle_flow(
    client: &AscendClient,
    cmd: Option<FlowCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        use clap::CommandFactory;
        crate::cli::CliParser::command()
            .find_subcommand_mut("flow")
            .expect("flow subcommand exists")
            .print_help()?;
        return Ok(());
    };
    match cmd {
        FlowCommands::List {
            workspace,
            deployment,
            uuid,
        } => {
            let runtime_uuid = resolve_runtime_target(
                client,
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;
            let flows = client.list_flows(&runtime_uuid)?;
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
            workspace,
            deployment,
            uuid,
            flow_name,
            spec,
            resume,
        } => {
            let runtime_uuid = resolve_runtime_target(
                client,
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;
            let spec_value = parse_spec(spec)?;
            let trigger = client.run_flow(&runtime_uuid, &flow_name, spec_value, resume)?;
            match output {
                OutputMode::Json => print_json(&trigger)?,
                OutputMode::Text => println!("{}", trigger.event_uuid),
            }
        }
        FlowCommands::ListRuns {
            workspace,
            deployment,
            uuid,
            status,
            flow,
            since,
            until,
            offset,
            limit,
        } => {
            let runtime_uuid = resolve_runtime_target(
                client,
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;
            let mut filters = FlowRunFilters::default();
            filters.status = status;
            filters.flow = flow;
            filters.since = since;
            filters.until = until;
            filters.offset = offset;
            filters.limit = limit;
            let result = client.list_flow_runs(&runtime_uuid, filters)?;
            if result.truncated {
                eprintln!("Warning: results may be incomplete (server-side limit reached)");
            }
            let runs = &result.items;
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
        FlowCommands::GetRun {
            name,
            workspace,
            deployment,
            uuid,
        } => {
            let runtime_uuid = resolve_runtime_target(
                client,
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;
            let r = client.get_flow_run(&runtime_uuid, &name)?;
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
