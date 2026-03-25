use anyhow::Result;
use ascend_tools::Error;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{Conversation, ConversationFilters};
use clap::Subcommand;
use serde_json::Value;

use crate::common::{OutputMode, print_json, print_subcommand_help, print_table};

#[derive(Subcommand)]
pub(crate) enum ConversationCommands {
    /// List recent conversations
    List {
        /// Maximum number of conversations to return
        #[arg(long, default_value = "40")]
        limit: u64,
        /// Pagination offset
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Get a conversation by title or ID
    #[command(arg_required_else_help = true)]
    Get {
        /// Conversation title or ID
        title_or_id: String,
        /// Treat the argument as an ID (skip title lookup)
        #[arg(long)]
        id: bool,
    },
}

pub(crate) fn handle_conversation(
    client: &AscendClient,
    cmd: Option<ConversationCommands>,
    output: &OutputMode,
) -> Result<()> {
    let Some(cmd) = cmd else {
        return print_subcommand_help("otto conversation");
    };
    match cmd {
        ConversationCommands::List { limit, offset } => {
            let mut filters = ConversationFilters::default();
            filters.offset = Some(offset);
            filters.limit = Some(limit);
            let list = client.list_conversations(filters)?;
            match output {
                OutputMode::Json => print_json(&list)?,
                OutputMode::Text => {
                    let rows: Vec<Vec<String>> = list
                        .threads
                        .iter()
                        .map(|c| {
                            vec![
                                c.title.as_deref().unwrap_or("-").to_owned(),
                                c.id.clone(),
                                c.updated_at.clone(),
                            ]
                        })
                        .collect();
                    print_table(&["TITLE", "ID", "UPDATED"], &rows);
                    if list.total > 0 {
                        eprintln!("Showing {}/{} conversations", rows.len(), list.total);
                    }
                }
            }
            Ok(())
        }
        ConversationCommands::Get { title_or_id, id } => {
            let conversation = if id {
                client.get_conversation(&title_or_id)?
            } else {
                let resolved = resolve_conversation_id_interactive(client, &title_or_id)?;
                client.get_conversation(&resolved)?
            };
            match output {
                OutputMode::Json => print_json(&conversation)?,
                OutputMode::Text => {
                    println!("ID:       {}", conversation.id);
                    println!("Title:    {}", conversation.title.as_deref().unwrap_or("-"));
                    println!("Updated:  {}", conversation.updated_at);
                    if let Some(stats) = &conversation.context_window_stats
                        && let Some(tokens) = stats.get("input_tokens").and_then(|v| v.as_u64())
                    {
                        println!("Tokens:   {tokens}");
                    }
                    if let Some(messages) = &conversation.messages {
                        println!();
                        for msg in messages {
                            print_message(msg);
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

/// Render a single message in text mode.
fn print_message(msg: &Value) {
    let role = msg
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Skip tool/function messages in text mode — they're noise
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if msg_type == "function_call"
        || msg_type == "function_call_output"
        || role == "tool"
        || role == "system"
    {
        return;
    }

    let text = Conversation::extract_message_text(msg);
    if text.is_empty() {
        return;
    }
    println!("{role}> {text}");
    println!();
}

/// Resolve a conversation title or ID to a thread ID for the CLI.
///
/// On ambiguous title matches, presents an interactive menu so the user can
/// pick. SDK clients should use `client.resolve_conversation_id()` instead
/// (which returns an error on ambiguity).
pub(crate) fn resolve_conversation_id_interactive(
    client: &AscendClient,
    title_or_id: &str,
) -> Result<String> {
    match client.resolve_conversation_id(title_or_id) {
        Ok(id) => Ok(id),
        Err(Error::AmbiguousTitle { matches, .. }) => pick_from_matches(title_or_id, &matches),
        Err(e) => Err(e.into()),
    }
}

/// Present numbered choices to the user and return the selected ID.
fn pick_from_matches(title: &str, matches: &[(String, String)]) -> Result<String> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("multiple conversations match '{title}', use --id to disambiguate");
    }
    eprintln!("Multiple conversations match '{title}'. Pick one (or Ctrl+C to cancel):\n");
    for (i, (id, t)) in matches.iter().enumerate() {
        eprintln!("  {}) {} ({})", i + 1, t, id);
    }
    eprint!("\n> ");
    std::io::Write::flush(&mut std::io::stderr())?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let choice: usize = input
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid selection"))?;
    matches
        .get(choice.wrapping_sub(1))
        .map(|(id, _)| id.clone())
        .ok_or_else(|| anyhow::anyhow!("selection out of range"))
}

/// Resolve `--conversation`, `--resume`, or `--thread` to a thread ID.
///
/// Returns `Ok(Some(thread_id))` if any flag was given, `Ok(None)` otherwise.
pub(crate) fn resolve_conversation_flag(
    client: &AscendClient,
    thread: Option<String>,
    conversation: Option<String>,
    resume: bool,
) -> Result<Option<String>> {
    if resume {
        Ok(Some(client.latest_conversation_id()?))
    } else if let Some(conv) = conversation {
        Ok(Some(resolve_conversation_id_interactive(client, &conv)?))
    } else {
        Ok(thread)
    }
}
