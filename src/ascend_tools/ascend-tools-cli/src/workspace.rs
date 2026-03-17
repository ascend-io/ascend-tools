use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{RuntimeCreate, RuntimeKind, RuntimeUpdate};
use clap::Subcommand;

use crate::common::{
    OutputMode, handle_runtime_create, handle_runtime_delete, handle_runtime_get,
    handle_runtime_list, handle_runtime_pause, handle_runtime_resume, handle_runtime_update,
    print_subcommand_help,
};

#[derive(Subcommand)]
pub(crate) enum WorkspaceCommands {
    /// List workspaces
    #[command(
        long_about = "List workspaces, optionally filtered by title, environment, or project.\n\n\
            The --environment and --project flags accept names (resolved to UUIDs) or UUIDs directly.\n\n\
            Examples:\n  \
            ascend-tools workspace list\n  \
            ascend-tools workspace list --environment Production\n  \
            ascend-tools workspace list --project MyProject"
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
    /// Get a workspace by title
    #[command(arg_required_else_help = true)]
    Get {
        /// Workspace title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
    /// Create a workspace
    #[command(
        arg_required_else_help = true,
        long_about = "Create a new workspace.\n\n\
            The --environment and --project flags accept names (resolved to UUIDs) or UUIDs directly.\n\n\
            Examples:\n  \
            ascend-tools workspace create --title my-ws --environment Production --project MyProject --profile default --git-branch main\n  \
            ascend-tools workspace create --title my-ws --environment Production --project MyProject --profile default --git-branch feature/abc --size Medium"
    )]
    Create {
        /// Workspace title
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
        /// Minutes of inactivity before auto-snooze
        #[arg(long)]
        auto_snooze_timeout_minutes: Option<u32>,
    },
    /// Update a workspace
    #[command(arg_required_else_help = true)]
    Update {
        /// Workspace title
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
        /// New auto-snooze timeout in minutes
        #[arg(long)]
        auto_snooze_timeout_minutes: Option<u32>,
    },
    /// Pause a workspace
    #[command(arg_required_else_help = true)]
    Pause {
        /// Workspace title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
    /// Resume a paused workspace
    #[command(arg_required_else_help = true)]
    Resume {
        /// Workspace title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
    },
    /// Delete a workspace
    #[command(arg_required_else_help = true)]
    Delete {
        /// Workspace title
        title: String,
        /// Use UUID instead of title
        #[arg(long)]
        uuid: Option<String>,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
}

pub(crate) fn handle_workspace(
    client: &AscendClient,
    cmd: Option<WorkspaceCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        return print_subcommand_help("workspace");
    };
    match cmd {
        WorkspaceCommands::List {
            title,
            project,
            environment,
        } => handle_runtime_list(
            client,
            RuntimeKind::Workspace,
            title,
            project,
            environment,
            output,
        ),
        WorkspaceCommands::Get { title, uuid } => handle_runtime_get(
            client,
            RuntimeKind::Workspace,
            &title,
            uuid.as_deref(),
            output,
        ),
        WorkspaceCommands::Create {
            title,
            environment,
            project,
            profile,
            git_branch,
            git_branch_base,
            size,
            storage_size,
            auto_snooze_timeout_minutes,
        } => {
            let mut create = RuntimeCreate::new(title, environment, project, profile, git_branch);
            create.git_branch_base = git_branch_base;
            create.size = size;
            create.storage_size = storage_size;
            create.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
            handle_runtime_create(client, RuntimeKind::Workspace, &create, output)
        }
        WorkspaceCommands::Update {
            current_title,
            uuid,
            title,
            git_branch,
            git_branch_base,
            profile,
            size,
            storage_size,
            auto_snooze_timeout_minutes,
        } => {
            let mut update = RuntimeUpdate::default();
            update.title = title;
            update.git_branch = git_branch;
            update.git_branch_base = git_branch_base;
            update.profile = profile;
            update.size = size;
            update.storage_size = storage_size;
            update.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
            handle_runtime_update(
                client,
                RuntimeKind::Workspace,
                &current_title,
                uuid.as_deref(),
                update,
                output,
            )
        }
        WorkspaceCommands::Pause { title, uuid } => handle_runtime_pause(
            client,
            RuntimeKind::Workspace,
            &title,
            uuid.as_deref(),
            output,
        ),
        WorkspaceCommands::Resume { title, uuid } => handle_runtime_resume(
            client,
            RuntimeKind::Workspace,
            &title,
            uuid.as_deref(),
            output,
        ),
        WorkspaceCommands::Delete { title, uuid, yes } => handle_runtime_delete(
            client,
            RuntimeKind::Workspace,
            &title,
            uuid.as_deref(),
            yes,
            output,
        ),
    }
}
