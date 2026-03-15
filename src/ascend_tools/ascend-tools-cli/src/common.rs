use anyhow::Result;
use ascend_tools::client::AscendClient;
use ascend_tools::models::{RuntimeCreate, RuntimeFilters, RuntimeKind, RuntimeUpdate};
use clap::ValueEnum;

#[derive(Clone, PartialEq, ValueEnum)]
pub(crate) enum OutputMode {
    Text,
    Json,
}

/// Print help for a subcommand when no sub-subcommand is given.
///
/// Accepts a space-separated path for nested subcommands (e.g. "otto provider").
pub(crate) fn print_subcommand_help(path: &str) -> Result<()> {
    use clap::CommandFactory;
    let mut cmd = crate::cli::CliParser::command();
    for part in path.split_whitespace() {
        cmd = cmd
            .find_subcommand_mut(part)
            .expect("subcommand exists")
            .clone();
    }
    cmd.print_help()?;
    Ok(())
}

pub(crate) fn handle_runtime_list(
    client: &AscendClient,
    kind: RuntimeKind,
    title: Option<String>,
    project: Option<String>,
    environment: Option<String>,
    output: &OutputMode,
) -> Result<()> {
    let mut filters = RuntimeFilters::default();
    filters.title = title;
    filters.kind = Some(kind);
    filters.project = project;
    filters.environment = environment;
    let runtimes = client.list_runtimes(filters)?;
    match output {
        OutputMode::Json => print_json(&runtimes)?,
        OutputMode::Text => {
            if kind == RuntimeKind::Deployment {
                let rows: Vec<Vec<String>> = runtimes
                    .iter()
                    .map(|r| {
                        vec![
                            r.title.clone(),
                            r.uuid.clone(),
                            display_health(r),
                            r.enable_automations
                                .map(|b| if b { "on" } else { "off" })
                                .unwrap_or("-")
                                .into(),
                        ]
                    })
                    .collect();
                print_table(&["TITLE", "UUID", "HEALTH", "AUTOMATIONS"], &rows);
            } else {
                let rows: Vec<Vec<String>> = runtimes
                    .iter()
                    .map(|r| {
                        vec![
                            r.title.clone(),
                            r.uuid.clone(),
                            display_health(r),
                            r.profile.as_deref().unwrap_or("-").to_owned(),
                        ]
                    })
                    .collect();
                print_table(&["TITLE", "UUID", "HEALTH", "PROFILE"], &rows);
            }
        }
    }
    Ok(())
}

pub(crate) fn handle_runtime_create(
    client: &AscendClient,
    kind: RuntimeKind,
    create: &RuntimeCreate,
    output: &OutputMode,
) -> Result<()> {
    let r = match kind {
        RuntimeKind::Workspace => client.create_workspace(create)?.0,
        RuntimeKind::Deployment => client.create_deployment(create)?.0,
    };
    match output {
        OutputMode::Json => print_json(&r)?,
        OutputMode::Text => println!("Created {kind} '{}' ({})", r.title, r.uuid),
    }
    Ok(())
}

pub(crate) fn handle_runtime_update(
    client: &AscendClient,
    kind: RuntimeKind,
    current_title: &str,
    uuid: Option<&str>,
    update: RuntimeUpdate,
    output: &OutputMode,
) -> Result<()> {
    let runtime_uuid = client.resolve_runtime_uuid(current_title, kind, uuid)?;
    let r = client.update_runtime(&runtime_uuid, &update)?;
    match output {
        OutputMode::Json => print_json(&r)?,
        OutputMode::Text => println!("Updated {kind} '{}' ({})", r.title, r.uuid),
    }
    Ok(())
}

pub(crate) fn handle_runtime_get(
    client: &AscendClient,
    kind: RuntimeKind,
    title: &str,
    uuid: Option<&str>,
    output: &OutputMode,
) -> Result<()> {
    // If UUID is provided, fetch directly. Otherwise resolve by title
    // (which already returns the full Runtime, avoiding a redundant GET).
    let r = if let Some(uuid) = uuid {
        client.get_runtime(uuid)?
    } else {
        client.resolve_runtime_by_title(title, kind)?
    };
    match output {
        OutputMode::Json => print_json(&r)?,
        OutputMode::Text => print_runtime_detail(&r),
    }
    Ok(())
}

