use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::FlowRunFilters;
use clap::Subcommand;

use crate::common::{OutputMode, parse_spec, print_json, print_subcommand_help, print_table};

#[derive(Subcommand)]
pub(crate) enum FlowCommands {
    /// List flows
    #[command(arg_required_else_help = true)]
    #[command(group = clap::ArgGroup::new("target").required(true))]
    List {
        /// Workspace title
        #[arg(long, group = "target")]
        workspace: Option<String>,
        /// Deployment title
        #[arg(long, group = "target")]
        deployment: Option<String>,
        /// Use UUID instead of title
        #[arg(long, group = "target")]
        uuid: Option<String>,
    },
    /// Run a flow
    #[command(arg_required_else_help = true)]
    #[command(group = clap::ArgGroup::new("target").required(true))]
    Run {
        /// Flow name
        flow: String,
        /// Workspace title
        #[arg(long, group = "target")]
        workspace: Option<String>,
        /// Deployment title
        #[arg(long, group = "target")]
        deployment: Option<String>,
        /// Use UUID instead of title
        #[arg(long, group = "target")]
        uuid: Option<String>,
        /// Optional spec as JSON
        #[arg(long)]
        spec: Option<String>,
        /// Resume the workspace/deployment if paused before running
        #[arg(long)]
        resume: bool,
    },
    /// List flow runs
    #[command(arg_required_else_help = true)]
    #[command(group = clap::ArgGroup::new("target").required(true))]
    ListRuns {
        /// Workspace title
        #[arg(long, group = "target")]
        workspace: Option<String>,
        /// Deployment title
        #[arg(long, group = "target")]
        deployment: Option<String>,
        /// Use UUID instead of title
        #[arg(long, group = "target")]
        uuid: Option<String>,
        /// Filter by status (e.g. running, succeeded, failed)
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
    #[command(group = clap::ArgGroup::new("target").required(true))]
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
        #[arg(long, group = "target")]
        uuid: Option<String>,
    },
}

pub(crate) fn handle_flow(
    client: &AscendClient,
    cmd: Option<FlowCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        return print_subcommand_help("flow");
    };
    match cmd {
        FlowCommands::List {
            workspace,
            deployment,
            uuid,
        } => {
            let runtime_uuid = client.resolve_runtime_target(
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
            flow,
            spec,
            resume,
        } => {
            let runtime_uuid = client.resolve_runtime_target(
                workspace.as_deref(),
                deployment.as_deref(),
                uuid.as_deref(),
            )?;
            let spec_value = parse_spec(spec)?;
            let trigger = client.run_flow(&runtime_uuid, &flow, spec_value, resume)?;
            match output {
                OutputMode::Json => print_json(&trigger)?,
                OutputMode::Text => println!("Triggered flow '{}' ({})", flow, trigger.event_uuid),
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
            let runtime_uuid = client.resolve_runtime_target(
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
                eprintln!("warning: results may be incomplete (server-side limit reached)");
            }
            let runs = &result.items;
            match output {
                OutputMode::Json => print_json(&result)?,
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
            let runtime_uuid = client.resolve_runtime_target(
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
                    println!("Target:   {}", r.runtime_uuid);
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
