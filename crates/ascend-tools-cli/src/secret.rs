use std::io::{IsTerminal, Read};
use std::path::PathBuf;

use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::SecretValue;
use clap::Subcommand;

use crate::common::{OutputMode, print_json, print_subcommand_help, print_table};

#[derive(Subcommand)]
pub(crate) enum SecretCommands {
    /// List secrets in the instance vault (or an environment vault)
    List {
        /// Environment name — list environment-scoped secrets instead of instance-scoped
        #[arg(long)]
        environment: Option<String>,
    },
    /// Get a secret value (requires cloud admin)
    #[command(arg_required_else_help = true)]
    Get {
        /// Secret name
        name: String,
        /// Environment name (omit for instance scope)
        #[arg(long)]
        environment: Option<String>,
    },
    /// Create or update a secret
    #[command(
        arg_required_else_help = true,
        long_about = "Create or update a secret.\n\n\
            The secret value can be provided via --value, --from-file, or stdin.\n\
            Use --generate-ssh-key to generate an SSH private key instead.\n\n\
            Examples:\n  \
            ascend-tools secret set my-secret --value 'hunter2'\n  \
            ascend-tools secret set my-secret --from-file ./secret.txt\n  \
            echo 'hunter2' | ascend-tools secret set my-secret\n  \
            ascend-tools secret set ssh-key --generate-ssh-key\n  \
            ascend-tools secret set ssh-key --generate-ssh-key --algorithm rsa4096 --format pem"
    )]
    Set {
        /// Secret name
        name: String,
        /// Secret value (omit to read from stdin; use --generate-ssh-key for SSH keys)
        #[arg(long)]
        value: Option<String>,
        /// Read secret value from a file
        #[arg(long, conflicts_with = "value")]
        from_file: Option<PathBuf>,
        /// Generate an SSH private key instead of setting a value
        #[arg(long, conflicts_with_all = ["value", "from_file"])]
        generate_ssh_key: bool,
        /// SSH key algorithm: ed25519 (default), rsa4096, rsa3072, rsa2048
        #[arg(long, requires = "generate_ssh_key")]
        algorithm: Option<String>,
        /// SSH key format: openssh (default), pem, pkcs8
        #[arg(long, requires = "generate_ssh_key")]
        format: Option<String>,
        /// Environment name (omit for instance scope)
        #[arg(long)]
        environment: Option<String>,
    },
    /// Delete a secret
    #[command(arg_required_else_help = true)]
    Delete {
        /// Secret name
        name: String,
        /// Environment name (omit for instance scope)
        #[arg(long)]
        environment: Option<String>,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Get the SSH public key for a stored SSH private key (instance scope only)
    #[command(arg_required_else_help = true)]
    GetSshPublicKey {
        /// Secret name containing an SSH private key
        name: String,
    },
}

pub(crate) fn handle_secret(
    client: &AscendClient,
    cmd: Option<SecretCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        return print_subcommand_help("secret");
    };
    match cmd {
        SecretCommands::List { environment } => {
            let secrets = client.list_secrets(environment.as_deref())?;
            match output {
                OutputMode::Json => print_json(&secrets)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = secrets.into_iter().map(|s| vec![s]).collect();
                    print_table(&["NAME"], &rows);
                }
            }
        }
        SecretCommands::Get { name, environment } => {
            let secret = client.get_secret(&name, environment.as_deref())?;
            match output {
                OutputMode::Json => print_json(&secret)?,
                OutputMode::Text => {
                    println!("{}", secret.secret_value);
                }
            }
        }
        SecretCommands::Set {
            name,
            value,
            from_file,
            generate_ssh_key,
            algorithm,
            format,
            environment,
        } => {
            let secret_value = if generate_ssh_key {
                SecretValue::GenerateSshKey { algorithm, format }
            } else if let Some(v) = value {
                SecretValue::Value(v)
            } else if let Some(path) = from_file {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
                SecretValue::Value(content)
            } else {
                // Read from stdin
                let stdin = std::io::stdin();
                if stdin.is_terminal() {
                    eprint!("Enter secret value: ");
                    std::io::Write::flush(&mut std::io::stderr())?;
                    let mut line = String::new();
                    stdin.read_line(&mut line)?;
                    // Strip trailing newline from interactive input
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    SecretValue::Value(line)
                } else {
                    let mut buf = String::new();
                    stdin.lock().read_to_string(&mut buf)?;
                    SecretValue::Value(buf.trim_end_matches('\n').to_string())
                }
            };

            let result = client.set_secret(&name, &secret_value, environment.as_deref())?;
            match output {
                OutputMode::Json => print_json(&result)?,
                OutputMode::Text => println!("Secret '{}' {}", name, result.status),
            }
        }
        SecretCommands::Delete {
            name,
            environment,
            yes,
        } => {
            if !yes {
                eprint!("Delete secret '{name}'? [y/N] ");
                std::io::Write::flush(&mut std::io::stderr())?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    eprintln!("Cancelled.");
                    return Ok(());
                }
            }
            let result = client.delete_secret(&name, environment.as_deref())?;
            match output {
                OutputMode::Json => print_json(&result)?,
                OutputMode::Text => println!("Secret '{}' {}", name, result.status),
            }
        }
        SecretCommands::GetSshPublicKey { name } => {
            let key = client.get_secret_ssh_public_key(&name)?;
            match output {
                OutputMode::Json => print_json(&key)?,
                OutputMode::Text => {
                    println!("{}", key.public_key);
                }
            }
        }
    }
    Ok(())
}
