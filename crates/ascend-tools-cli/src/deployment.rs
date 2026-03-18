use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{RuntimeCreate, RuntimeKind, RuntimeUpdate};
use clap::Subcommand;

use crate::common::{
    OutputMode, handle_runtime_create, handle_runtime_delete, handle_runtime_get,
    handle_runtime_list, handle_runtime_update, print_json, print_subcommand_help,
};

#[derive(Subcommand)]
pub(crate) enum DeploymentCommands {
    /// List deployments
    #[command(
        long_about = "List deployments, optionally filtered by title, environment, or project.\n\n\
            The --environment and --project flags accept names (resolved to UUIDs) or UUIDs directly.\n\n\
            Examples:\n  \
            ascend-tools deployment list\n  \
            ascend-tools deployment list --environment Production\n  \
            ascend-tools deployment list --project MyProject"
    )]
    List {
        /// Filter by title
        #[arg(long)]
        title: Option<String>,
        /// Filter by project name (or UUID)
        #[arg(long)]
        project: Option<String>,
        /// Filter by environment name (or UUID)
        #[arg(long)]
        environment: Option<String>,
    },
    /// Get a deployment by title
    #[command(arg_required_else_help = true)]
    Get {
        /// Deployment title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
    /// Create a deployment
    #[command(
        arg_required_else_help = true,
        long_about = "Create a new deployment.\n\n\
            The --environment and --project flags accept names (resolved to UUIDs) or UUIDs directly.\n\n\
            Examples:\n  \
            ascend-tools deployment create --title prod --environment Production --project MyProject --profile default --git-branch main\n  \
            ascend-tools deployment create --title prod --environment Production --project MyProject --profile default --git-branch main --enable-automations true"
    )]
    Create {
        /// Deployment title
        #[arg(long, required = true)]
        title: String,
        /// Environment name (or UUID)
        #[arg(long, required = true)]
        environment: String,
        /// Project name (or UUID)
        #[arg(long, required = true)]
        project: String,
        /// Configuration profile
        #[arg(long, required = true)]
        profile: String,
        /// Git branch
        #[arg(long, required = true)]
        git_branch: String,
        /// Base git branch
        #[arg(long)]
        git_branch_base: Option<String>,
        /// Size (e.g. Small, Medium, Large)
        #[arg(long)]
        size: Option<String>,
        /// Storage size in GB
        #[arg(long)]
        storage_size: Option<u32>,
        /// Enable automations (default: true for deployments)
        #[arg(long)]
        enable_automations: Option<bool>,
    },
    /// Update a deployment
    #[command(arg_required_else_help = true)]
    Update {
        /// Deployment title
        current_title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// Switch to a different git branch
        #[arg(long)]
        git_branch: Option<String>,
        /// New base git branch
        #[arg(long)]
        git_branch_base: Option<String>,
        /// New profile
        #[arg(long)]
        profile: Option<String>,
        /// New size
        #[arg(long)]
        size: Option<String>,
        /// New storage size in GB
        #[arg(long)]
        storage_size: Option<u32>,
        /// Enable or disable automations
        #[arg(long)]
        enable_automations: Option<bool>,
    },
    /// Pause automations on a deployment
    #[command(arg_required_else_help = true)]
    PauseAutomations {
        /// Deployment title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
    /// Resume automations on a deployment
    #[command(arg_required_else_help = true)]
    ResumeAutomations {
        /// Deployment title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
    /// Delete a deployment
    #[command(arg_required_else_help = true)]
    Delete {
        /// Deployment title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
}

pub(crate) fn handle_deployment(
    client: &AscendClient,
    cmd: Option<DeploymentCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        return print_subcommand_help("deployment");
    };
    match cmd {
        DeploymentCommands::List {
            title,
            project,
            environment,
        } => handle_runtime_list(
            client,
            RuntimeKind::Deployment,
            title,
            project,
            environment,
            output,
        ),
        DeploymentCommands::Get { title, uuid } => handle_runtime_get(
            client,
            RuntimeKind::Deployment,
            &title,
            uuid.as_deref(),
            output,
        ),
        DeploymentCommands::Create {
            title,
            environment,
            project,
            profile,
            git_branch,
            git_branch_base,
            size,
            storage_size,
            enable_automations,
        } => {
            let mut create = RuntimeCreate::new(title, environment, project, profile, git_branch);
            create.git_branch_base = git_branch_base;
            create.size = size;
            create.storage_size = storage_size;
            create.enable_automations = enable_automations;
            handle_runtime_create(client, RuntimeKind::Deployment, &create, output)
        }
        DeploymentCommands::Update {
            current_title,
            uuid,
            title,
            git_branch,
            git_branch_base,
            profile,
            size,
            storage_size,
            enable_automations,
        } => {
            let mut update = RuntimeUpdate::default();
            update.title = title;
            update.git_branch = git_branch;
            update.git_branch_base = git_branch_base;
            update.profile = profile;
            update.size = size;
            update.storage_size = storage_size;
            update.enable_automations = enable_automations;
            handle_runtime_update(
                client,
                RuntimeKind::Deployment,
                &current_title,
                uuid.as_deref(),
                update,
                output,
            )
        }
        DeploymentCommands::PauseAutomations { title, uuid } => {
            let r = client.pause_deployment_automations(&title, uuid.as_deref())?;
            match output {
                OutputMode::Json => print_json(&r)?,
                OutputMode::Text => println!("Paused automations on deployment '{}'", r.title),
            }
            Ok(())
        }
        DeploymentCommands::ResumeAutomations { title, uuid } => {
            let r = client.resume_deployment_automations(&title, uuid.as_deref())?;
            match output {
                OutputMode::Json => print_json(&r)?,
                OutputMode::Text => println!("Resumed automations on deployment '{}'", r.title),
            }
            Ok(())
        }
        DeploymentCommands::Delete { title, uuid, yes } => handle_runtime_delete(
            client,
            RuntimeKind::Deployment,
            &title,
            uuid.as_deref(),
            yes,
            output,
        ),
    }
}
