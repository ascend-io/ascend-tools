use anyhow::Result;
use ascend_tools::config::{config_file_path, instance_config};
use clap::Subcommand;

use crate::common::{OutputMode, print_subcommand_help, print_table};

#[derive(Subcommand)]
pub(crate) enum InstanceCommands {
    /// Add or update an instance configuration
    Add {
        /// Instance name (used with --instance flag)
        name: String,
        /// Service account ID
        #[arg(long)]
        service_account_id: String,
        /// Instance API URL
        #[arg(long)]
        instance_api_url: String,
        /// Environment variable name containing the service account key (not the key itself)
        #[arg(long, default_value = "ASCEND_SERVICE_ACCOUNT_KEY")]
        service_account_key_env: String,
    },
    /// List configured instances
    List,
    /// Remove an instance configuration
    Remove {
        /// Instance name to remove
        name: String,
    },
    /// Set the default instance
    SetDefault {
        /// Instance name to make the default
        name: String,
    },
}

pub(crate) fn handle_instance(
    command: Option<InstanceCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(command) = command else {
        return print_subcommand_help("instance");
    };

    match command {
        InstanceCommands::Add {
            name,
            service_account_id,
            instance_api_url,
            service_account_key_env,
        } => {
            instance_config::add(
                &name,
                &service_account_id,
                &instance_api_url,
                &service_account_key_env,
            )?;
            if let Some(path) = config_file_path() {
                eprintln!(
                    "Instance '{name}' saved to {} (key env: {service_account_key_env})",
                    path.display()
                );
            } else {
                eprintln!("Instance '{name}' saved (key env: {service_account_key_env}).");
            }
            Ok(())
        }
        InstanceCommands::List => {
            let (default_name, entries) = instance_config::list()?;

            if entries.is_empty() {
                eprintln!(
                    "No instances configured. Run `ascend-tools instance add` to get started."
                );
                return Ok(());
            }

            match output {
                OutputMode::Json => {
                    let items: Vec<serde_json::Value> = entries
                        .iter()
                        .map(|(name, entry)| {
                            serde_json::json!({
                                "name": name,
                                "default": *name == default_name,
                                "service_account_id": entry.service_account_id,
                                "instance_api_url": entry.instance_api_url,
                                "service_account_key_env": entry.service_account_key_env,
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&items)?);
                }
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = entries
                        .iter()
                        .map(|(name, entry)| {
                            let display_name = if *name == default_name {
                                format!("{name} *")
                            } else {
                                name.clone()
                            };
                            vec![
                                display_name,
                                entry.service_account_id.clone(),
                                entry.instance_api_url.clone(),
                                entry.service_account_key_env.clone(),
                            ]
                        })
                        .collect();
                    print_table(
                        &["NAME", "SERVICE_ACCOUNT_ID", "INSTANCE_API_URL", "KEY_ENV"],
                        &rows,
                    );
                }
            }

            Ok(())
        }
        InstanceCommands::Remove { name } => {
            instance_config::remove(&name)?;
            eprintln!("Instance '{name}' removed.");
            Ok(())
        }
        InstanceCommands::SetDefault { name } => {
            instance_config::set_default(&name)?;
            eprintln!("Default instance set to '{name}'.");
            Ok(())
        }
    }
}
