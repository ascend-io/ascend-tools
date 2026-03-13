# ascend-tools-tui

Interactive terminal UI for chatting with [Otto](https://www.ascend.io), the Ascend AI assistant.

Built on [`ascend-tools-core`](../ascend-tools-core), [`ratatui`](https://crates.io/crates/ratatui), and [`crossterm`](https://crates.io/crates/crossterm).

## Features

- Vi keybindings (default) with Emacs mode toggle
- Multi-line input (Alt+Enter for newlines)
- Input history persisted across sessions
- Smooth streaming output (~200 chars/sec)
- Markdown rendering (code blocks, bold, inline code)
- Tab completion for slash commands
- Clipboard copy (`/copy`)
- Message timestamps (`/timestamps`)
- Scrollable chat with scrollbar
- Cursor shape changes (block/bar) for Vi modes

## Usage

The TUI is typically launched via the CLI:

```bash
ascend-tools otto tui
ascend-tools otto tui --workspace my-ws
```

### As a library

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let config = Config::from_env()?;
let client = AscendClient::new(config)?;
ascend_tools_tui::run_tui(&client, None, None, None)?;
```

## Slash Commands

| Command | Description |
|---------|-------------|
| `/help` | Show commands and keybindings |
| `/vim`, `/vi` | Switch to Vi keybindings |
| `/emacs` | Switch to Emacs keybindings |
| `/copy` | Copy last Otto response to clipboard |
| `/timestamps` | Toggle message timestamps |
| `/clear` | Clear chat and start new thread |
| `/quit`, `/exit` | Exit |

See the [top-level README](../../../README.md) for full documentation.
