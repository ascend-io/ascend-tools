use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

const SKILL_CLI: &str = include_str!("skill-cli.md");
const SKILL_PY: &str = include_str!("skill-py.md");
const SKILL_JS: &str = include_str!("skill-js.md");
const SKILL_MCP: &str = include_str!("skill-mcp.md");

#[derive(Subcommand)]
pub(crate) enum SkillCommands {
    /// Install ascend-tools skill(s)
    #[command(arg_required_else_help = true)]
    Install {
        /// Target directory
        ///
        /// Examples: ./.claude/skills, ~/.claude/skills, ./.agents/skills
        #[arg(long, required = true)]
        target: String,
        /// Install the CLI skill (default if no flags given)
        #[arg(long)]
        cli: bool,
        /// Install the Python SDK skill
        #[arg(long)]
        python: bool,
        /// Install the JavaScript SDK skill
        #[arg(long)]
        javascript: bool,
        /// Install the MCP server skill
        #[arg(long)]
        mcp: bool,
        /// Install all skills
        #[arg(long)]
        all: bool,
    },
}

pub(crate) fn handle_skill(cmd: Option<SkillCommands>) -> Result<()> {
    let Some(cmd) = cmd else {
        return crate::common::print_subcommand_help("skill");
    };
    match cmd {
        SkillCommands::Install {
            target,
            cli,
            python,
            javascript,
            mcp,
            all,
        } => {
            let target = if target.starts_with("~/") || target == "~" {
                let home = std::env::var("HOME").context("HOME environment variable not set")?;
                PathBuf::from(target.replacen('~', &home, 1))
            } else {
                PathBuf::from(&target)
            };

            // --all enables everything; default to --cli when no flags are given
            let cli = all || cli || (!python && !javascript && !mcp);
            let python = all || python;
            let javascript = all || javascript;
            let mcp = all || mcp;

            for (dir_name, content, enabled) in [
                ("ascend-tools-cli", SKILL_CLI, cli),
                ("ascend-tools-python", SKILL_PY, python),
                ("ascend-tools-javascript", SKILL_JS, javascript),
                ("ascend-tools-mcp", SKILL_MCP, mcp),
            ] {
                if !enabled {
                    continue;
                }
                let dir = target.join(dir_name);
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("failed to create directory {}", dir.display()))?;
                let path = dir.join("SKILL.md");
                std::fs::write(&path, content)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                let abs = std::fs::canonicalize(&path).unwrap_or(path);
                println!("Installed {dir_name} skill to {}", abs.display());
            }
            Ok(())
        }
    }
}
