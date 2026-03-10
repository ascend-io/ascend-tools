use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::RuntimeKind;
use clap::Subcommand;

use crate::common::{OutputMode, print_json, print_table};

#[derive(Subcommand)]
pub(crate) enum ProfileCommands {
    /// List available profiles
    #[command(arg_required_else_help = true)]
    List {
        /// Workspace title (derives project and branch)
        #[arg(long, group = "source")]
        workspace: Option<String>,
        /// Deployment title (derives project and branch)
        #[arg(long, group = "source")]
        deployment: Option<String>,
        /// Project name (or UUID) — requires --git-branch
        #[arg(long, group = "source")]
        project: Option<String>,
        /// Git branch (required with --project)
        #[arg(long = "git-branch")]
        branch: Option<String>,
        /// Runtime UUID (direct override)
        #[arg(long)]
        uuid: Option<String>,
    },
}

pub(crate) fn handle_profile(
    client: &AscendClient,
    cmd: Option<ProfileCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        use clap::CommandFactory;
        crate::cli::CliParser::command()
            .find_subcommand_mut("profile")
            .expect("profile subcommand exists")
            .print_help()?;
        return Ok(());
    };
    match cmd {
        ProfileCommands::List {
            workspace,
            deployment,
            project,
            branch,
            uuid,
        } => {
            let (runtime_uuid, project_name, branch_val) = if let Some(uuid) = uuid {
                (Some(uuid), None, None)
            } else if let Some(ws) = workspace {
                let rt = client.resolve_runtime_by_title(&ws, RuntimeKind::Workspace)?;
                (Some(rt.uuid), None, None)
            } else if let Some(dep) = deployment {
                let rt = client.resolve_runtime_by_title(&dep, RuntimeKind::Deployment)?;
                (Some(rt.uuid), None, None)
            } else if let Some(proj) = project {
                (None, Some(proj), branch)
            } else {
                anyhow::bail!("Provide --workspace, --deployment, or --project with --git-branch");
            };
            let profiles = client.list_profiles(
                runtime_uuid.as_deref(),
                project_name.as_deref(),
                branch_val.as_deref(),
            )?;
            match output {
                OutputMode::Json => print_json(&profiles)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = profiles.iter().map(|p| vec![p.clone()]).collect();
                    print_table(&["PROFILE"], &rows);
                }
            }
            Ok(())
        }
    }
}
