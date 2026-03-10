use anyhow::Result;
use ascend_tools::client::AscendClient;
use clap::Subcommand;

use crate::common::{OutputMode, print_json, print_table};

#[derive(Subcommand)]
pub(crate) enum ProjectCommands {
    /// List projects
    List,
    /// Get a project by title
    #[command(arg_required_else_help = true)]
    Get {
        /// Project title
        title: String,
    },
}

pub(crate) fn handle_project(
    client: &AscendClient,
    cmd: Option<ProjectCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        use clap::CommandFactory;
        crate::cli::CliParser::command()
            .find_subcommand_mut("project")
            .expect("project subcommand exists")
            .print_help()?;
        return Ok(());
    };
    match cmd {
        ProjectCommands::List => {
            let projects = client.list_projects()?;
            match output {
                OutputMode::Json => print_json(&projects)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = projects
                        .iter()
                        .map(|p| {
                            vec![
                                p.title.clone(),
                                p.uuid.clone(),
                                p.path.clone().unwrap_or_else(|| "-".into()),
                            ]
                        })
                        .collect();
                    print_table(&["TITLE", "UUID", "PATH"], &rows);
                }
            }
            Ok(())
        }
        ProjectCommands::Get { title } => {
            let project = client.get_project(&title)?;
            match output {
                OutputMode::Json => print_json(&project)?,
                OutputMode::Text => {
                    println!("Title:       {}", project.title);
                    println!("UUID:        {}", project.uuid);
                    println!("ID:          {}", project.id);
                    println!("Path:        {}", project.path.as_deref().unwrap_or("-"));
                    println!("Repository:  {}", project.repository_uuid);
                }
            }
            Ok(())
        }
    }
}
