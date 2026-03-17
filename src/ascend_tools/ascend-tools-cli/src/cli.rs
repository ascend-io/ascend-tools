use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use clap::{CommandFactory, Parser, Subcommand};
use std::ffi::OsString;

use crate::common::OutputMode;
use crate::deployment::DeploymentCommands;
use crate::environment::EnvironmentCommands;
use crate::flow::FlowCommands;
use crate::profile::ProfileCommands;
use crate::project::ProjectCommands;
use crate::skill::SkillCommands;
use crate::workspace::WorkspaceCommands;

#[derive(Parser)]
#[command(name = "ascend-tools", version, about = "CLI for the Ascend REST API")]
pub(crate) struct CliParser {
    #[arg(short, long, global = true, value_enum, default_value_t = OutputMode::Text)]
    output: OutputMode,

    /// Service account ID
    #[arg(long, global = true, env = "ASCEND_SERVICE_ACCOUNT_ID")]
    service_account_id: Option<String>,

    /// Service account key
    #[arg(
        long,
        global = true,
        env = "ASCEND_SERVICE_ACCOUNT_KEY",
        hide_env_values = true
    )]
    service_account_key: Option<String>,

    /// Instance API URL
    #[arg(long, global = true, env = "ASCEND_INSTANCE_API_URL")]
    instance_api_url: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage workspaces
    #[command(long_about = "Manage Ascend workspaces.\n\n\
            Examples:\n  \
            ascend-tools workspace list\n  \
            ascend-tools workspace list --environment Production\n  \
            ascend-tools workspace get my-workspace\n  \
            ascend-tools workspace create --title my-ws --environment Production --project MyProject --profile default --git-branch main\n  \
            ascend-tools workspace pause my-workspace\n  \
            ascend-tools workspace resume my-workspace")]
    Workspace {
        #[command(subcommand)]
        command: Option<WorkspaceCommands>,
    },
    /// Manage deployments
    #[command(long_about = "Manage Ascend deployments.\n\n\
            Examples:\n  \
            ascend-tools deployment list\n  \
            ascend-tools deployment list --project MyProject\n  \
            ascend-tools deployment get prod\n  \
            ascend-tools deployment create --title prod --environment Production --project MyProject --profile default --git-branch main\n  \
            ascend-tools deployment pause-automations prod\n  \
            ascend-tools deployment resume-automations prod")]
    Deployment {
        #[command(subcommand)]
        command: Option<DeploymentCommands>,
    },
    /// Manage flows and flow runs
    #[command(
        long_about = "Manage flows and flow runs in a workspace or deployment.\n\n\
            Examples:\n  \
            ascend-tools flow list --workspace my-ws\n  \
            ascend-tools flow run sales --workspace my-ws\n  \
            ascend-tools flow run sales --deployment prod --resume\n  \
            ascend-tools flow list-runs --workspace my-ws --status running\n  \
            ascend-tools flow get-run fr-abc123 --workspace my-ws"
    )]
    Flow {
        #[command(subcommand)]
        command: Option<FlowCommands>,
    },
    /// Manage environments
    #[command(long_about = "Manage Ascend environments.\n\n\
            Examples:\n  \
            ascend-tools environment list\n  \
            ascend-tools environment get Production")]
    Environment {
        #[command(subcommand)]
        command: Option<EnvironmentCommands>,
    },
    /// Manage projects
    #[command(long_about = "Manage Ascend projects.\n\n\
            Examples:\n  \
            ascend-tools project list\n  \
            ascend-tools project get \"My Project\"")]
    Project {
        #[command(subcommand)]
        command: Option<ProjectCommands>,
    },
    /// Manage profiles
    #[command(long_about = "Manage Ascend profiles.\n\n\
            Examples:\n  \
            ascend-tools profile list --workspace my-ws\n  \
            ascend-tools profile list --project MyProject --git-branch main")]
    Profile {
        #[command(subcommand)]
        command: Option<ProfileCommands>,
    },
    /// Start an MCP server
    Mcp {
        /// Use HTTP transport instead of stdio
        #[arg(long)]
        http: bool,
        /// Bind address for HTTP transport
        #[arg(long, default_value = "127.0.0.1:8000", requires = "http")]
        bind: String,
    },
    /// Manage skills
    Skill {
        #[command(subcommand)]
        command: Option<SkillCommands>,
    },
}

