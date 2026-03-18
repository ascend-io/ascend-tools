# ascend-tools-tui

[![crates.io](https://img.shields.io/crates/v/ascend-tools-tui.svg)](https://crates.io/crates/ascend-tools-tui)

Interactive terminal UI for chatting with [Otto](https://www.ascend.io), the Ascend AI assistant.

Built on [`ascend-tools-core`](https://crates.io/crates/ascend-tools-core), [`ratatui`](https://crates.io/crates/ratatui), and [`crossterm`](https://crates.io/crates/crossterm).

## Usage

```bash
ascend-tools otto tui
ascend-tools otto tui --workspace "My Workspace"
```

### As a library

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let client = AscendClient::new(Config::from_env()?)?;
ascend_tools_tui::run_tui(&client, None, None, None)?;
```

## Features

- Vi keybindings (default) with Emacs mode toggle (`/emacs`)
- Multi-line input (Alt+Enter for newlines, grows up to 8 lines)
- Input history persisted across sessions (`~/.ascend-tools/history`)
- Smooth streaming output (~200 chars/sec) with spinner
- Markdown rendering (code blocks, bold, inline code)
- Scrollable chat (PageUp/Down, mouse wheel, Ctrl+U/D)
- Tab completion for slash commands
- Clipboard copy (`/copy`), timestamps (`/timestamps`)
- Cursor shape changes (block in vi normal, bar in insert/emacs)

## Slash commands

| Command | Description |
|---------|-------------|
| `/help` | Show commands and keybindings |
| `/vim`, `/vi` | Switch to Vi keybindings |
| `/emacs` | Switch to Emacs keybindings |
| `/copy` | Copy last Otto response to clipboard |
| `/timestamps` | Toggle message timestamps |
| `/clear` | Clear chat and start new thread |
| `/quit`, `/exit` | Exit |

See the [full documentation](https://github.com/ascend-io/ascend-tools) for more details.
