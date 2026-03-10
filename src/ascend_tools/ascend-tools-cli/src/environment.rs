use anyhow::Result;
use ascend_tools::client::AscendClient;
use clap::Subcommand;

use crate::common::{OutputMode, print_json, print_table};

#[derive(Subcommand)]
pub(crate) enum EnvironmentCommands {
    /// List environments
    List,
    /// Get an environment by title
    #[command(arg_required_else_help = true)]
    Get {
        /// Environment title
        title: String,
    },
}

pub(crate) fn handle_environment(
    client: &AscendClient,
    cmd: Option<EnvironmentCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        use clap::CommandFactory;
        crate::cli::CliParser::command()
            .find_subcommand_mut("environment")
            .expect("environment subcommand exists")
            .print_help()?;
        return Ok(());
    };
    match cmd {
        EnvironmentCommands::List => {
            let envs = client.list_environments()?;
            match output {
                OutputMode::Json => print_json(&envs)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = envs
                        .iter()
                        .map(|e| vec![e.title.clone(), e.uuid.clone()])
                        .collect();
                    print_table(&["TITLE", "UUID"], &rows);
                }
            }
            Ok(())
        }
        EnvironmentCommands::Get { title } => {
            let env = client.get_environment(&title)?;
            match output {
                OutputMode::Json => print_json(&env)?,
                OutputMode::Text => {
                    println!("Title:  {}", env.title);
                    println!("UUID:   {}", env.uuid);
                    println!("ID:     {}", env.id);
                }
            }
            Ok(())
        }
    }
}