pub fn run_cli<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = CliParser::parse_from(args);

    let Some(command) = cli.command else {
        CliParser::command().print_help()?;
        return Ok(());
    };

    // Commands that don't require authentication
    if let Commands::Skill { command } = command {
        return crate::skill::handle_skill(command);
    }

    if let Commands::Mcp { http, bind } = command {
        let config = Config::with_overrides(
            cli.service_account_id.as_deref(),
            cli.service_account_key.as_deref(),
            cli.instance_api_url.as_deref(),
        );
        let rt = tokio::runtime::Runtime::new()?;
        return if http {
            rt.block_on(ascend_tools_mcp::run_http(config, &bind))
        } else {
            rt.block_on(ascend_tools_mcp::run_stdio(config))
        };
    }

    let config = Config::with_overrides(
        cli.service_account_id.as_deref(),
        cli.service_account_key.as_deref(),
        cli.instance_api_url.as_deref(),
    )?;

    let client = AscendClient::new(config)?;

    match command {
        Commands::Workspace { command } => {
            crate::workspace::handle_workspace(&client, command, &cli.output)
        }
        Commands::Deployment { command } => {
            crate::deployment::handle_deployment(&client, command, &cli.output)
        }
        Commands::Environment { command } => {
            crate::environment::handle_environment(&client, command, &cli.output)
        }
        Commands::Project { command } => {
            crate::project::handle_project(&client, command, &cli.output)
        }
        Commands::Profile { command } => {
            crate::profile::handle_profile(&client, command, &cli.output)
        }
        Commands::Flow { command } => crate::flow::handle_flow(&client, command, &cli.output),
        Commands::Mcp { .. } | Commands::Skill { .. } => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common;

    #[test]
    fn test_cli_parses_workspace_list() {
        let cli = CliParser::parse_from(["ascend-tools", "workspace", "list"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Workspace {
                command: Some(WorkspaceCommands::List { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_workspace_list_with_environment() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "workspace",
            "list",
            "--environment",
            "Production",
        ]);
        match cli.command {
            Some(Commands::Workspace {
                command:
                    Some(WorkspaceCommands::List {
                        environment,
                        project,
                        ..
                    }),
            }) => {
                assert_eq!(environment.as_deref(), Some("Production"));
                assert!(project.is_none());
            }
            _ => panic!("expected Workspace List command"),
        }
    }

    #[test]
    fn test_cli_parses_workspace_list_with_project() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "workspace",
            "list",
            "--project",
            "MyProject",
        ]);
        match cli.command {
            Some(Commands::Workspace {
                command:
                    Some(WorkspaceCommands::List {
                        project,
                        environment,
                        ..
                    }),
            }) => {
                assert_eq!(project.as_deref(), Some("MyProject"));
                assert!(environment.is_none());
            }
            _ => panic!("expected Workspace List command"),
        }
    }

    #[test]
    fn test_cli_parses_workspace_get() {
        let cli = CliParser::parse_from(["ascend-tools", "workspace", "get", "my-workspace"]);
        match cli.command {
            Some(Commands::Workspace {
                command: Some(WorkspaceCommands::Get { title, uuid }),
            }) => {
                assert_eq!(title, "my-workspace");
                assert!(uuid.is_none());
            }
            _ => panic!("expected Workspace Get command"),
        }
    }

    #[test]
    fn test_cli_parses_workspace_get_with_uuid() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "workspace",
            "get",
            "ignored",
            "--uuid",
            "abc-123",
        ]);
        match cli.command {
            Some(Commands::Workspace {
                command: Some(WorkspaceCommands::Get { uuid, .. }),
            }) => {
                assert_eq!(uuid.as_deref(), Some("abc-123"));
            }
            _ => panic!("expected Workspace Get command"),
        }
    }

    #[test]
    fn test_cli_parses_workspace_create() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "workspace",
            "create",
            "--title",
            "My WS",
            "--environment",
            "Production",
            "--project",
            "MyProject",
            "--profile",
            "default",
            "--git-branch",
            "main",
            "--size",
            "Medium",
        ]);
        match cli.command {
            Some(Commands::Workspace {
                command:
                    Some(WorkspaceCommands::Create {
                        title,
                        environment,
                        project,
                        size,
                        profile,
                        ..
                    }),
            }) => {
                assert_eq!(title, "My WS");
                assert_eq!(environment, "Production");
                assert_eq!(project, "MyProject");
                assert_eq!(size.as_deref(), Some("Medium"));
                assert_eq!(profile, "default");
            }
            _ => panic!("expected Workspace Create command"),
        }
    }

    #[test]
    fn test_cli_parses_workspace_pause() {
        let cli = CliParser::parse_from(["ascend-tools", "workspace", "pause", "my-workspace"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Workspace {
                command: Some(WorkspaceCommands::Pause { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_workspace_delete_yes() {
        let cli = CliParser::parse_from(["ascend-tools", "workspace", "delete", "old-ws", "--yes"]);
        match cli.command {
            Some(Commands::Workspace {
                command: Some(WorkspaceCommands::Delete { title, yes, .. }),
            }) => {
                assert_eq!(title, "old-ws");
                assert!(yes);
            }
            _ => panic!("expected Workspace Delete command"),
        }
    }

    #[test]
    fn test_cli_parses_deployment_list() {
        let cli = CliParser::parse_from(["ascend-tools", "deployment", "list"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Deployment {
                command: Some(DeploymentCommands::List { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_deployment_create() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "deployment",
            "create",
            "--title",
            "prod",
            "--environment",
            "Production",
            "--project",
            "MyProject",
            "--profile",
            "default",
            "--git-branch",
            "main",
        ]);
        match cli.command {
            Some(Commands::Deployment {
                command:
                    Some(DeploymentCommands::Create {
                        title,
                        environment,
                        project,
                        ..
                    }),
            }) => {
                assert_eq!(title, "prod");
                assert_eq!(environment, "Production");
                assert_eq!(project, "MyProject");
            }
            _ => panic!("expected Deployment Create command"),
        }
    }

    #[test]
    fn test_cli_parses_deployment_pause_automations() {
        let cli =
            CliParser::parse_from(["ascend-tools", "deployment", "pause-automations", "prod"]);
        match cli.command {
            Some(Commands::Deployment {
                command: Some(DeploymentCommands::PauseAutomations { title, .. }),
            }) => {
                assert_eq!(title, "prod");
            }
            _ => panic!("expected Deployment PauseAutomations command"),
        }
    }

    #[test]
    fn test_cli_parses_deployment_resume_automations() {
        let cli =
            CliParser::parse_from(["ascend-tools", "deployment", "resume-automations", "prod"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Deployment {
                command: Some(DeploymentCommands::ResumeAutomations { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_flow_list_with_workspace() {
        let cli = CliParser::parse_from(["ascend-tools", "flow", "list", "--workspace", "my-ws"]);
        match cli.command {
            Some(Commands::Flow {
                command: Some(FlowCommands::List { workspace, .. }),
            }) => {
                assert_eq!(workspace.as_deref(), Some("my-ws"));
            }
            _ => panic!("expected Flow List command"),
        }
    }

    #[test]
    fn test_cli_parses_flow_run_with_deployment() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "flow",
            "run",
            "sales",
            "--deployment",
            "prod",
        ]);
        match cli.command {
            Some(Commands::Flow {
                command:
                    Some(FlowCommands::Run {
                        flow, deployment, ..
                    }),
            }) => {
                assert_eq!(flow, "sales");
                assert_eq!(deployment.as_deref(), Some("prod"));
            }
            _ => panic!("expected Flow Run command"),
        }
    }

    #[test]
    fn test_cli_no_subcommand_is_none() {
        let cli = CliParser::parse_from(["ascend-tools"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parses_output_json() {
        let cli = CliParser::parse_from(["ascend-tools", "-o", "json", "workspace", "list"]);
        assert!(matches!(cli.output, OutputMode::Json));
    }

    #[test]
    fn test_cli_default_output_is_text() {
        let cli = CliParser::parse_from(["ascend-tools", "workspace", "list"]);
        assert!(cli.output == OutputMode::Text);
    }

    #[test]
    fn test_parse_spec_valid() {
        let result = common::parse_spec(Some(r#"{"key": "value"}"#.to_string()));
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_parse_spec_none() {
        let result = common::parse_spec(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_spec_invalid() {
        let result = common::parse_spec(Some("not json".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_print_table_empty() {
        common::print_table(&["A", "B"], &[]);
    }

    #[test]
    fn test_print_table_rows() {
        common::print_table(
            &["ID", "NAME"],
            &[
                vec!["1".into(), "alice".into()],
                vec!["1000".into(), "b".into()],
            ],
        );
    }

    #[test]
    fn test_cli_parses_skill_install() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "skill",
            "install",
            "--target",
            "./.claude/skills",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::Skill {
                command: Some(SkillCommands::Install { .. })
            })
        ));
    }

    #[test]
    fn test_cli_parses_mcp_stdio() {
        let cli = CliParser::parse_from(["ascend-tools", "mcp"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Mcp { http: false, .. })
        ));
    }

    #[test]
    fn test_cli_parses_mcp_http() {
        let cli = CliParser::parse_from(["ascend-tools", "mcp", "--http"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Mcp { http: true, .. })
        ));
    }

    #[test]
    fn test_cli_parses_environment_list() {
        let cli = CliParser::parse_from(["ascend-tools", "environment", "list"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Environment {
                command: Some(EnvironmentCommands::List)
            })
        ));
    }

    #[test]
    fn test_cli_parses_environment_bare_is_none() {
        let cli = CliParser::parse_from(["ascend-tools", "environment"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Environment { command: None })
        ));
    }

    #[test]
    fn test_cli_parses_project_list() {
        let cli = CliParser::parse_from(["ascend-tools", "project", "list"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Project {
                command: Some(ProjectCommands::List)
            })
        ));
    }

    #[test]
    fn test_cli_parses_profile_list_with_workspace() {
        let cli = CliParser::parse_from(["ascend-tools", "profile", "list", "--workspace", "Cody"]);
        match cli.command {
            Some(Commands::Profile {
                command:
                    Some(ProfileCommands::List {
                        workspace,
                        deployment,
                        project,
                        ..
                    }),
            }) => {
                assert_eq!(workspace.as_deref(), Some("Cody"));
                assert!(deployment.is_none());
                assert!(project.is_none());
            }
            _ => panic!("expected Profile List command"),
        }
    }

    #[test]
    fn test_cli_parses_profile_list_with_project_and_branch() {
        let cli = CliParser::parse_from([
            "ascend-tools",
            "profile",
            "list",
            "--project",
            "MyProject",
            "--git-branch",
            "main",
        ]);
        match cli.command {
            Some(Commands::Profile {
                command:
                    Some(ProfileCommands::List {
                        project, branch, ..
                    }),
            }) => {
                assert_eq!(project.as_deref(), Some("MyProject"));
                assert_eq!(branch.as_deref(), Some("main"));
            }
            _ => panic!("expected Profile List command"),
        }
    }
}
