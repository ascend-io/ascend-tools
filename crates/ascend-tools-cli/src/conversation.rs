use anyhow::Result;
use ascend_tools::Error;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{
    Conversation, ConversationFilters, ConversationMessagesPage, ConversationOpen,
};
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
    /// Open a conversation via the progressive `/updates` contract
    #[command(arg_required_else_help = true)]
    Open {
        /// Conversation title or ID
        title_or_id: String,
        /// Treat the argument as an ID (skip title lookup)
        #[arg(long)]
        id: bool,
        /// Reopen from a previously loaded latest message ID
        #[arg(long)]
        after: Option<String>,
    },
    /// Fetch older conversation history before a known message ID
    #[command(arg_required_else_help = true)]
    History {
        /// Conversation title or ID
        title_or_id: String,
        /// Treat the argument as an ID (skip title lookup)
        #[arg(long)]
        id: bool,
        /// Fetch messages older than this message ID
        #[arg(long)]
        before: String,
        /// Maximum number of older messages to return
        #[arg(long, default_value = "20")]
        limit: u64,
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
        ConversationCommands::Open {
            title_or_id,
            id,
            after,
        } => {
            let resolved = resolve_conversation_target(client, &title_or_id, id)?;
            let open = client.open_conversation_progressive(&resolved, after.as_deref())?;
            match output {
                OutputMode::Json => print_json(&open)?,
                OutputMode::Text => print_progressive_open(&open),
            }
            Ok(())
        }
        ConversationCommands::History {
            title_or_id,
            id,
            before,
            limit,
        } => {
            let resolved = resolve_conversation_target(client, &title_or_id, id)?;
            let page = client.get_conversation_messages_before(&resolved, &before, Some(limit))?;
            match output {
                OutputMode::Json => print_json(&page)?,
                OutputMode::Text => print_history_page(&page),
            }
            Ok(())
        }
    }
}

fn resolve_conversation_target(
    client: &AscendClient,
    title_or_id: &str,
    id: bool,
) -> Result<String> {
    if id {
        Ok(title_or_id.to_string())
    } else {
        resolve_conversation_id_interactive(client, title_or_id)
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

fn print_progressive_open(open: &ConversationOpen) {
    match open {
        ConversationOpen::Preview(preview) => {
            println!("Kind:         preview");
            println!("ID:           {}", preview.id);
            println!("Title:        {}", preview.title.as_deref().unwrap_or("-"));
            println!(
                "Updated:      {}",
                preview.updated_at.as_deref().unwrap_or("-")
            );
            println!("Processing:   {}", preview.is_processing);
            println!("Total:        {}", preview.total_message_count);
            println!("Has more:     {}", preview.has_more);
            println!(
                "Oldest ID:    {}",
                preview.oldest_message_id.as_deref().unwrap_or("-")
            );
            println!(
                "Latest ID:    {}",
                preview.latest_message_id.as_deref().unwrap_or("-")
            );
            println!();
            for msg in preview.ordered_messages() {
                print_message(msg);
            }
        }
        ConversationOpen::Delta(delta) => {
            println!("Kind:         delta");
            println!("Title:        {}", delta.title.as_deref().unwrap_or("-"));
            println!(
                "Updated:      {}",
                delta.updated_at.as_deref().unwrap_or("-")
            );
            println!("Processing:   {}", delta.is_processing);
            println!("Messages:     {}", delta.messages.len());
            println!(
                "Latest ID:    {}",
                delta.latest_message_id.as_deref().unwrap_or("-")
            );
            println!();
            for msg in delta.ordered_messages() {
                print_message(msg);
            }
        }
    }
}

fn print_history_page(page: &ConversationMessagesPage) {
    println!("Messages:     {}", page.messages.len());
    println!("Has more:     {}", page.has_more);
    println!(
        "Oldest ID:    {}",
        page.oldest_message_id.as_deref().unwrap_or("-")
    );
    println!();
    for msg in page.ordered_messages() {
        print_message(msg);
    }
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
        match client.resolve_otto_thread(Some(&conv), None) {
            Ok(thread_id) => Ok(thread_id),
            Err(Error::AmbiguousTitle { matches, .. }) => {
                pick_from_matches(&conv, &matches).map(Some)
            }
            Err(e) => Err(e.into()),
        }
    } else {
        Ok(thread)
    }
}