pub(crate) fn handle_runtime_delete(
    client: &AscendClient,
    kind: RuntimeKind,
    title: &str,
    uuid: Option<&str>,
    yes: bool,
    output: &OutputMode,
) -> Result<()> {
    let runtime_uuid = client.resolve_runtime_uuid(title, kind, uuid)?;
    if !yes {
        eprint!("Delete {kind} '{title}'? [y/N] ");
        std::io::Write::flush(&mut std::io::stderr())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }
    client.delete_runtime(&runtime_uuid)?;
    match output {
        OutputMode::Json => print_json(&serde_json::json!({"deleted": runtime_uuid}))?,
        OutputMode::Text => println!("Deleted {kind} '{title}'"),
    }
    Ok(())
}

pub(crate) fn handle_runtime_pause(
    client: &AscendClient,
    kind: RuntimeKind,
    title: &str,
    uuid: Option<&str>,
    output: &OutputMode,
) -> Result<()> {
    let runtime_uuid = client.resolve_runtime_uuid(title, kind, uuid)?;
    let r = client.pause_runtime(&runtime_uuid)?;
    match output {
        OutputMode::Json => print_json(&r)?,
        OutputMode::Text => println!("Paused {kind} '{}'", r.title),
    }
    Ok(())
}

pub(crate) fn handle_runtime_resume(
    client: &AscendClient,
    kind: RuntimeKind,
    title: &str,
    uuid: Option<&str>,
    output: &OutputMode,
) -> Result<()> {
    let runtime_uuid = client.resolve_runtime_uuid(title, kind, uuid)?;
    let r = client.resume_runtime(&runtime_uuid)?;
    match output {
        OutputMode::Json => print_json(&r)?,
        OutputMode::Text => println!("Resumed {kind} '{}'", r.title),
    }
    Ok(())
}

pub(crate) fn display_health(r: &ascend_tools::models::Runtime) -> String {
    if r.paused {
        "paused".into()
    } else {
        r.health.as_deref().unwrap_or("-").to_owned()
    }
}

pub(crate) fn print_runtime_detail(r: &ascend_tools::models::Runtime) {
    println!("Title:        {}", r.title);
    println!("UUID:         {}", r.uuid);
    println!("ID:           {}", r.id);
    println!("Kind:         {}", r.kind);
    println!("Health:       {}", display_health(r));
    println!("Project:      {}", r.project_uuid);
    println!("Environment:  {}", r.environment_uuid);
    println!("Profile:      {}", r.profile.as_deref().unwrap_or("-"));
    println!("Branch:       {}", r.git_branch.as_deref().unwrap_or("-"));
    if let Some(automations) = r.enable_automations {
        println!("Automations:  {}", if automations { "on" } else { "off" });
    }
    if let Some(snooze) = r.auto_snooze_timeout_minutes {
        println!("Auto-snooze:  {} min", snooze);
    }
    println!("Build:        {}", r.build_uuid.as_deref().unwrap_or("-"));
    println!("Created:      {}", r.created_at);
    println!("Updated:      {}", r.updated_at);
}

pub(crate) fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Print rows as a fixed-width table with a header.
pub(crate) fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        eprintln!("No results.");
        return;
    }

    let widths: Vec<usize> = (0..headers.len())
        .map(|i| {
            let header_w = headers[i].len();
            let max_row_w = rows
                .iter()
                .map(|r| r.get(i).map_or(0, |s| s.len()))
                .max()
                .unwrap_or(0);
            header_w.max(max_row_w)
        })
        .collect();

    let last = headers.len() - 1;

    // Header
    for (i, h) in headers.iter().enumerate() {
        if i < last {
            print!("{:<width$}  ", h, width = widths[i]);
        } else {
            println!("{h}");
        }
    }

    // Rows
    for row in rows {
        for (i, val) in row.iter().enumerate() {
            if i < last {
                print!("{:<width$}  ", val, width = widths[i]);
            } else {
                println!("{val}");
            }
        }
    }
}

pub(crate) fn parse_spec(spec: Option<String>) -> Result<Option<serde_json::Value>> {
    spec.map(|s| serde_json::from_str(&s).map_err(|e| anyhow::anyhow!("invalid JSON spec: {e}")))
        .transpose()
}
