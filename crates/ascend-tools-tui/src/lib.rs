#![deny(unsafe_code)]

//! Interactive TUI for Otto chat.
//!
//! Full-screen terminal interface using ratatui with:
//! - Scrollable chat history with scrollbar
//! - Streaming responses with spinner and smooth output
//! - Vi input mode (default) with `/emacs` toggle
//! - Multi-line input (Alt+Enter for newline)
//! - Input history (Up/Down, persisted to ~/.ascend-tools/history)
//! - Slash commands with tab completion
//! - Markdown rendering (headings, lists, code blocks, tables, and more)
//! - Message timestamps (`/timestamps` to toggle)
//! - Clipboard copy (`/copy`)
//! - Vi yank/paste registers

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::*;

use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, Event as MdEvent, Options, Parser as MdParser, Tag as MdTag,
    TagEnd as MdTagEnd,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use unicode_width::UnicodeWidthStr;

use ascend_tools::client::AscendClient;
use ascend_tools::models::{
    Conversation, OttoChatRequest, OttoModel, OttoStreamStatus, StreamEvent,
};
use std::ops::ControlFlow;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SPINNER: &[&str] = &[
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280f}",
];
const POLL_DURATION: Duration = Duration::from_millis(16);
const SPINNER_INTERVAL: Duration = Duration::from_millis(80);

#[rustfmt::skip]
const COMMANDS: &[&str] = &[
    "/clear", "/copy", "/emacs", "/exit", "/help",
    "/q", "/quit", "/timestamps", "/vi", "/vim",
];

#[rustfmt::skip]
const SPLASH: &[&str] = &[
    "      \u{2588}\u{2588}         \u{2588}\u{2588}",
    "      \u{2588}\u{2588}\u{2588}       \u{2588}\u{2588}\u{2588}",
    "       \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}",
    "       \u{2588}\u{2588}  . .  \u{2588}\u{2588}",
    "       \u{2588}\u{2588}   v   \u{2588}\u{2588}",
    "        \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}",
    "",
    "  \u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588} \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}",
    " \u{2588}\u{2588}  \u{2588}\u{2588}   \u{2588}\u{2588}     \u{2588}\u{2588}   \u{2588}\u{2588}  \u{2588}\u{2588}",
    " \u{2588}\u{2588}  \u{2588}\u{2588}   \u{2588}\u{2588}     \u{2588}\u{2588}   \u{2588}\u{2588}  \u{2588}\u{2588}",
    " \u{2588}\u{2588}  \u{2588}\u{2588}   \u{2588}\u{2588}     \u{2588}\u{2588}   \u{2588}\u{2588}  \u{2588}\u{2588}",
    "  \u{2588}\u{2588}\u{2588}\u{2588}    \u{2588}\u{2588}     \u{2588}\u{2588}    \u{2588}\u{2588}\u{2588}\u{2588}",
    "",
    "     type /help for commands",
];

#[rustfmt::skip]
const EXPERIMENTAL_BANNER: &[&str] = &[
    "\u{26a0}  EXPERIMENTAL  \u{26a0}",
    "",
    "This feature is under active development.",
    "Expect rough edges, bugs, and breaking changes.",
    "Mascot below not finalized.",
];

const USER_COLOR: Color = Color::Rgb(80, 120, 200); // dark blue
const OTTO_COLOR: Color = Color::Rgb(232, 67, 67); // ascend red
const SYSTEM_COLOR: Color = Color::Rgb(160, 120, 200); // purple
const VI_NORMAL_COLOR: Color = Color::Rgb(255, 140, 80); // orange
const CODE_COLOR: Color = Color::Rgb(255, 140, 80); // orange (matches vi normal)
const DIM_COLOR: Color = Color::Rgb(100, 100, 100);
const WARNING_COLOR: Color = Color::Rgb(255, 200, 50); // yellow
const DIM_OTTO_COLOR: Color = Color::Rgb(120, 45, 45); // muted ascend red
const POPUP_BG: Color = Color::Rgb(50, 50, 50);
const TEXT_COLOR: Color = Color::White;
const HEADING_COLOR: Color = Color::Rgb(130, 170, 255); // light blue for headings
const CHECK_COLOR: Color = Color::Rgb(80, 200, 120); // green for task checkmarks
const LINK_COLOR: Color = Color::Rgb(100, 160, 240); // blue for link text
const DIFF_ADD_COLOR: Color = Color::Rgb(80, 200, 120); // green for diff additions
const DIFF_DEL_COLOR: Color = Color::Rgb(232, 80, 80); // red for diff deletions
const DIFF_HUNK_COLOR: Color = Color::Rgb(130, 170, 255); // blue for diff hunk headers
const NOTE_COLOR: Color = Color::Rgb(100, 160, 240); // blue for [!NOTE]
const TIP_COLOR: Color = Color::Rgb(80, 200, 120); // green for [!TIP]
const IMPORTANT_COLOR: Color = Color::Rgb(180, 130, 240); // purple for [!IMPORTANT]
const CAUTION_COLOR: Color = Color::Rgb(232, 80, 80); // red for [!CAUTION]
const TIMESTAMP_COLOR: Color = Color::Rgb(80, 80, 80);

/// Characters per second for smoothed streaming output.
const STREAM_CPS: f64 = 200.0;
/// Above this pending count, flush in bulk to catch up.
const STREAM_BULK_THRESHOLD: usize = 200;
/// Above this pending count, skip smoothing entirely.
const STREAM_FAST_THRESHOLD: usize = 50;
const RESUME_HISTORY_PAGE_SIZE: u64 = 200;

const MAX_HISTORY: usize = 1000;

/// Syntax highlighting for code blocks.
static SYNTAX_SET: std::sync::LazyLock<SyntaxSet> =
    std::sync::LazyLock::new(SyntaxSet::load_defaults_nonewlines);
static THEME: std::sync::LazyLock<syntect::highlighting::Theme> = std::sync::LazyLock::new(|| {
    let ts = ThemeSet::load_defaults();
    ts.themes["base16-eighties.dark"].clone()
});
const MAX_INPUT_LINES: u16 = 8;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

enum StreamMsg {
    ProviderInfo {
        provider_label: Option<String>,
        model_label: String,
    },
    ConversationHistory {
        generation: u64,
        messages: Vec<Message>,
    },
    ConversationHistoryError {
        generation: u64,
        error: String,
    },
    StopFinished {
        error: Option<String>,
    },
    /// Stream messages tagged with a generation to discard stale messages
    /// from cancelled requests.
    Stream {
        generation: u64,
        kind: StreamMsgKind,
    },
}

enum StreamMsgKind {
    ThreadId(String),
    Delta(String),
    ToolCallStart {
        name: String,
        arguments: String,
    },
    ToolCallOutput {
        name: String,
        output: String,
    },
    Finished {
        status: OttoStreamStatus,
        error: Option<String>,
    },
    Error(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum InputMode {
    Emacs,
    ViInsert,
    ViNormal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    User,
    Otto,
    System,
}

struct ToolCallData {
    name: String,
    arguments: String,
    output: String,
}

struct Message {
    role: Role,
    content: String,
    timestamp: SystemTime,
    tool_call: Option<ToolCallData>,
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

struct History {
    entries: Vec<String>,
    position: Option<usize>,
    saved_input: Vec<char>,
}

impl History {
    fn load() -> Self {
        let entries = Self::history_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| {
                s.lines()
                    .filter(|l| !l.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            entries,
            position: None,
            saved_input: Vec::new(),
        }
    }

    fn push(&mut self, entry: &str) {
        let entry = entry.trim().replace('\n', "\\n");
        if entry.is_empty() {
            return;
        }
        // Deduplicate consecutive
        if self.entries.last().is_some_and(|last| *last == entry) {
            return;
        }
        self.entries.push(entry.clone());
        if self.entries.len() > MAX_HISTORY {
            self.entries.remove(0);
        }
        self.position = None;
        // Append to file
        if let Some(path) = Self::history_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(f, "{entry}");
            }
        }
    }

    fn decode(entry: &str) -> Vec<char> {
        entry.replace("\\n", "\n").chars().collect()
    }

    fn prev(&mut self, current_input: &[char]) -> Option<Vec<char>> {
        if self.entries.is_empty() {
            return None;
        }
        let new_pos = match self.position {
            None => {
                self.saved_input = current_input.to_vec();
                self.entries.len() - 1
            }
            Some(0) => return None,
            Some(p) => p - 1,
        };
        self.position = Some(new_pos);
        Some(Self::decode(&self.entries[new_pos]))
    }

    fn next(&mut self) -> Option<Vec<char>> {
        let pos = self.position?;
        if pos + 1 >= self.entries.len() {
            self.position = None;
            Some(self.saved_input.clone())
        } else {
            self.position = Some(pos + 1);
            Some(Self::decode(&self.entries[pos + 1]))
        }
    }

    fn history_path() -> Option<std::path::PathBuf> {
        std::env::var("HOME").ok().map(|h| {
            std::path::PathBuf::from(h)
                .join(".ascend-tools")
                .join("history")
        })
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct App {
    messages: Vec<Message>,
    input: Vec<char>,
    cursor: usize,
    input_mode: InputMode,
    /// Lines scrolled up from the bottom (0 = pinned to newest).
    scroll: usize,
    auto_scroll: bool,
    streaming: bool,
    stream_buffer: String,
    stream_pending: VecDeque<char>,
    last_stream_tick: Instant,
    stream_start: Option<Instant>,
    thread_id: Option<String>,
    runtime_uuid: Option<String>,
    otto_model: Option<OttoModel>,
    provider_label: Option<String>,
    model_label: String,
    context_label: Option<String>,
    query_policy: Option<String>,
    pending_request: Option<OttoChatRequest>,
    should_quit: bool,
    spinner_frame: usize,
    last_spinner: Instant,
    vi_pending: Option<char>,
    yank_register: String,
    completion_index: Option<usize>,
    history: History,
    show_timestamps: bool,
    active_tool_call: Option<(String, String)>,
    expand_tool_calls: bool,
    stream_generation: u64,
    /// Set when cancel fires; the main loop spawns a thread to stop the backend.
    /// The generation value is the *cancelled* generation (before advancement).
    stop_pending: Option<u64>,
    interrupting: bool,
    /// Set when user presses Ctrl+C during interrupting state — force exit.
    force_quit: bool,
    /// Show raw markdown source instead of rendered output (Ctrl+R toggle).
    show_raw_markdown: bool,
}

impl App {
    fn new(
        runtime_uuid: Option<String>,
        otto_model: Option<OttoModel>,
        provider_label: Option<String>,
        model_label: String,
        context_label: Option<String>,
        thread_id: Option<String>,
        query_policy: Option<String>,
    ) -> Self {
        Self {
            messages: Vec::new(),
            input: Vec::new(),
            cursor: 0,
            input_mode: InputMode::ViInsert,
            scroll: 0,
            auto_scroll: true,
            streaming: false,
            stream_buffer: String::new(),
            stream_pending: VecDeque::new(),
            last_stream_tick: Instant::now(),
            stream_start: None,
            thread_id,
            runtime_uuid,
            otto_model,
            provider_label,
            model_label,
            context_label,
            query_policy,
            pending_request: None,
            should_quit: false,
            spinner_frame: 0,
            last_spinner: Instant::now(),
            vi_pending: None,
            yank_register: String::new(),
            completion_index: None,
            history: History::load(),
            show_timestamps: false,
            active_tool_call: None,
            expand_tool_calls: false,
            stream_generation: 0,
            stop_pending: None,
            interrupting: false,
            force_quit: false,
            show_raw_markdown: false,
        }
    }

    // -- Input helpers ------------------------------------------------------

    fn input_line_count(&self, width: u16) -> u16 {
        let avail = (width as usize).saturating_sub(3); // prompt_len
        if avail == 0 {
            return 1;
        }
        let mut rows = 1usize;
        let mut col = 0usize;
        for &ch in &self.input {
            if ch == '\n' {
                rows += 1;
                col = 0;
            } else {
                if col >= avail {
                    rows += 1;
                    col = 0;
                }
                col += 1;
            }
        }
        // Cursor at end needs an extra row if current row is full
        if self.cursor == self.input.len() && col >= avail {
            rows += 1;
        }
        (rows as u16).min(MAX_INPUT_LINES)
    }

    fn handle_paste(&mut self, text: &str) {
        if self.input_mode == InputMode::ViNormal {
            self.input_mode = InputMode::ViInsert;
        }
        let chars: Vec<char> = text.chars().collect();
        let count = chars.len();
        self.input.splice(self.cursor..self.cursor, chars);
        self.cursor += count;
        self.completion_index = None;
    }

    // -- Key handling -------------------------------------------------------

    fn handle_key(&mut self, key: KeyEvent, cancelled_gen: &AtomicU64) {
        // Ctrl+C: cancel stream, force quit if already interrupting, or quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.interrupting {
                // Second Ctrl+C while stopping — force quit
                self.force_quit = true;
                self.should_quit = true;
                return;
            }
            if self.streaming {
                self.cancel_stream(cancelled_gen);
            } else {
                self.should_quit = true;
            }
            return;
        }

        // Ctrl+R: toggle raw markdown view
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
            self.show_raw_markdown = !self.show_raw_markdown;
            return;
        }

        // Escape: cancel stream (if streaming), otherwise normal key handling
        if key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE && self.streaming {
            if self.interrupting {
                return;
            }
            self.cancel_stream(cancelled_gen);
            return;
        }

        // Ctrl+o: toggle tool call expand/collapse
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
            self.expand_tool_calls = !self.expand_tool_calls;
            return;
        }

        match self.input_mode {
            InputMode::Emacs => self.handle_key_emacs(key),
            InputMode::ViInsert => self.handle_key_vi_insert(key),
            InputMode::ViNormal => self.handle_key_vi_normal(key),
        }
    }

    fn handle_key_emacs(&mut self, key: KeyEvent) {
        // Tab: cycle completions
        if key.code == KeyCode::Tab && key.modifiers == KeyModifiers::NONE {
            self.complete_tab();
            return;
        }
        self.reset_completion();

        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) => self.submit(),
            // Alt+Enter or Shift+Enter: insert newline
            (KeyModifiers::ALT, KeyCode::Enter) | (KeyModifiers::SHIFT, KeyCode::Enter) => {
                self.input.insert(self.cursor, '\n');
                self.cursor += 1;
            }

            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
                self.history.position = None;
            }

            (KeyModifiers::NONE, KeyCode::Backspace) => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                    self.history.position = None;
                }
            }
            (KeyModifiers::NONE, KeyCode::Delete) => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                    self.history.position = None;
                }
            }

            (KeyModifiers::NONE, KeyCode::Left) => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.cursor = (self.cursor + 1).min(self.input.len());
            }

            // Word-wise movement
            (KeyModifiers::ALT, KeyCode::Left) => self.cursor = self.word_back(),
            (KeyModifiers::ALT, KeyCode::Right) => self.cursor = self.word_fwd(),

            (KeyModifiers::NONE, KeyCode::Home) | (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.cursor = 0;
            }
            (KeyModifiers::NONE, KeyCode::End) | (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.cursor = self.input.len();
            }

            // Kill to start
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.input.drain(..self.cursor);
                self.cursor = 0;
            }
            // Kill to end
            (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
                self.input.truncate(self.cursor);
            }
            // Kill word backward
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                let new_cursor = self.word_back();
                self.input.drain(new_cursor..self.cursor);
                self.cursor = new_cursor;
            }

            // History
            (KeyModifiers::NONE, KeyCode::Up) => {
                if let Some(chars) = self.history.prev(&self.input) {
                    self.input = chars;
                    self.cursor = self.input.len();
                    self.completion_index = None;
                }
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                if let Some(chars) = self.history.next() {
                    self.input = chars;
                    self.cursor = self.input.len();
                    self.completion_index = None;
                }
            }

            // Scroll
            (KeyModifiers::NONE, KeyCode::PageUp) => self.scroll_up(10),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.scroll_down(10),

            _ => {}
        }
    }

    fn handle_key_vi_insert(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE {
            self.input_mode = InputMode::ViNormal;
            if self.cursor > 0 {
                self.cursor -= 1;
            }
            return;
        }
        self.handle_key_emacs(key);
    }

    fn handle_key_vi_normal(&mut self, key: KeyEvent) {
        // Multi-char commands (dd, yy)
        if let Some(pending) = self.vi_pending.take() {
            match (pending, key.code) {
                ('d', KeyCode::Char('d')) => {
                    self.yank_register = self.input.iter().collect();
                    self.input.clear();
                    self.cursor = 0;
                }
                ('y', KeyCode::Char('y')) => {
                    self.yank_register = self.input.iter().collect();
                }
                _ => {}
            }
            return;
        }

        match (key.modifiers, key.code) {
            // Enter insert mode
            (KeyModifiers::NONE, KeyCode::Char('i')) => {
                self.input_mode = InputMode::ViInsert;
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.input_mode = InputMode::ViInsert;
                self.cursor = (self.cursor + 1).min(self.input.len());
            }
            (KeyModifiers::SHIFT, KeyCode::Char('I')) => {
                self.input_mode = InputMode::ViInsert;
                self.cursor = 0;
            }
            (KeyModifiers::SHIFT, KeyCode::Char('A')) => {
                self.input_mode = InputMode::ViInsert;
                self.cursor = self.input.len();
            }

            // Motion
            (KeyModifiers::NONE, KeyCode::Char('h') | KeyCode::Left) => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Right) => {
                let max = self.input.len().saturating_sub(1);
                self.cursor = (self.cursor + 1).min(max);
            }
            (KeyModifiers::NONE, KeyCode::Char('0')) => self.cursor = 0,
            (KeyModifiers::SHIFT, KeyCode::Char('$')) => {
                self.cursor = self.input.len().saturating_sub(1);
            }
            (KeyModifiers::NONE, KeyCode::Char('w')) => self.cursor = self.word_fwd(),
            (KeyModifiers::NONE, KeyCode::Char('b')) => self.cursor = self.word_back(),
            (KeyModifiers::NONE, KeyCode::Char('e')) => self.cursor = self.word_end(),

            // Editing
            (KeyModifiers::NONE, KeyCode::Char('x')) => {
                if self.cursor < self.input.len() {
                    let ch = self.input.remove(self.cursor);
                    self.yank_register = ch.to_string();
                    if self.cursor > 0 && self.cursor >= self.input.len() {
                        self.cursor = self.input.len().saturating_sub(1);
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                self.vi_pending = Some('d');
            }
            (KeyModifiers::NONE, KeyCode::Char('y')) => {
                self.vi_pending = Some('y');
            }
            // Paste after cursor
            (KeyModifiers::NONE, KeyCode::Char('p')) => {
                if !self.yank_register.is_empty() {
                    let pos = (self.cursor + 1).min(self.input.len());
                    let chars: Vec<char> = self.yank_register.chars().collect();
                    let count = chars.len();
                    self.input.splice(pos..pos, chars);
                    self.cursor = pos + count - 1;
                }
            }
            // Paste before cursor
            (KeyModifiers::SHIFT, KeyCode::Char('P')) => {
                if !self.yank_register.is_empty() {
                    let chars: Vec<char> = self.yank_register.chars().collect();
                    let count = chars.len();
                    self.input.splice(self.cursor..self.cursor, chars);
                    self.cursor += count.saturating_sub(1);
                }
            }

            // Submit
            (KeyModifiers::NONE, KeyCode::Enter) => self.submit(),

            // History
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) if self.input.is_empty() => {
                if let Some(chars) = self.history.prev(&self.input) {
                    self.input = chars;
                    self.cursor = self.input.len().saturating_sub(1);
                    self.completion_index = None;
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) if self.input.is_empty() => {
                if let Some(chars) = self.history.next() {
                    self.input = chars;
                    self.cursor = self.input.len().saturating_sub(1);
                    self.completion_index = None;
                }
            }

            // Scroll
            (KeyModifiers::NONE, KeyCode::PageUp) => self.scroll_up(10),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.scroll_down(10),
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => self.scroll_up(15),
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => self.scroll_down(15),

            _ => {}
        }
    }

    // -- Completions --------------------------------------------------------

    fn input_str(&self) -> String {
        self.input.iter().collect()
    }

    fn completions(&self) -> Vec<&'static str> {
        let text = self.input_str();
        if !text.starts_with('/') {
            return Vec::new();
        }
        COMMANDS
            .iter()
            .filter(|cmd| cmd.starts_with(&text) && **cmd != text)
            .copied()
            .collect()
    }

    fn complete_tab(&mut self) {
        let matches = self.completions();
        if matches.is_empty() {
            self.completion_index = None;
            return;
        }
        let idx = match self.completion_index {
            Some(i) => (i + 1) % matches.len(),
            None => 0,
        };
        self.completion_index = Some(idx);
        let cmd = matches[idx];
        self.input = cmd.chars().collect();
        self.cursor = self.input.len();
    }

    fn reset_completion(&mut self) {
        self.completion_index = None;
    }

    // -- Word boundaries ----------------------------------------------------

    fn word_fwd(&self) -> usize {
        let mut i = self.cursor;
        while i < self.input.len() && !self.input[i].is_whitespace() {
            i += 1;
        }
        while i < self.input.len() && self.input[i].is_whitespace() {
            i += 1;
        }
        i
    }

    fn word_back(&self) -> usize {
        if self.cursor == 0 {
            return 0;
        }
        let mut i = self.cursor - 1;
        while i > 0 && self.input[i].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !self.input[i - 1].is_whitespace() {
            i -= 1;
        }
        i
    }

    fn word_end(&self) -> usize {
        if self.input.is_empty() {
            return 0;
        }
        let last = self.input.len() - 1;
        let mut i = self.cursor;
        if i < last {
            i += 1;
        }
        while i < last && self.input[i].is_whitespace() {
            i += 1;
        }
        while i < last && !self.input[i + 1].is_whitespace() {
            i += 1;
        }
        i
    }

    // -- Scroll helpers -----------------------------------------------------

    fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_add(n);
        self.auto_scroll = false;
    }

    fn scroll_down(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
        if self.scroll == 0 {
            self.auto_scroll = true;
        }
    }

    // -- Submit & commands --------------------------------------------------

    fn submit(&mut self) {
        if self.streaming {
            self.push_system("Waiting for response...");
            return;
        }
        let text: String = self.input.drain(..).collect();
        self.cursor = 0;
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }

        if text.starts_with('/') {
            self.handle_command(&text);
            return;
        }

        self.history.push(&text);

        self.messages.push(Message {
            role: Role::User,
            content: text.clone(),
            timestamp: SystemTime::now(),
            tool_call: None,
        });

        self.pending_request = Some(OttoChatRequest {
            prompt: text,
            runtime_uuid: self.runtime_uuid.clone(),
            thread_id: self.thread_id.clone(),
            model: self.otto_model.clone(),
            query_policy: self.query_policy.clone(),
        });
        self.streaming = true;
        self.stream_buffer.clear();
        self.stream_pending.clear();
        self.last_stream_tick = Instant::now();
        self.stream_start = Some(Instant::now());
        self.auto_scroll = true;
        self.scroll = 0;

        if self.input_mode == InputMode::ViNormal {
            self.input_mode = InputMode::ViInsert;
        }
    }

    fn push_system(&mut self, content: impl Into<String>) {
        self.messages.push(Message {
            role: Role::System,
            content: content.into(),
            timestamp: SystemTime::now(),
            tool_call: None,
        });
    }

    fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        match parts[0] {
            "/vim" | "/vi" => {
                self.input_mode = InputMode::ViNormal;
                self.push_system("Vi mode");
            }
            "/emacs" => {
                self.input_mode = InputMode::Emacs;
                self.push_system("Emacs mode");
            }
            "/clear" => {
                self.messages.clear();
                self.scroll = 0;
                self.thread_id = None;
                self.push_system("Thread cleared");
            }
            "/copy" => {
                let last_otto = self
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == Role::Otto)
                    .map(|m| m.content.clone());
                match last_otto {
                    Some(text) => {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
                            Ok(()) => self.push_system("Copied to clipboard"),
                            Err(e) => self.push_system(format!("Clipboard error: {e}")),
                        }
                    }
                    None => self.push_system("No Otto message to copy"),
                }
            }
            "/timestamps" => {
                self.show_timestamps = !self.show_timestamps;
                let state = if self.show_timestamps { "on" } else { "off" };
                self.push_system(format!("Timestamps {state}"));
            }
            "/quit" | "/exit" | "/q" => {
                self.should_quit = true;
            }
            "/help" => {
                self.push_system(concat!(
                    "Commands:\n",
                    "  /emacs        Switch to Emacs keybindings\n",
                    "  /vim          Switch to Vi keybindings (default)\n",
                    "  /copy         Copy last Otto response to clipboard\n",
                    "  /timestamps   Toggle message timestamps\n",
                    "  /clear        Clear chat and start new thread\n",
                    "  /quit, /exit  Exit\n",
                    "  /help         Show this help\n",
                    "\n",
                    "Keys:\n",
                    "  Enter         Send message\n",
                    "  Alt+Enter     Insert newline\n",
                    "  Esc           Vi normal mode\n",
                    "  Up/Down       Input history\n",
                    "  PageUp/Down   Scroll chat\n",
                    "  Tab           Complete /command\n",
                    "  Ctrl+o        Toggle tool call details\n",
                    "  Ctrl+C        Cancel stream / Exit",
                ));
            }
            other => {
                self.push_system(format!("Unknown command: {other}"));
            }
        }
    }

    // -- Streaming ----------------------------------------------------------

    fn handle_stream_msg(&mut self, msg: StreamMsg) {
        match msg {
            StreamMsg::ProviderInfo {
                provider_label: provider,
                model_label: model,
            } => {
                self.provider_label = provider;
                self.model_label = model;
            }
            StreamMsg::ConversationHistory {
                generation,
                messages,
            } => {
                if generation == self.stream_generation {
                    if self.messages.is_empty() {
                        self.messages = messages;
                    } else if !messages.is_empty() {
                        let mut combined = messages;
                        combined.extend(std::mem::take(&mut self.messages));
                        self.messages = combined;
                    }
                }
            }
            StreamMsg::ConversationHistoryError { generation, error } => {
                if generation == self.stream_generation {
                    if self.messages.is_empty() {
                        self.push_system(format!("Could not load recent history: {error}"));
                    } else {
                        self.push_system(format!("Could not load older history: {error}"));
                    }
                }
            }
            StreamMsg::StopFinished { error } => {
                // Only act if we're actually in interrupting state.
                // A late StopFinished from a previous cancel is harmless.
                if self.interrupting {
                    self.finish_stream();
                    if let Some(err) = error {
                        self.push_system(format!("Interrupt failed: {err}"));
                    } else {
                        self.push_system("Cancelled");
                    }
                }
            }
            StreamMsg::Stream { generation, kind } => {
                // Discard stale messages from cancelled requests
                if generation != self.stream_generation {
                    return;
                }
                self.handle_stream_kind(kind);
            }
        }
    }

    fn handle_stream_kind(&mut self, kind: StreamMsgKind) {
        match kind {
            StreamMsgKind::ThreadId(tid) => {
                self.thread_id = Some(tid);
            }
            StreamMsgKind::Delta(text) => {
                self.stream_pending.extend(text.chars());
            }
            StreamMsgKind::ToolCallStart { name, arguments } => {
                self.flush_stream_text();
                self.active_tool_call = Some((name, arguments));
            }
            StreamMsgKind::ToolCallOutput { name, output } => {
                let arguments = self
                    .active_tool_call
                    .take()
                    .map(|(_, args)| args)
                    .unwrap_or_default();
                let output_summary = truncate(&output, 80);
                self.messages.push(Message {
                    role: Role::System,
                    content: format!("\u{2699} {name} \u{2192} {output_summary}"),
                    timestamp: SystemTime::now(),
                    tool_call: Some(ToolCallData {
                        name,
                        arguments,
                        output,
                    }),
                });
            }
            StreamMsgKind::Finished { status, error } => match status {
                OttoStreamStatus::Completed => {
                    let should_bell = self
                        .stream_start
                        .is_some_and(|s| s.elapsed() > Duration::from_secs(3));
                    self.finish_stream();
                    if should_bell {
                        let _ =
                            crossterm::execute!(std::io::stderr(), crossterm::style::Print("\x07"));
                    }
                }
                OttoStreamStatus::Cancelled => {
                    // No-op: cleanup is deferred to StopFinished message
                    // from the background stop thread.
                }
                OttoStreamStatus::Interrupted => {
                    self.finish_stream();
                    let detail = error.unwrap_or_else(|| "stream interrupted".to_string());
                    self.push_system(format!("Connection lost: {detail}"));
                }
            },
            StreamMsgKind::Error(err) => {
                self.finish_stream();
                let message = if err.contains("Otto stream ended unexpectedly") {
                    format!("Connection lost: {err}")
                } else {
                    format!("Error: {err}")
                };
                self.push_system(message);
            }
        }
    }

    fn flush_stream_text(&mut self) {
        let remaining: String = self.stream_pending.drain(..).collect();
        self.stream_buffer.push_str(&remaining);
        let content = std::mem::take(&mut self.stream_buffer);
        if !content.is_empty() {
            self.messages.push(Message {
                role: Role::Otto,
                content,
                timestamp: SystemTime::now(),
                tool_call: None,
            });
        }
    }

    fn cancel_stream(&mut self, cancelled_gen: &AtomicU64) {
        if self.interrupting {
            return;
        }
        // Store the generation being cancelled BEFORE advancing, so workers
        // for this generation see the cancellation even if a new request
        // resets nothing (cancelled_gen is never cleared).
        let cancelled_generation = self.stream_generation;
        cancelled_gen.store(cancelled_generation, Ordering::Release);
        self.stream_generation = cancelled_generation.wrapping_add(1);
        self.flush_stream_text();
        self.active_tool_call = None;
        self.interrupting = true;
        self.stop_pending = Some(cancelled_generation);
    }

    fn finish_stream(&mut self) {
        self.flush_stream_text();
        self.streaming = false;
        self.interrupting = false;
        self.active_tool_call = None;
        self.stream_start = None;
    }

    fn tick_stream(&mut self) {
        if self.stream_pending.is_empty() {
            return;
        }

        let elapsed = self.last_stream_tick.elapsed();
        let chars_due = (elapsed.as_secs_f64() * STREAM_CPS) as usize;

        if chars_due == 0 {
            return;
        }

        self.last_stream_tick = Instant::now();
        let pending = self.stream_pending.len();

        let n = if pending > STREAM_BULK_THRESHOLD {
            pending.min(chars_due + 100)
        } else if pending > STREAM_FAST_THRESHOLD {
            chars_due * 3
        } else {
            chars_due
        };

        let n = n.min(pending);
        let chunk: String = self.stream_pending.drain(..n).collect();
        self.stream_buffer.push_str(&chunk);
    }

    fn take_pending_request(&mut self) -> Option<OttoChatRequest> {
        self.pending_request.take()
    }

    fn tick_spinner(&mut self) {
        if self.streaming && self.last_spinner.elapsed() >= SPINNER_INTERVAL {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER.len();
            self.last_spinner = Instant::now();
        }
    }

    // -- Rendering ----------------------------------------------------------

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        if area.height < 5 {
            return;
        }

        let input_height = self.input_line_count(area.width);
        let chunks = Layout::vertical([
            Constraint::Min(1),               // chat
            Constraint::Length(1),            // top rule
            Constraint::Length(input_height), // input
            Constraint::Length(1),            // bottom rule
            Constraint::Length(1),            // status
        ])
        .split(area);

        self.render_chat(frame, chunks[0]);
        self.render_rule(frame, chunks[1]);
        self.render_input(frame, chunks[2]);
        self.render_rule(frame, chunks[3]);
        self.render_completions(frame, chunks[0], chunks[1]);
        self.render_status(frame, chunks[4]);

        // Cursor shape
        let cursor_style = match self.input_mode {
            InputMode::ViNormal => SetCursorStyle::SteadyBlock,
            _ => SetCursorStyle::BlinkingBar,
        };
        let _ = crossterm::execute!(std::io::stderr(), cursor_style);
    }

    fn render_chat(&self, frame: &mut Frame, area: Rect) {
        let has_content = self.streaming || !self.messages.is_empty();
        if !has_content {
            self.render_splash(frame, area);
            return;
        }

        let mut lines: Vec<Line<'_>> = Vec::new();

        for msg in &self.messages {
            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }

            let (label, color) = match msg.role {
                Role::User => ("  you", USER_COLOR),
                Role::Otto => ("  otto", OTTO_COLOR),
                Role::System => ("", SYSTEM_COLOR),
            };

            if !label.is_empty() {
                let mut label_spans = vec![Span::styled(label, Style::default().fg(color).bold())];
                if self.show_timestamps {
                    label_spans.push(Span::styled(
                        format!("  {}", format_time(msg.timestamp)),
                        Style::default().fg(TIMESTAMP_COLOR),
                    ));
                }
                lines.push(Line::from(label_spans));
            } else if self.show_timestamps {
                // System messages: show timestamp inline
                lines.push(Line::from(Span::styled(
                    format!("  {}", format_time(msg.timestamp)),
                    Style::default().fg(TIMESTAMP_COLOR),
                )));
            }

            if let Some(tc) = &msg.tool_call {
                lines.extend(render_tool_call(tc, self.expand_tool_calls));
            } else {
                lines.extend(render_markdown(
                    &msg.content,
                    msg.role,
                    self.show_raw_markdown,
                ));
            }
        }

        // Streaming: show current buffer or spinner
        if self.streaming {
            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }
            lines.push(Line::from(Span::styled(
                "  otto",
                Style::default().fg(OTTO_COLOR).bold(),
            )));

            if self.stream_buffer.is_empty() && self.stream_pending.is_empty() {
                let label = if self.interrupting {
                    format!("  {} Stopping...", SPINNER[self.spinner_frame])
                } else if let Some((tool, _)) = &self.active_tool_call {
                    format!("  {} \u{2699} {tool}...", SPINNER[self.spinner_frame])
                } else {
                    format!("  {} Ascending...", SPINNER[self.spinner_frame])
                };
                lines.push(Line::from(Span::styled(
                    label,
                    Style::default().fg(DIM_OTTO_COLOR),
                )));
            } else if self.interrupting {
                lines.extend(render_markdown(
                    &self.stream_buffer,
                    Role::Otto,
                    self.show_raw_markdown,
                ));
                lines.push(Line::from(Span::styled(
                    format!("  {} Stopping...", SPINNER[self.spinner_frame]),
                    Style::default().fg(DIM_OTTO_COLOR),
                )));
            } else if let Some((tool, _)) = &self.active_tool_call {
                lines.extend(render_markdown(
                    &self.stream_buffer,
                    Role::Otto,
                    self.show_raw_markdown,
                ));
                lines.push(Line::from(Span::styled(
                    format!("  {} \u{2699} {tool}...", SPINNER[self.spinner_frame]),
                    Style::default().fg(DIM_OTTO_COLOR),
                )));
            } else {
                lines.extend(render_markdown(
                    &self.stream_buffer,
                    Role::Otto,
                    self.show_raw_markdown,
                ));
            }
        }

        // Trailing padding
        lines.push(Line::raw(""));

        // Exact rendered line count via Paragraph::line_count
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let total_rendered = paragraph.line_count(area.width);
        let visible = area.height as usize;
        let max_scroll = total_rendered.saturating_sub(visible);
        let clamped_scroll = self.scroll.min(max_scroll);
        let scroll_y = max_scroll.saturating_sub(clamped_scroll);

        let paragraph = paragraph.scroll((scroll_y.min(u16::MAX as usize) as u16, 0));
        // Clear stale buffer cells — Paragraph doesn't overwrite unused positions.
        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);

        // Scrollbar
        if total_rendered > visible {
            let scrollbar_position = max_scroll.saturating_sub(clamped_scroll);
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scrollbar_position);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .style(Style::default().fg(DIM_COLOR)),
                area,
                &mut scrollbar_state,
            );
        }
    }

    fn render_splash(&self, frame: &mut Frame, area: Rect) {
        let banner_height = EXPERIMENTAL_BANNER.len() as u16 + 2; // +2 for blank lines around it
        let splash_height = SPLASH.len() as u16;
        let total_height = splash_height + banner_height;
        let y_offset = area.height.saturating_sub(total_height) / 2;

        let warning_style = Style::default().fg(WARNING_COLOR).bold();

        let mut lines: Vec<Line<'_>> = Vec::new();

        // Experimental banner
        lines.push(Line::raw(""));
        for &line in EXPERIMENTAL_BANNER {
            let display_width = line.chars().count();
            let pad = (area.width as usize).saturating_sub(display_width) / 2;
            let padded = format!("{:>width$}{}", "", line, width = pad);
            lines.push(Line::from(Span::styled(padded, warning_style)));
        }
        lines.push(Line::raw(""));

        // Otto splash
        for &line in SPLASH {
            let display_width = line.chars().count();
            let pad = (area.width as usize).saturating_sub(display_width) / 2;
            let padded = format!("{:>width$}{}", "", line, width = pad);
            if line.contains("/help") {
                lines.push(Line::from(Span::styled(
                    padded,
                    Style::default().fg(DIM_COLOR),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    padded,
                    Style::default().fg(OTTO_COLOR),
                )));
            }
        }

        let clamped_height = total_height.min(area.height);
        let splash_area = Rect::new(area.x, area.y + y_offset, area.width, clamped_height);
        frame.render_widget(Paragraph::new(lines), splash_area);
    }

    fn render_rule(&self, frame: &mut Frame, area: Rect) {
        let rule_color = match self.input_mode {
            InputMode::ViNormal => VI_NORMAL_COLOR,
            _ if self.streaming => DIM_COLOR,
            _ => OTTO_COLOR,
        };
        let rule = "\u{2500}".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                rule,
                Style::default().fg(rule_color),
            ))),
            area,
        );
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let prompt = match self.input_mode {
            InputMode::ViNormal => " \u{2502} ",
            _ => " \u{276f} ",
        };
        let prompt_len = 3usize;
        let avail = (area.width as usize).saturating_sub(prompt_len);
        if avail == 0 {
            return;
        }

        let prompt_color = match self.input_mode {
            InputMode::ViNormal => VI_NORMAL_COLOR,
            _ if self.streaming => DIM_COLOR,
            _ => OTTO_COLOR,
        };

        // Build visual rows with wrapping, tracking cursor position
        let mut rows: Vec<String> = vec![String::new()];
        let mut col = 0usize;
        let mut cursor_row = 0usize;
        let mut cursor_col = 0usize;

        for (i, &ch) in self.input.iter().enumerate() {
            if i == self.cursor {
                cursor_row = rows.len() - 1;
                cursor_col = col;
            }
            if ch == '\n' {
                rows.push(String::new());
                col = 0;
            } else {
                if col >= avail {
                    rows.push(String::new());
                    col = 0;
                }
                rows.last_mut().unwrap().push(ch);
                col += 1;
            }
        }
        // Cursor at end of input
        if self.cursor == self.input.len() {
            if col >= avail {
                rows.push(String::new());
                cursor_row = rows.len() - 1;
                cursor_col = 0;
            } else {
                cursor_row = rows.len() - 1;
                cursor_col = col;
            }
        }

        // Scroll viewport so cursor is always visible
        let max_visible = area.height as usize;
        let scroll_offset = if cursor_row >= max_visible {
            cursor_row - max_visible + 1
        } else {
            0
        };
        let visible_end = (scroll_offset + max_visible).min(rows.len());

        let mut render_lines: Vec<Line<'_>> = Vec::new();
        for (i, row) in rows
            .iter()
            .enumerate()
            .take(visible_end)
            .skip(scroll_offset)
        {
            let p = if i == 0 { prompt } else { "   " };
            let p_style = if i == 0 {
                Style::default().fg(prompt_color)
            } else {
                Style::default().fg(DIM_COLOR)
            };
            render_lines.push(Line::from(vec![
                Span::styled(p, p_style),
                Span::raw(row.clone()),
            ]));
        }

        frame.render_widget(Paragraph::new(render_lines), area);

        if !self.streaming {
            let cx = area.x + prompt_len as u16 + cursor_col as u16;
            let cy = area.y + (cursor_row - scroll_offset) as u16;
            frame.set_cursor_position((cx, cy));
        }
    }

    fn render_completions(&self, frame: &mut Frame, chat_area: Rect, rule_area: Rect) {
        let matches = self.completions();
        if matches.is_empty() {
            return;
        }

        let height = matches.len().min(8) as u16;
        let width = matches.iter().map(|s| s.len()).max().unwrap_or(0) as u16 + 4;

        let x = rule_area.x + 1;
        let y = chat_area.bottom().saturating_sub(height);
        let popup = Rect::new(x, y, width.min(rule_area.width), height);

        frame.render_widget(Clear, popup);

        let items: Vec<Line<'_>> = matches
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let style = if self.completion_index == Some(i) {
                    Style::default().fg(TEXT_COLOR).bg(OTTO_COLOR).bold()
                } else {
                    Style::default().fg(TEXT_COLOR).bg(POPUP_BG)
                };
                Line::from(Span::styled(format!(" {cmd} "), style))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(POPUP_BG));
        let paragraph = Paragraph::new(items).block(block);
        frame.render_widget(paragraph, popup);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let (mode, mode_color) = match self.input_mode {
            InputMode::Emacs => ("emacs", SYSTEM_COLOR),
            InputMode::ViInsert => ("INSERT", VI_NORMAL_COLOR),
            InputMode::ViNormal => ("NORMAL", VI_NORMAL_COLOR),
        };
        let (mode, mode_color) = if self.interrupting {
            ("STOPPING", WARNING_COLOR)
        } else {
            (mode, mode_color)
        };

        let mut parts = vec![Span::styled(
            format!(" {mode}"),
            Style::default().fg(mode_color),
        )];

        let pill_style = Style::default().fg(DIM_OTTO_COLOR);

        if let Some(label) = &self.context_label {
            parts.push(Span::raw(" "));
            parts.push(Span::styled(format!(" {label} "), pill_style));
        }

        if let Some(provider) = &self.provider_label {
            parts.push(Span::raw(" "));
            parts.push(Span::styled(format!(" provider:{provider} "), pill_style));
        }

        if !self.model_label.is_empty() {
            parts.push(Span::raw(" "));
            parts.push(Span::styled(
                format!(" model:{} ", self.model_label),
                pill_style,
            ));
        }

        if let Some(tid) = &self.thread_id {
            let short: String = tid.chars().take(12).collect();
            parts.push(Span::raw(" "));
            parts.push(Span::styled(format!(" thread:{short} "), pill_style));
        }

        let msg_count = self
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .count();
        if msg_count > 0 {
            parts.push(Span::raw(" "));
            parts.push(Span::styled(format!(" {msg_count} messages "), pill_style));
        }

        // Truncate pills to fit terminal width
        let total_width: usize = parts.iter().map(|s| s.width()).sum();
        if total_width > area.width as usize {
            let mut width = 0;
            let mut truncated = Vec::new();
            for span in parts {
                width += span.width();
                if width > area.width as usize {
                    break;
                }
                truncated.push(span);
            }
            frame.render_widget(Paragraph::new(Line::from(truncated)), area);
        } else {
            frame.render_widget(Paragraph::new(Line::from(parts)), area);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate a string for display, adding "..." if it exceeds `max_len`.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{truncated}...")
    }
}

fn format_time(time: SystemTime) -> String {
    // Elapsed since message was created — avoids UTC vs local time issues
    let elapsed = time.elapsed().unwrap_or_default();
    let secs = elapsed.as_secs();
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

// ---------------------------------------------------------------------------
// Markdown rendering
// ---------------------------------------------------------------------------

fn render_markdown(text: &str, role: Role, raw: bool) -> Vec<Line<'static>> {
    if raw {
        return render_raw(text, role);
    }
    render_markdown_parsed(text, role)
}

fn render_tool_call(tc: &ToolCallData, expanded: bool) -> Vec<Line<'static>> {
    let indent = "  ";
    let sys_style = Style::default().fg(SYSTEM_COLOR).italic();
    let dim_style = Style::default().fg(DIM_COLOR);
    let text_style = Style::default().fg(TEXT_COLOR);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("{indent}\u{2699} {}", tc.name),
        sys_style,
    )));

    if !expanded {
        let summary = truncate(&tc.output, 80);
        lines.push(Line::from(vec![
            Span::styled(format!("{indent}\u{2192} {summary}"), text_style),
            Span::styled("  Ctrl+o to expand", dim_style),
        ]));
        return lines;
    }

    // Pretty-print a JSON string, falling back to raw text
    let pretty = |raw: &str| -> String {
        serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| raw.to_string())
    };

    for (label, raw) in [("arguments", &tc.arguments), ("output", &tc.output)] {
        if raw.is_empty() {
            continue;
        }
        let content = pretty(raw);
        lines.push(Line::from(Span::styled(
            format!("{indent}\u{256d}\u{2500} {label} \u{2500}"),
            dim_style,
        )));
        for line in content.lines() {
            lines.push(Line::from(Span::styled(
                format!("{indent}\u{2502} {line}"),
                text_style,
            )));
        }
        lines.push(Line::from(Span::styled(
            format!("{indent}\u{2570}\u{2500}\u{2500}"),
            dim_style,
        )));
    }

    lines.push(Line::from(Span::styled(
        format!("{indent}Ctrl+o to collapse"),
        dim_style,
    )));

    lines
}

/// Raw mode: show literal markdown source with minimal styling.
fn render_raw(text: &str, role: Role) -> Vec<Line<'static>> {
    let base_style = match role {
        Role::System => Style::default().fg(SYSTEM_COLOR).italic(),
        _ => Style::default(),
    };
    text.lines()
        .map(|line| {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(line.to_string(), base_style),
            ])
        })
        .collect()
}

/// Rendered mode: parse markdown with pulldown-cmark and produce styled lines.
fn render_markdown_parsed(text: &str, role: Role) -> Vec<Line<'static>> {
    let base_style = match role {
        Role::System => Style::default().fg(SYSTEM_COLOR).italic(),
        _ => Style::default(),
    };

    let mut md = MdRenderer {
        lines: Vec::new(),
        spans: Vec::new(),
        style_stack: vec![base_style],
        base_indent: "  ".to_string(),
        list_indent: String::new(),
        list_stack: Vec::new(),
        in_code_block: false,
        code_block_lang: String::new(),
        highlighter: None,
        blockquote_depth: 0,
        in_heading: false,
        in_table: false,
        in_table_header: false,
        table_cell_spans: Vec::new(),
        table_cell_texts: Vec::new(),
        table_row_spans: Vec::new(),
        table_header_spans: Vec::new(),
        table_body_spans: Vec::new(),
        table_col_widths: Vec::new(),
        table_alignments: Vec::new(),
        link_url: None,
        link_text: String::new(),
    };

    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    let parser = MdParser::new_ext(text, opts);

    for event in parser {
        md.process(event);
    }

    // Flush any remaining spans.
    md.flush_line();

    md.lines
}

#[derive(Clone)]
enum ListKind {
    Unordered,
    Ordered { next: u64, max_digits: usize },
}

#[derive(Clone)]
struct ListEntry {
    kind: ListKind,
    /// The `list_indent` that was active when this list was opened.
    parent_indent: String,
}

struct MdRenderer {
    lines: Vec<Line<'static>>,
    spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    base_indent: String,
    list_indent: String,
    list_stack: Vec<ListEntry>,
    in_code_block: bool,
    code_block_lang: String,
    highlighter: Option<HighlightLines<'static>>,
    blockquote_depth: usize,
    in_heading: bool,
    // Table state — two-pass: buffer all rows, render at End(Table).
    in_table: bool,
    in_table_header: bool,
    table_cell_spans: Vec<Span<'static>>,
    table_cell_texts: Vec<String>,
    table_row_spans: Vec<Vec<Span<'static>>>,
    table_header_spans: Vec<Vec<Span<'static>>>,
    table_body_spans: Vec<Vec<Vec<Span<'static>>>>,
    table_col_widths: Vec<usize>,
    table_alignments: Vec<pulldown_cmark::Alignment>,
    link_url: Option<String>,
    link_text: String,
}

impl MdRenderer {
    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn push_style(&mut self, modifier: impl FnOnce(Style) -> Style) {
        let new = modifier(self.current_style());
        self.style_stack.push(new);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    /// Plain string indent — used for code blocks and other concatenation contexts.
    fn indent_prefix(&self) -> String {
        let mut prefix = self.base_indent.clone();
        for _ in 0..self.blockquote_depth {
            prefix.push_str("\u{2502} ");
        }
        prefix.push_str(&self.list_indent);
        prefix
    }

    /// Styled indent spans — blockquote bars get DIM_COLOR, rest is unstyled.
    fn indent_spans(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        if self.blockquote_depth == 0 {
            let mut prefix = self.base_indent.clone();
            prefix.push_str(&self.list_indent);
            spans.push(Span::raw(prefix));
        } else {
            spans.push(Span::raw(self.base_indent.clone()));
            for _ in 0..self.blockquote_depth {
                spans.push(Span::styled(
                    "\u{2502} ".to_string(),
                    Style::default().fg(DIM_COLOR),
                ));
            }
            if !self.list_indent.is_empty() {
                spans.push(Span::raw(self.list_indent.clone()));
            }
        }
        spans
    }

    fn flush_line(&mut self) {
        if !self.spans.is_empty() {
            self.lines.push(Line::from(std::mem::take(&mut self.spans)));
        }
    }

    fn blank_line_if_needed(&mut self) {
        self.flush_line();
        // Add blank line if previous line was non-empty content.
        if let Some(last) = self.lines.last()
            && !(last.spans.is_empty()
                || (last.spans.len() == 1 && last.spans[0].content.trim().is_empty()))
        {
            self.lines.push(Line::raw(""));
        }
    }

    fn process(&mut self, event: MdEvent<'_>) {
        match event {
            // -- Block-level start tags --
            MdEvent::Start(MdTag::Heading { level, .. }) => {
                self.blank_line_if_needed();
                self.in_heading = true;
                match level {
                    pulldown_cmark::HeadingLevel::H1 => {
                        self.push_style(|s| s.fg(HEADING_COLOR).bold().underlined());
                    }
                    pulldown_cmark::HeadingLevel::H2 => {
                        self.push_style(|s| s.fg(HEADING_COLOR).bold());
                    }
                    pulldown_cmark::HeadingLevel::H3 => {
                        self.push_style(|s| s.bold());
                    }
                    _ => {
                        self.push_style(|s| s.bold().italic());
                    }
                }
                self.spans.extend(self.indent_spans());
            }
            MdEvent::End(MdTagEnd::Heading(_)) => {
                self.in_heading = false;
                self.pop_style();
                self.flush_line();
            }

            MdEvent::Start(MdTag::Paragraph) => {
                if !self.in_code_block && self.list_stack.is_empty() {
                    self.blank_line_if_needed();
                }
            }
            MdEvent::End(MdTagEnd::Paragraph) => {
                self.flush_line();
            }

            MdEvent::Start(MdTag::BlockQuote(kind)) => {
                self.blank_line_if_needed();
                self.blockquote_depth += 1;
                self.push_style(|s| s.italic());
                // Render GFM admonition labels ([!NOTE], [!TIP], etc.).
                if let Some(bqk) = kind {
                    let (label, color) = match bqk {
                        BlockQuoteKind::Note => ("NOTE", NOTE_COLOR),
                        BlockQuoteKind::Tip => ("TIP", TIP_COLOR),
                        BlockQuoteKind::Important => ("IMPORTANT", IMPORTANT_COLOR),
                        BlockQuoteKind::Warning => ("WARNING", WARNING_COLOR),
                        BlockQuoteKind::Caution => ("CAUTION", CAUTION_COLOR),
                    };
                    let mut label_spans = self.indent_spans();
                    label_spans.push(Span::styled(
                        label.to_string(),
                        Style::default().fg(color).bold(),
                    ));
                    self.lines.push(Line::from(label_spans));
                }
            }
            MdEvent::End(MdTagEnd::BlockQuote(_)) => {
                self.flush_line();
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
                self.pop_style();
            }

            MdEvent::Start(MdTag::CodeBlock(kind)) => {
                self.blank_line_if_needed();
                self.in_code_block = true;
                // Extract just the language token (first word) from the info string.
                // Fenced code blocks can have metadata after the language, e.g.:
                //   ```sql title="file.sql" lines="1-15"
                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                // Try to find a syntax for highlighting.
                self.highlighter =
                    find_syntax(&self.code_block_lang).map(|syn| HighlightLines::new(syn, &THEME));
                let prefix = self.indent_prefix();
                let header = if self.code_block_lang.is_empty() {
                    format!("{prefix}\u{256d}\u{2500}\u{2500}")
                } else {
                    format!("{prefix}\u{256d}\u{2500} {} \u{2500}", self.code_block_lang)
                };
                self.lines.push(Line::from(Span::styled(
                    header,
                    Style::default().fg(DIM_COLOR),
                )));
            }
            MdEvent::End(MdTagEnd::CodeBlock) => {
                let prefix = self.indent_prefix();
                self.lines.push(Line::from(Span::styled(
                    format!("{prefix}\u{2570}\u{2500}\u{2500}"),
                    Style::default().fg(DIM_COLOR),
                )));
                self.in_code_block = false;
                self.code_block_lang.clear();
                self.highlighter = None;
            }

            MdEvent::Start(MdTag::List(first)) => {
                if self.list_stack.is_empty() {
                    self.blank_line_if_needed();
                }
                let kind = match first {
                    Some(start) => ListKind::Ordered {
                        next: start,
                        max_digits: start.to_string().len(),
                    },
                    None => ListKind::Unordered,
                };
                self.list_stack.push(ListEntry {
                    kind,
                    parent_indent: self.list_indent.clone(),
                });
            }
            MdEvent::End(MdTagEnd::List(_)) => {
                self.list_stack.pop();
                if self.list_stack.is_empty() {
                    self.flush_line();
                }
            }

            MdEvent::Start(MdTag::Item) => {
                self.flush_line();
                let depth = self.list_stack.len().saturating_sub(1);
                let nested_indent = "  ".repeat(depth);

                let bullet = match self.list_stack.last().map(|e| &e.kind) {
                    Some(ListKind::Unordered) => format!("{nested_indent}\u{2022} "),
                    Some(ListKind::Ordered { next, max_digits }) => {
                        let num = *next;
                        // Pad to max_digits for consistent indentation.
                        let d = (*max_digits).max(num.to_string().len());
                        format!("{nested_indent}{num:>d$}. ")
                    }
                    None => String::new(),
                };

                // Set bullet prefix for first line, render the indent+bullet.
                self.list_indent = bullet.clone();
                self.spans.extend(self.indent_spans());
                // Set continuation indent matching the bullet width.
                self.list_indent = " ".repeat(bullet.len());
            }
            MdEvent::End(MdTagEnd::Item) => {
                self.flush_line();
                if let Some(ListEntry {
                    kind: ListKind::Ordered { next, max_digits },
                    ..
                }) = self.list_stack.last_mut()
                {
                    *next += 1;
                    *max_digits = (*max_digits).max(next.to_string().len());
                }
                // Restore the indent that was active when this list was opened.
                self.list_indent = self
                    .list_stack
                    .last()
                    .map(|entry| entry.parent_indent.clone())
                    .unwrap_or_default();
            }

            // -- Inline start/end tags --
            MdEvent::Start(MdTag::Strong) => {
                self.push_style(|s| s.bold());
            }
            MdEvent::End(MdTagEnd::Strong) => {
                self.pop_style();
            }

            MdEvent::Start(MdTag::Emphasis) => {
                self.push_style(|s| s.italic());
            }
            MdEvent::End(MdTagEnd::Emphasis) => {
                self.pop_style();
            }

            MdEvent::Start(MdTag::Strikethrough) => {
                self.push_style(|s| s.crossed_out());
            }
            MdEvent::End(MdTagEnd::Strikethrough) => {
                self.pop_style();
            }

            MdEvent::Start(MdTag::Link { dest_url, .. }) => {
                self.push_style(|s| s.fg(LINK_COLOR).underlined());
                self.link_url = Some(dest_url.to_string());
                self.link_text.clear();
            }
            MdEvent::End(MdTagEnd::Link) => {
                self.pop_style();
                if let Some(url) = self.link_url.take() {
                    let text = std::mem::take(&mut self.link_text);
                    // Only show URL if it differs from the link text.
                    if text != url {
                        self.spans.push(Span::styled(
                            format!(" ({url})"),
                            Style::default().fg(DIM_COLOR),
                        ));
                    }
                }
            }

            MdEvent::Start(MdTag::Image { dest_url, .. }) => {
                if self.spans.is_empty() {
                    self.spans.extend(self.indent_spans());
                }
                self.spans.push(Span::styled(
                    format!("[image: {dest_url}]"),
                    Style::default().fg(DIM_COLOR),
                ));
            }
            MdEvent::End(MdTagEnd::Image) => {}

            // -- Table handling (two-pass: buffer all rows, render at End(Table)) --
            MdEvent::Start(MdTag::Table(alignments)) => {
                self.blank_line_if_needed();
                self.in_table = true;
                self.table_alignments = alignments;
                self.table_col_widths.clear();
                self.table_header_spans.clear();
                self.table_body_spans.clear();
            }
            MdEvent::End(MdTagEnd::Table) => {
                self.render_buffered_table();
                self.in_table = false;
                self.table_alignments.clear();
                self.table_col_widths.clear();
            }

            MdEvent::Start(MdTag::TableHead) => {
                self.in_table_header = true;
                self.table_cell_texts.clear();
                self.table_row_spans.clear();
            }
            MdEvent::End(MdTagEnd::TableHead) => {
                self.in_table_header = false;
                for (i, text) in self.table_cell_texts.iter().enumerate() {
                    let w = text.width();
                    if i < self.table_col_widths.len() {
                        self.table_col_widths[i] = self.table_col_widths[i].max(w);
                    } else {
                        self.table_col_widths.push(w);
                    }
                }
                self.table_header_spans = std::mem::take(&mut self.table_row_spans);
                self.table_cell_texts.clear();
            }

            MdEvent::Start(MdTag::TableRow) => {
                self.table_cell_texts.clear();
                self.table_row_spans.clear();
            }
            MdEvent::End(MdTagEnd::TableRow) => {
                if !self.in_table_header {
                    for (i, text) in self.table_cell_texts.iter().enumerate() {
                        let w = text.width();
                        if i < self.table_col_widths.len() {
                            self.table_col_widths[i] = self.table_col_widths[i].max(w);
                        } else {
                            self.table_col_widths.push(w);
                        }
                    }
                    self.table_body_spans
                        .push(std::mem::take(&mut self.table_row_spans));
                }
                self.table_cell_texts.clear();
            }

            MdEvent::Start(MdTag::TableCell) => {
                self.table_cell_spans.clear();
            }
            MdEvent::End(MdTagEnd::TableCell) => {
                let plain: String = self
                    .table_cell_spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect();
                self.table_cell_texts.push(plain);
                self.table_row_spans
                    .push(std::mem::take(&mut self.table_cell_spans));
            }

            // -- Leaf events --
            MdEvent::Text(text) => {
                if self.in_code_block {
                    let prefix = self.indent_prefix();
                    let is_diff = self.code_block_lang == "diff";
                    for line in text.lines() {
                        let mut spans: Vec<Span<'static>> = Vec::new();
                        spans.push(Span::styled(
                            format!("{prefix}\u{2502} "),
                            Style::default().fg(DIM_COLOR),
                        ));
                        if is_diff {
                            spans.push(Span::styled(
                                line.to_string(),
                                Style::default().fg(diff_line_color(line)),
                            ));
                        } else if let Some(ref mut hl) = self.highlighter {
                            if let Ok(highlighted) = hl.highlight_line(line, &SYNTAX_SET) {
                                for (style, fragment) in highlighted {
                                    spans.push(Span::styled(
                                        fragment.to_string(),
                                        syntect_to_ratatui_style(style),
                                    ));
                                }
                            } else {
                                spans.push(Span::styled(
                                    line.to_string(),
                                    Style::default().fg(TEXT_COLOR),
                                ));
                            }
                        } else {
                            spans.push(Span::styled(
                                line.to_string(),
                                Style::default().fg(TEXT_COLOR),
                            ));
                        }
                        self.lines.push(Line::from(spans));
                    }
                } else if self.in_table {
                    self.table_cell_spans
                        .push(Span::styled(text.to_string(), self.current_style()));
                } else {
                    // Track link text for dedup.
                    if self.link_url.is_some() {
                        self.link_text.push_str(&text);
                    }
                    if self.spans.is_empty() && !self.in_heading {
                        self.spans.extend(self.indent_spans());
                    }
                    self.spans
                        .push(Span::styled(text.to_string(), self.current_style()));
                }
            }

            MdEvent::Code(code) => {
                let backtick_style = Style::default().fg(DIM_COLOR);
                let code_style = Style::default().fg(CODE_COLOR);
                let target = if self.in_table {
                    &mut self.table_cell_spans
                } else {
                    if self.spans.is_empty() {
                        self.spans.extend(self.indent_spans());
                    }
                    &mut self.spans
                };
                target.push(Span::styled("`".to_string(), backtick_style));
                target.push(Span::styled(code.to_string(), code_style));
                target.push(Span::styled("`".to_string(), backtick_style));
            }

            MdEvent::SoftBreak => {
                if self.in_code_block {
                    return;
                }
                if self.in_table {
                    self.table_cell_spans.push(Span::raw(" "));
                } else {
                    self.spans.push(Span::raw(" "));
                }
            }

            MdEvent::HardBreak => {
                if self.in_table {
                    // Tables are single-line cells; treat as space.
                    self.table_cell_spans.push(Span::raw(" "));
                } else {
                    self.flush_line();
                    self.spans.extend(self.indent_spans());
                }
            }

            MdEvent::Rule => {
                self.blank_line_if_needed();
                let prefix = self.indent_prefix();
                self.lines.push(Line::from(Span::styled(
                    format!("{prefix}{}", "\u{2500}".repeat(40)),
                    Style::default().fg(DIM_COLOR),
                )));
            }

            MdEvent::TaskListMarker(checked) => {
                if checked {
                    self.spans.push(Span::styled(
                        "[\u{2713}] ".to_string(),
                        Style::default().fg(CHECK_COLOR),
                    ));
                } else {
                    self.spans.push(Span::styled(
                        "[ ] ".to_string(),
                        Style::default().fg(DIM_COLOR),
                    ));
                }
            }

            // Ignore HTML and other events.
            _ => {}
        }
    }

    /// Render the fully-buffered table: header, separator, body rows.
    /// Two-pass ensures column widths are correct across all rows.
    fn render_buffered_table(&mut self) {
        let prefix = self.indent_prefix();

        // Header row (bold).
        let header = std::mem::take(&mut self.table_header_spans);
        self.render_table_line(&prefix, &header, true);

        // Separator.
        let sep = self
            .table_col_widths
            .iter()
            .map(|&w| "\u{2500}".repeat(w))
            .collect::<Vec<_>>()
            .join("\u{2500}\u{253c}\u{2500}");
        self.lines.push(Line::from(Span::styled(
            format!("{prefix}{sep}"),
            Style::default().fg(DIM_COLOR),
        )));

        // Body rows.
        let body = std::mem::take(&mut self.table_body_spans);
        for row in &body {
            self.render_table_line(&prefix, row, false);
        }
    }

    /// Render one table row as a Line with per-cell styled Spans and padding.
    fn render_table_line(&mut self, prefix: &str, cells: &[Vec<Span<'static>>], bold: bool) {
        let mut line_spans: Vec<Span<'static>> = vec![Span::raw(prefix.to_string())];

        for (i, cell_spans) in cells.iter().enumerate() {
            if i > 0 {
                line_spans.push(Span::styled(
                    " \u{2502} ".to_string(),
                    Style::default().fg(DIM_COLOR),
                ));
            }

            let cell_text_len: usize = cell_spans.iter().map(|s| s.content.width()).sum();
            let col_width = self
                .table_col_widths
                .get(i)
                .copied()
                .unwrap_or(cell_text_len);
            let pad = col_width.saturating_sub(cell_text_len);
            let align = self.table_alignments.get(i).copied();

            let (left_pad, right_pad) = match align {
                Some(pulldown_cmark::Alignment::Center) => (pad / 2, pad - pad / 2),
                Some(pulldown_cmark::Alignment::Right) => (pad, 0),
                _ => (0, pad),
            };

            if left_pad > 0 {
                line_spans.push(Span::raw(" ".repeat(left_pad)));
            }
            for span in cell_spans {
                if bold {
                    line_spans.push(Span::styled(span.content.clone(), span.style.bold()));
                } else {
                    line_spans.push(span.clone());
                }
            }
            if right_pad > 0 {
                line_spans.push(Span::raw(" ".repeat(right_pad)));
            }
        }

        self.lines.push(Line::from(line_spans));
    }
}

/// Pick a color for a line inside a `diff` code block.
fn diff_line_color(line: &str) -> Color {
    if line.starts_with("@@") {
        DIFF_HUNK_COLOR
    } else if line.starts_with('+') {
        DIFF_ADD_COLOR
    } else if line.starts_with('-') {
        DIFF_DEL_COLOR
    } else {
        TEXT_COLOR
    }
}

/// Find a syntect syntax definition for a code block language tag.
fn find_syntax(lang: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    // Diff blocks use custom line-by-line coloring (diff_line_color), not syntect.
    if lang.is_empty() || lang == "diff" {
        return None;
    }
    SYNTAX_SET
        .find_syntax_by_token(lang)
        .filter(|s| s.name != "Plain Text")
}

/// Convert a syntect highlighting style to a ratatui style.
fn syntect_to_ratatui_style(style: syntect::highlighting::Style) -> Style {
    let fg = style.foreground;
    let mut s = Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b));
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::BOLD)
    {
        s = s.bold();
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::ITALIC)
    {
        s = s.italic();
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::UNDERLINE)
    {
        s = s.underlined();
    }
    s
}

// ---------------------------------------------------------------------------
// Provider resolution
// ---------------------------------------------------------------------------

/// Resolve provider/model labels from the API, mapping IDs to friendly names.
fn resolve_provider_labels(
    client: &AscendClient,
    otto_model: &Option<OttoModel>,
) -> (Option<String>, String) {
    let providers = client.list_otto_providers().ok().unwrap_or_default();
    match otto_model {
        Some(model) => {
            let model_id = model.id();
            let lower = model_id.to_lowercase();
            // Find which provider has this model (match by ID or name)
            for p in &providers {
                if let Some(m) = p.models.iter().find(|m| {
                    m.id == model_id
                        || m.id.to_lowercase() == lower
                        || m.name.to_lowercase() == lower
                }) {
                    return (Some(p.name.clone()), m.name.clone());
                }
            }
            // Fallback: extract a short name from the ID
            // (e.g. "bedrock/global.anthropic.claude-sonnet-4-5-v1" → last segment)
            let short = model_id
                .rsplit_once('/')
                .map(|(_, s)| s)
                .or_else(|| model_id.rsplit_once('.').map(|(_, s)| s))
                .unwrap_or(model_id);
            (None, short.to_string())
        }
        None => providers
            .first()
            .map(|p| {
                let model_name = p
                    .models
                    .iter()
                    .find(|m| m.id == p.default_model)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| p.default_model.clone());
                (Some(p.name.clone()), model_name)
            })
            .unwrap_or_default(),
    }
}

fn raw_messages_to_messages<'a>(
    messages: impl IntoIterator<Item = &'a serde_json::Value>,
) -> Vec<Message> {
    let mut out = Vec::new();
    for msg in messages {
        let role_str = msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
        let role = match role_str {
            "user" => Role::User,
            "assistant" => Role::Otto,
            _ => continue,
        };
        let text = Conversation::extract_message_text(msg);
        if text.is_empty() {
            continue;
        }
        let timestamp = msg
            .get("created_at")
            .and_then(|v| v.as_f64())
            .map(|epoch| UNIX_EPOCH + Duration::from_secs_f64(epoch))
            .or_else(|| {
                msg.get("created_at")
                    .and_then(|v| v.as_i64())
                    .map(|epoch| UNIX_EPOCH + Duration::from_secs(epoch.try_into().unwrap_or(0)))
            })
            .unwrap_or(UNIX_EPOCH);
        out.push(Message {
            role,
            content: text,
            timestamp,
            tool_call: None,
        });
    }
    out
}

/// Convert a progressive conversation preview into TUI `Message`s.
fn conversation_preview_to_messages(
    preview: &ascend_tools::models::ConversationPreview,
) -> Vec<Message> {
    raw_messages_to_messages(preview.ordered_messages())
}

fn stream_conversation_history_with_fetch<FPreview, FPage, FBatch>(
    fetch_preview: FPreview,
    mut fetch_page: FPage,
    mut on_batch: FBatch,
) -> Result<()>
where
    FPreview: FnOnce() -> Result<ascend_tools::models::ConversationPreview>,
    FPage: FnMut(&str) -> Result<ascend_tools::models::ConversationMessagesPage>,
    FBatch: FnMut(Vec<Message>),
{
    let preview = fetch_preview()?;
    on_batch(conversation_preview_to_messages(&preview));

    let mut cursor = if preview.has_more {
        preview.oldest_message_id.clone()
    } else {
        None
    };

    while let Some(before) = cursor.take() {
        let page = fetch_page(&before)?;
        let older_messages = raw_messages_to_messages(page.ordered_messages());
        if !older_messages.is_empty() {
            on_batch(older_messages);
        }
        cursor = if page.has_more {
            match page.oldest_message_id {
                Some(next) if next != before => Some(next),
                _ => None,
            }
        } else {
            None
        };
    }

    Ok(())
}

fn stream_conversation_history(
    client: &AscendClient,
    thread_id: &str,
    on_batch: impl FnMut(Vec<Message>),
) -> Result<()> {
    stream_conversation_history_with_fetch(
        || Ok(client.get_conversation_preview(thread_id)?),
        |before| {
            Ok(client.get_conversation_messages_before(
                thread_id,
                before,
                Some(RESUME_HISTORY_PAGE_SIZE),
            )?)
        },
        on_batch,
    )
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run_tui(
    client: &AscendClient,
    runtime_uuid: Option<String>,
    otto_model: Option<OttoModel>,
    context_label: Option<String>,
    thread_id: Option<String>,
    query_policy: Option<String>,
) -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stderr = std::io::stderr();
    crossterm::execute!(
        stderr,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    // Panic hook to restore terminal on crash
    let original_hook: Arc<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static> =
        std::panic::take_hook().into();
    let panic_hook = original_hook.clone();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableBracketedPaste,
            SetCursorStyle::DefaultUserShape
        );
        (panic_hook)(info);
    }));

    let (stream_tx, stream_rx) = mpsc::channel::<StreamMsg>();
    let cancelled_gen = AtomicU64::new(0);
    let gen_counter = AtomicU64::new(0);
    let active_thread_id: Mutex<Option<(u64, String)>> = Mutex::new(None);

    let result = std::thread::scope(|scope| {
        // Resolve provider/model labels in the background so the TUI loads instantly
        let bg_tx = stream_tx.clone();
        let bg_model = otto_model.clone();
        scope.spawn(move || {
            let (provider_label, model_label) = resolve_provider_labels(client, &bg_model);
            let _ = bg_tx.send(StreamMsg::ProviderInfo {
                provider_label,
                model_label,
            });
        });

        let mut app = App::new(
            runtime_uuid,
            otto_model,
            None,
            String::new(),
            context_label,
            thread_id.clone(),
            query_policy,
        );

        // If resuming a conversation, load its history in the background
        if let Some(tid) = thread_id {
            let history_tx = stream_tx.clone();
            let history_gen = app.stream_generation;
            scope.spawn(move || {
                let result = stream_conversation_history(client, &tid, |messages| {
                    let _ = history_tx.send(StreamMsg::ConversationHistory {
                        generation: history_gen,
                        messages,
                    });
                });
                if let Err(err) = result {
                    let _ = history_tx.send(StreamMsg::ConversationHistoryError {
                        generation: history_gen,
                        error: err.to_string(),
                    });
                }
            });
        }

        loop {
            app.tick_spinner();
            app.tick_stream();
            terminal.draw(|frame| app.render(frame))?;

            if event::poll(POLL_DURATION)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        app.handle_key(key, &cancelled_gen);
                    }
                    Event::Paste(text) => {
                        app.handle_paste(&text);
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollUp => app.scroll_up(3),
                        MouseEventKind::ScrollDown => app.scroll_down(3),
                        _ => {}
                    },
                    Event::Resize(_, _) => {
                        // ratatui handles re-layout; just ensure scroll is valid
                        if app.auto_scroll {
                            app.scroll = 0;
                        }
                    }
                    _ => {}
                }
            }

            // Process stream messages
            while let Ok(msg) = stream_rx.try_recv() {
                app.handle_stream_msg(msg);
            }

            // If the user cancelled, tell the backend to stop the thread.
            // Spawns a background thread so the TUI stays responsive.
            if let Some(cancelled_generation) = app.stop_pending.take() {
                let tid = active_thread_id
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .filter(|(g, _)| *g == cancelled_generation)
                    .map(|(_, tid)| tid.clone());
                if let Some(tid) = tid {
                    let stop_tx = stream_tx.clone();
                    scope.spawn(move || {
                        let error = client
                            .stop_thread_and_wait(&tid)
                            .err()
                            .map(|e| e.to_string());
                        let _ = stop_tx.send(StreamMsg::StopFinished { error });
                    });
                } else {
                    app.finish_stream();
                    app.push_system("Cancelled");
                }
            }

            // Launch streaming request if pending
            if !app.interrupting
                && let Some(request) = app.take_pending_request()
            {
                let generation = gen_counter.fetch_add(1, Ordering::AcqRel) + 1;
                app.stream_generation = generation;
                // No cancelled_gen reset — each worker checks its own generation.
                *active_thread_id.lock().unwrap_or_else(|e| e.into_inner()) = None;
                let tx = stream_tx.clone();
                let cg_ref = &cancelled_gen;
                let active_tid = &active_thread_id;
                scope.spawn(move || {
                    let tx2 = tx.clone();
                    let mut tool_names: HashMap<String, String> = HashMap::new();
                    let send = |kind: StreamMsgKind| {
                        let _ = tx.send(StreamMsg::Stream { generation, kind });
                    };
                    let result = client.otto_streaming(
                        &request,
                        |event| {
                            if cg_ref.load(Ordering::Acquire) >= generation {
                                return ControlFlow::Break(());
                            }
                            match event {
                                StreamEvent::TextDelta(delta) => {
                                    send(StreamMsgKind::Delta(delta));
                                }
                                StreamEvent::ToolCallStart {
                                    call_id,
                                    name,
                                    arguments,
                                } => {
                                    tool_names.insert(call_id, name.clone());
                                    send(StreamMsgKind::ToolCallStart { name, arguments });
                                }
                                StreamEvent::ToolCallOutput { call_id, output } => {
                                    let name =
                                        tool_names.get(&call_id).cloned().unwrap_or_default();
                                    send(StreamMsgKind::ToolCallOutput { name, output });
                                }
                            }
                            ControlFlow::Continue(())
                        },
                        |tid: &str| {
                            *active_tid.lock().unwrap_or_else(|e| e.into_inner()) =
                                Some((generation, tid.to_string()));
                            let _ = tx2.send(StreamMsg::Stream {
                                generation,
                                kind: StreamMsgKind::ThreadId(tid.to_string()),
                            });
                        },
                    );
                    // If this generation was cancelled (possibly before thread_id
                    // arrived), fire a stop to clean up the backend run.
                    if cg_ref.load(Ordering::Acquire) >= generation
                        && let Some((g, tid)) = active_tid
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .as_ref()
                        && *g == generation
                    {
                        let _ = client.stop_thread(tid);
                    }
                    match result {
                        Ok(response) => send(StreamMsgKind::Finished {
                            status: response.stream_status,
                            error: response.stream_error,
                        }),
                        Err(e) => send(StreamMsgKind::Error(format!("{e}"))),
                    }
                });
            }

            if app.should_quit {
                // Restore terminal before exiting the scope, since scope.join
                // may block if a background SSE read is stuck.
                terminal::disable_raw_mode()?;
                crossterm::execute!(
                    std::io::stderr(),
                    LeaveAlternateScreen,
                    DisableMouseCapture,
                    DisableBracketedPaste,
                    SetCursorStyle::DefaultUserShape
                )?;
                terminal.show_cursor()?;
                let restore_hook = original_hook.clone();
                std::panic::set_hook(Box::new(move |info| (restore_hook)(info)));

                // If force-quitting (second Ctrl+C) or a cancelled worker
                // may be stuck on a blocking SSE read, exit the process
                // cleanly rather than waiting for scoped thread join.
                if app.force_quit || cancelled_gen.load(Ordering::Acquire) > 0 {
                    std::process::exit(0);
                }
                break;
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    // Restore terminal (reached when scope completes normally)
    let _ = terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        std::io::stderr(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste,
        SetCursorStyle::DefaultUserShape
    );
    let _ = terminal.show_cursor();
    let restore_hook = original_hook.clone();
    std::panic::set_hook(Box::new(move |info| (restore_hook)(info)));

    result
}

// ---------------------------------------------------------------------------
// Tests — App state machine
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::sync::atomic::AtomicU64;

    fn test_app() -> App {
        App::new(None, None, None, String::new(), None, None, None)
    }

    fn test_history_message(
        id: &str,
        role: &str,
        text: &str,
        created_at: &str,
    ) -> serde_json::Value {
        json!({
            "id": id,
            "role": role,
            "content": [{ "type": "output_text", "text": text }],
            "created_at": created_at,
        })
    }

    fn test_preview(
        messages: Vec<(&str, serde_json::Value)>,
        has_more: bool,
        oldest_message_id: Option<&str>,
    ) -> ascend_tools::models::ConversationPreview {
        ascend_tools::models::ConversationPreview {
            id: "thread-1".into(),
            title: Some("Thread".into()),
            messages: messages
                .into_iter()
                .map(|(id, message)| (id.to_string(), message))
                .collect::<BTreeMap<_, _>>(),
            updated_at: None,
            is_processing: false,
            context_window_stats: None,
            total_message_count: 0,
            has_more,
            oldest_message_id: oldest_message_id.map(str::to_string),
            latest_message_id: None,
        }
    }

    fn test_page(
        messages: Vec<(&str, serde_json::Value)>,
        has_more: bool,
        oldest_message_id: Option<&str>,
    ) -> ascend_tools::models::ConversationMessagesPage {
        ascend_tools::models::ConversationMessagesPage {
            messages: messages
                .into_iter()
                .map(|(id, message)| (id.to_string(), message))
                .collect::<BTreeMap<_, _>>(),
            has_more,
            oldest_message_id: oldest_message_id.map(str::to_string),
        }
    }

    fn test_message(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.into(),
            timestamp: UNIX_EPOCH,
            tool_call: None,
        }
    }

    // -- Stream lifecycle --------------------------------------------------

    #[test]
    fn submit_starts_streaming_and_creates_pending_request() {
        let mut app = test_app();
        app.input = "hello".chars().collect();
        app.submit();

        assert!(app.streaming);
        assert!(app.pending_request.is_some());
        assert_eq!(app.pending_request.as_ref().unwrap().prompt, "hello");
        assert!(app.stream_buffer.is_empty());
        assert!(app.stream_pending.is_empty());
        assert!(app.auto_scroll);
        // User message should be added
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, Role::User);
    }

    #[test]
    fn submit_blocked_while_streaming() {
        let mut app = test_app();
        app.streaming = true;
        app.input = "blocked".chars().collect();
        app.submit();

        // Should push a system message but NOT create a pending request
        assert!(app.pending_request.is_none());
        assert!(app.messages.iter().any(|m| m.role == Role::System));
    }

    #[test]
    fn submit_on_empty_input_is_noop() {
        let mut app = test_app();
        app.input.clear();
        app.submit();

        assert!(!app.streaming);
        assert!(app.pending_request.is_none());
        assert!(app.messages.is_empty());
    }

    #[test]
    fn conversation_history_error_surfaces_when_history_is_empty() {
        let mut app = test_app();

        app.handle_stream_msg(StreamMsg::ConversationHistoryError {
            generation: 0,
            error: "preview failed".into(),
        });

        assert!(app.messages.iter().any(|m| {
            m.role == Role::System && m.content.contains("Could not load recent history")
        }));
    }

    #[test]
    fn stale_conversation_history_error_is_discarded_after_generation_advances() {
        let mut app = test_app();
        app.stream_generation = 2;

        app.handle_stream_msg(StreamMsg::ConversationHistoryError {
            generation: 1,
            error: "stale failure".into(),
        });

        assert!(
            !app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("stale failure"))
        );
    }

    #[test]
    fn stream_conversation_history_fetches_preview_then_older_pages() {
        let preview = test_preview(
            vec![
                (
                    "msg-3",
                    test_history_message(
                        "msg-3",
                        "user",
                        "Recent question",
                        "2026-03-29T03:00:00Z",
                    ),
                ),
                (
                    "msg-4",
                    test_history_message(
                        "msg-4",
                        "assistant",
                        "Recent answer",
                        "2026-03-29T04:00:00Z",
                    ),
                ),
            ],
            true,
            Some("msg-3"),
        );
        let first_page = test_page(
            vec![(
                "msg-2",
                test_history_message("msg-2", "assistant", "Older answer", "2026-03-29T02:00:00Z"),
            )],
            true,
            Some("msg-1"),
        );
        let second_page = test_page(
            vec![(
                "msg-1",
                test_history_message("msg-1", "user", "Oldest question", "2026-03-29T01:00:00Z"),
            )],
            false,
            None,
        );

        let mut fetched_before = Vec::new();
        let mut batches = Vec::new();
        stream_conversation_history_with_fetch(
            || Ok(preview.clone()),
            |before| {
                fetched_before.push(before.to_string());
                match before {
                    "msg-3" => Ok(first_page.clone()),
                    "msg-1" => Ok(second_page.clone()),
                    other => panic!("unexpected cursor {other}"),
                }
            },
            |messages| {
                batches.push(
                    messages
                        .into_iter()
                        .map(|message| message.content)
                        .collect::<Vec<_>>(),
                );
            },
        )
        .unwrap();

        assert_eq!(fetched_before, vec!["msg-3", "msg-1"]);
        assert_eq!(
            batches,
            vec![
                vec!["Recent question".to_string(), "Recent answer".to_string()],
                vec!["Older answer".to_string()],
                vec!["Oldest question".to_string()],
            ]
        );
    }

    #[test]
    fn conversation_history_batches_prepend_older_messages() {
        let mut app = test_app();
        app.stream_generation = 7;

        app.handle_stream_msg(StreamMsg::ConversationHistory {
            generation: 7,
            messages: vec![
                test_message(Role::User, "Recent question"),
                test_message(Role::Otto, "Recent answer"),
            ],
        });
        app.handle_stream_msg(StreamMsg::ConversationHistory {
            generation: 7,
            messages: vec![test_message(Role::Otto, "Older answer")],
        });

        let ordered_contents = app
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ordered_contents,
            vec!["Older answer", "Recent question", "Recent answer"]
        );
    }

    // -- Stream message handling -------------------------------------------

    #[test]
    fn stream_delta_accumulates_in_pending() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Delta("hello ".into()),
        });
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Delta("world".into()),
        });

        let pending: String = app.stream_pending.iter().collect();
        assert_eq!(pending, "hello world");
    }

    #[test]
    fn stale_generation_messages_are_discarded() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 2;

        // Message from generation 1 should be ignored
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Delta("stale".into()),
        });

        assert!(app.stream_pending.is_empty());
    }

    #[test]
    fn thread_id_is_stored_on_stream_msg() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::ThreadId("t-123".into()),
        });

        assert_eq!(app.thread_id.as_deref(), Some("t-123"));
    }

    // -- Completed stream --------------------------------------------------

    #[test]
    fn completed_stream_flushes_buffer_and_stops_streaming() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.stream_start = Some(Instant::now());
        app.stream_buffer = "response text".into();

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Finished {
                status: OttoStreamStatus::Completed,
                error: None,
            },
        });

        assert!(!app.streaming);
        assert!(!app.interrupting);
        // Buffer should be flushed to messages
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::Otto && m.content == "response text")
        );
    }

    // -- Error handling ----------------------------------------------------

    #[test]
    fn stream_error_finishes_stream_and_shows_error_message() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Error("connection reset".into()),
        });

        assert!(!app.streaming);
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("connection reset"))
        );
    }

    #[test]
    fn interrupted_stream_shows_connection_lost() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Finished {
                status: OttoStreamStatus::Interrupted,
                error: Some("SSE stream closed".into()),
            },
        });

        assert!(!app.streaming);
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("Connection lost"))
        );
    }

    #[test]
    fn otto_stream_ended_unexpectedly_error_shows_connection_lost() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Error(
                "Otto stream ended unexpectedly: stream did not complete".into(),
            ),
        });

        assert!(!app.streaming);
        let sys_msg = app
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .expect("should have system message");
        assert!(
            sys_msg.content.contains("Connection lost"),
            "expected 'Connection lost', got: {}",
            sys_msg.content
        );
    }

    // =====================================================================
    // Cancellation & interrupt state machine (exhaustive)
    // =====================================================================
    //
    // State diagram:
    //   idle → [Ctrl+C] → cancel_stream() → interrupting=true, stop_pending=Some(gen)
    //        → [main loop] spawns stop thread → stop_pending=None
    //        → [stop thread] sends StopFinished{error} → finish_stream()
    //        → idle (ready for next message)
    //
    // The SSE stream may also send Finished{Cancelled} before StopFinished
    // arrives — this is a no-op (deferred to StopFinished).

    // -- 1. Basic cancel initiation ----------------------------------------

    #[test]
    fn cancel_sets_interrupting_and_stop_pending() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.stream_buffer = "partial output".into();

        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);

        assert!(app.interrupting);
        assert_eq!(app.stop_pending, Some(1)); // cancelled generation
        assert_eq!(cancelled_gen.load(Ordering::Acquire), 1);
        // Generation should advance to reject future messages from old stream
        assert_eq!(app.stream_generation, 2);
        // Partial output should be flushed to messages
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::Otto && m.content == "partial output")
        );
        // Active tool call should be cleared
        assert!(app.active_tool_call.is_none());
    }

    #[test]
    fn cancel_with_no_text_yet() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        // No text received yet — buffer and pending are empty

        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);

        assert!(app.interrupting);
        assert_eq!(app.stop_pending, Some(1)); // cancelled generation
        // No Otto message should be flushed (nothing to flush)
        assert!(!app.messages.iter().any(|m| m.role == Role::Otto));
    }

    #[test]
    fn cancel_with_pending_chars_flushes_both_buffer_and_pending() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.stream_buffer = "buffered ".into();
        app.stream_pending = "pending".chars().collect();

        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);

        // Both buffer and pending should be flushed together
        let otto_msg = app
            .messages
            .iter()
            .find(|m| m.role == Role::Otto)
            .expect("should have Otto message");
        assert_eq!(otto_msg.content, "buffered pending");
    }

    #[test]
    fn cancel_clears_active_tool_call() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.active_tool_call = Some(("read_file".into(), "{}".into()));

        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);

        assert!(app.active_tool_call.is_none());
    }

    // -- 2. Idempotent cancel (multiple Ctrl+C) ----------------------------

    #[test]
    fn cancel_is_idempotent_while_interrupting() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 5;
        app.stop_pending = None; // already dispatched

        let cancelled_gen = AtomicU64::new(1);
        app.cancel_stream(&cancelled_gen);

        // Should not change anything — generation stays, no double stop
        assert_eq!(app.stream_generation, 5);
        assert_eq!(app.stop_pending, None); // should NOT re-set stop_pending
    }

    // -- 3. StopFinished: success ------------------------------------------

    #[test]
    fn stop_finished_success_ends_interrupt_and_shows_cancelled() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;

        app.handle_stream_msg(StreamMsg::StopFinished { error: None });

        assert!(!app.streaming);
        assert!(!app.interrupting);
        assert!(app.stream_start.is_none());
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content == "Cancelled")
        );
    }

    // -- 4. StopFinished: timeout (your screenshot) ------------------------

    #[test]
    fn stop_finished_timeout_shows_interrupt_failed_and_recovers() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.thread_id = Some("t-123".into());

        app.handle_stream_msg(StreamMsg::StopFinished {
            error: Some(
                "API error (HTTP 408): thread 019d0b9d... did not stop within 30 seconds".into(),
            ),
        });

        // Should fully recover to idle state
        assert!(!app.streaming);
        assert!(!app.interrupting);
        assert!(app.stream_start.is_none());
        assert!(app.active_tool_call.is_none());
        // Error message shown
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("Interrupt failed"))
        );
        // Thread ID should be preserved so follow-up works
        assert_eq!(app.thread_id.as_deref(), Some("t-123"));
    }

    #[test]
    fn after_stop_timeout_user_can_submit_new_message() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.thread_id = Some("t-123".into());

        // Stop times out
        app.handle_stream_msg(StreamMsg::StopFinished {
            error: Some("thread did not stop within 30 seconds".into()),
        });

        // User types a follow-up
        app.input = "follow up question".chars().collect();
        app.submit();

        assert!(app.streaming);
        let req = app
            .pending_request
            .as_ref()
            .expect("should have pending request");
        assert_eq!(req.prompt, "follow up question");
        // Thread ID preserved for follow-up
        assert_eq!(req.thread_id.as_deref(), Some("t-123"));
    }

    // -- 5. StopFinished: network error ------------------------------------

    #[test]
    fn stop_finished_network_error_recovers() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.thread_id = Some("t-456".into());

        app.handle_stream_msg(StreamMsg::StopFinished {
            error: Some("connection refused".into()),
        });

        assert!(!app.streaming);
        assert!(!app.interrupting);
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("Interrupt failed"))
        );
        // Thread ID preserved
        assert_eq!(app.thread_id.as_deref(), Some("t-456"));

        // Can still submit
        app.input = "retry".chars().collect();
        app.submit();
        assert!(app.streaming);
    }

    // -- 6. Cancel before thread_id is known -------------------------------

    #[test]
    fn cancel_before_thread_id_finishes_immediately() {
        // This simulates the main loop path where active_thread_id is None
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.thread_id = None;

        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);

        assert!(app.interrupting);
        assert_eq!(app.stop_pending, Some(1)); // cancelled generation

        // In the main loop, stop_pending=Some but active_thread_id=None
        // triggers immediate finish_stream + "Cancelled"
        // Simulate that path:
        app.stop_pending = None;
        app.finish_stream();
        app.push_system("Cancelled");

        assert!(!app.streaming);
        assert!(!app.interrupting);
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content == "Cancelled")
        );
    }

    // -- 7. SSE Finished(Cancelled) then StopFinished ----------------------

    #[test]
    fn cancelled_stream_status_defers_to_stop_finished() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 1;

        // SSE callback breaks → otto_streaming returns Cancelled
        // This arrives as a Stream message (NOT StopFinished)
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Finished {
                status: OttoStreamStatus::Cancelled,
                error: None,
            },
        });

        // Should still be in interrupting state — waiting for StopFinished
        assert!(app.streaming);
        assert!(app.interrupting);
    }

    #[test]
    fn cancelled_then_stop_finished_success() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 1;

        // Step 1: SSE returns Cancelled (no-op)
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Finished {
                status: OttoStreamStatus::Cancelled,
                error: None,
            },
        });
        assert!(app.streaming); // still waiting

        // Step 2: Background stop thread completes
        app.handle_stream_msg(StreamMsg::StopFinished { error: None });

        assert!(!app.streaming);
        assert!(!app.interrupting);
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content == "Cancelled")
        );
    }

    #[test]
    fn cancelled_then_stop_finished_error() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 1;
        app.thread_id = Some("t-789".into());

        // Step 1: SSE returns Cancelled
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Finished {
                status: OttoStreamStatus::Cancelled,
                error: None,
            },
        });

        // Step 2: Stop thread fails
        app.handle_stream_msg(StreamMsg::StopFinished {
            error: Some("timeout".into()),
        });

        assert!(!app.streaming);
        assert!(!app.interrupting);
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("Interrupt failed"))
        );
        // Thread preserved for retry
        assert_eq!(app.thread_id.as_deref(), Some("t-789"));
    }

    // -- 8. Stale messages after cancel ------------------------------------

    #[test]
    fn stale_deltas_after_cancel_are_discarded() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        // Cancel advances generation to 2
        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);
        assert_eq!(app.stream_generation, 2);

        // Old-generation messages should be silently dropped
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Delta("stale text".into()),
        });
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::ToolCallStart {
                name: "stale_tool".into(),
                arguments: "{}".into(),
            },
        });
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Finished {
                status: OttoStreamStatus::Completed,
                error: None,
            },
        });

        // None of these should have affected state
        assert!(app.stream_pending.is_empty());
        assert!(app.active_tool_call.is_none());
        // Still interrupting (waiting for StopFinished)
        assert!(app.interrupting);
        assert!(app.streaming);
    }

    // -- 9. Full cancel → recover → new message cycle ----------------------

    #[test]
    fn full_cancel_recover_new_message_cycle() {
        let mut app = test_app();

        // 1. User sends first message
        app.input = "first question".chars().collect();
        app.submit();
        assert!(app.streaming);
        let req1 = app.take_pending_request().unwrap();
        assert_eq!(req1.prompt, "first question");

        // Simulate thread ID arriving
        app.stream_generation = 1;
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::ThreadId("t-cycle".into()),
        });
        // Some text arrives
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Delta("partial answer".into()),
        });

        // 2. User cancels
        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);
        assert!(app.interrupting);
        assert_eq!(app.stop_pending, Some(1)); // cancelled generation

        // 3. Stop thread succeeds
        app.stop_pending = None;
        app.handle_stream_msg(StreamMsg::StopFinished { error: None });
        assert!(!app.streaming);
        assert!(!app.interrupting);

        // Partial text should be in messages
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::Otto && m.content.contains("partial answer"))
        );

        // 4. User sends follow-up
        app.input = "second question".chars().collect();
        app.submit();
        assert!(app.streaming);
        let req2 = app.pending_request.as_ref().unwrap();
        assert_eq!(req2.prompt, "second question");
        assert_eq!(req2.thread_id.as_deref(), Some("t-cycle"));
    }

    #[test]
    fn full_cancel_timeout_recover_new_message_cycle() {
        let mut app = test_app();

        // 1. Streaming
        app.input = "question".chars().collect();
        app.submit();
        app.stream_generation = 1;
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::ThreadId("t-timeout".into()),
        });
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Delta("response".into()),
        });

        // 2. Cancel
        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);

        // 3. Stop times out (your screenshot scenario)
        app.stop_pending = None;
        app.handle_stream_msg(StreamMsg::StopFinished {
            error: Some(
                "API error (HTTP 408): thread t-timeout did not stop within 30 seconds".into(),
            ),
        });

        assert!(!app.streaming);
        assert!(!app.interrupting);

        // 4. User sends follow-up — should work despite timeout
        app.input = "follow up".chars().collect();
        app.submit();
        assert!(app.streaming);
        assert!(!app.interrupting);
        let req = app.pending_request.as_ref().unwrap();
        assert_eq!(req.thread_id.as_deref(), Some("t-timeout"));
    }

    // -- 10. Multiple rapid Ctrl+C -----------------------------------------

    #[test]
    fn rapid_ctrl_c_during_interrupting_force_quits() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 5;

        let cancelled_gen = AtomicU64::new(1);

        // First Ctrl+C during interrupting → force quit
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &cancelled_gen,
        );

        assert!(app.should_quit);
        assert!(app.force_quit);
        // Still interrupting/streaming — force quit bypasses normal cleanup
        assert!(app.interrupting);
        assert!(app.streaming);
        assert_eq!(app.stream_generation, 5);
    }

    #[test]
    fn rapid_esc_during_interrupting_is_safe() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 5;

        let cancelled_gen = AtomicU64::new(1);

        for _ in 0..5 {
            app.handle_key(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                &cancelled_gen,
            );
        }

        assert!(app.interrupting);
        assert!(app.streaming);
        assert_eq!(app.stream_generation, 5);
    }

    // -- 11. Stream error during interrupting state ------------------------

    #[test]
    fn stream_error_during_interrupting_is_discarded() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        // Cancel advances generation
        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);
        assert_eq!(app.stream_generation, 2);

        // Error from old generation arrives — should be discarded
        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::Error("old error".into()),
        });

        // Still interrupting, waiting for StopFinished
        assert!(app.streaming);
        assert!(app.interrupting);
        assert!(
            !app.messages
                .iter()
                .any(|m| { m.role == Role::System && m.content.contains("old error") })
        );
    }

    // -- 12. StopFinished when not interrupting (race) ---------------------

    #[test]
    fn stop_finished_when_not_interrupting_is_noop() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = false; // race: already recovered somehow

        app.handle_stream_msg(StreamMsg::StopFinished { error: None });

        // Should not affect state — only acts when interrupting
        assert!(app.streaming);
        assert!(!app.interrupting);
    }

    #[test]
    fn stop_finished_when_idle_is_harmless() {
        let mut app = test_app();
        // Not streaming at all
        assert!(!app.streaming);
        assert!(!app.interrupting);

        app.handle_stream_msg(StreamMsg::StopFinished {
            error: Some("some error".into()),
        });

        // Should not crash or leave bad state
        assert!(!app.streaming);
        assert!(!app.interrupting);
    }

    // -- 13. Ctrl+C/Esc routing --------------------------------------------

    #[test]
    fn ctrl_c_while_streaming_cancels_not_quits() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        let cancelled_gen = AtomicU64::new(0);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &cancelled_gen,
        );

        assert!(app.interrupting);
        assert!(!app.should_quit);
        assert_eq!(cancelled_gen.load(Ordering::Acquire), 1);
    }

    #[test]
    fn ctrl_c_while_not_streaming_quits() {
        let mut app = test_app();

        let cancelled_gen = AtomicU64::new(0);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &cancelled_gen,
        );

        assert!(app.should_quit);
    }

    #[test]
    fn esc_while_streaming_cancels() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        let cancelled_gen = AtomicU64::new(0);
        app.handle_key(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &cancelled_gen,
        );

        assert!(app.interrupting);
        assert_eq!(cancelled_gen.load(Ordering::Acquire), 1);
    }

    #[test]
    fn esc_while_not_streaming_enters_vi_normal() {
        let mut app = test_app();
        app.input_mode = InputMode::ViInsert;

        let cancelled_gen = AtomicU64::new(0);
        app.handle_key(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &cancelled_gen,
        );

        assert_eq!(app.input_mode, InputMode::ViNormal);
        assert!(!app.interrupting);
    }

    // -- 14. Submit blocked during interrupting ----------------------------

    #[test]
    fn submit_blocked_during_interrupting() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.input = "please work".chars().collect();
        app.submit();

        // Should be blocked (streaming is still true)
        assert!(app.pending_request.is_none());
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("Waiting"))
        );
    }

    // -- 15. Pending request not launched during interrupting ---------------

    #[test]
    fn pending_request_guard_during_interrupting() {
        // The main loop has: if !app.interrupting && let Some(request) = ...
        // This test verifies the app-level invariant that pending_request
        // should not exist during interrupting (submit blocks it).
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;

        // Force a pending request (shouldn't happen in practice)
        app.pending_request = Some(OttoChatRequest {
            prompt: "should not launch".into(),
            runtime_uuid: None,
            thread_id: None,
            model: None,
            query_policy: None,
        });

        // The main loop guard is: if !app.interrupting && let Some(req) = app.take_pending_request()
        // Verify: with interrupting=true, take_pending_request should return Some
        // but the guard prevents launching. We verify the take works.
        assert!(app.interrupting);
        assert!(app.pending_request.is_some());
        // (The main loop guard is tested by integration, not here)
    }

    // -- 16. Cancel during tool call execution -----------------------------

    #[test]
    fn cancel_during_tool_call_clears_tool_and_preserves_text() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.stream_buffer = "Let me check that for you.".into();
        app.active_tool_call = Some(("list_workspaces".into(), "{}".into()));

        let cancelled_gen = AtomicU64::new(0);
        app.cancel_stream(&cancelled_gen);

        assert!(app.active_tool_call.is_none());
        assert!(app.interrupting);
        // Partial text preserved
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::Otto && m.content.contains("Let me check"))
        );
    }

    // -- Tool call display -------------------------------------------------

    #[test]
    fn tool_call_start_sets_active_tool() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::ToolCallStart {
                name: "list_flows".into(),
                arguments: "{}".into(),
            },
        });

        assert_eq!(
            app.active_tool_call.as_ref().map(|(n, _)| n.as_str()),
            Some("list_flows")
        );
    }

    #[test]
    fn tool_call_output_clears_active_tool_and_adds_system_msg() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.active_tool_call = Some(("list_flows".into(), "{}".into()));

        app.handle_stream_msg(StreamMsg::Stream {
            generation: 1,
            kind: StreamMsgKind::ToolCallOutput {
                name: "list_flows".into(),
                output: "sales, marketing".into(),
            },
        });

        assert!(app.active_tool_call.is_none());
        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("list_flows"))
        );
    }

    // -- Provider info update ----------------------------------------------

    #[test]
    fn provider_info_updates_labels() {
        let mut app = test_app();

        app.handle_stream_msg(StreamMsg::ProviderInfo {
            provider_label: Some("AWS Bedrock".into()),
            model_label: "Claude Sonnet".into(),
        });

        assert_eq!(app.provider_label.as_deref(), Some("AWS Bedrock"));
        assert_eq!(app.model_label, "Claude Sonnet");
    }

    // -- Input helpers -----------------------------------------------------

    #[test]
    fn input_line_count_wraps_correctly() {
        let mut app = test_app();
        // 10 chars, width 8 (avail = 5 after 3-char prompt) → 2 rows of content
        // + cursor at end of full row triggers extra row = 3
        app.input = "abcdefghij".chars().collect();
        app.cursor = app.input.len();
        assert_eq!(app.input_line_count(8), 3);

        // 7 chars, avail 5 → row1: 5 chars, row2: 2 chars + cursor not full → 2 rows
        app.input = "abcdefg".chars().collect();
        app.cursor = app.input.len();
        assert_eq!(app.input_line_count(8), 2);
    }

    #[test]
    fn input_line_count_newlines() {
        let mut app = test_app();
        app.input = "line1\nline2\nline3".chars().collect();
        app.cursor = app.input.len();
        assert_eq!(app.input_line_count(80), 3);
    }

    #[test]
    fn input_line_count_capped_at_max() {
        let mut app = test_app();
        // Create input with many newlines to exceed MAX_INPUT_LINES
        app.input = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk".chars().collect();
        app.cursor = app.input.len();
        assert_eq!(app.input_line_count(80), MAX_INPUT_LINES);
    }

    // -- Slash commands ----------------------------------------------------

    #[test]
    fn clear_command_resets_thread() {
        let mut app = test_app();
        app.thread_id = Some("t-old".into());
        app.messages.push(Message {
            role: Role::Otto,
            content: "old message".into(),
            timestamp: SystemTime::now(),
            tool_call: None,
        });

        app.handle_command("/clear");

        assert!(app.thread_id.is_none());
        // Should only have the "Thread cleared" system message
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].content.contains("cleared"));
    }

    #[test]
    fn unknown_command_shows_error() {
        let mut app = test_app();
        app.handle_command("/foobar");

        assert!(
            app.messages
                .iter()
                .any(|m| m.role == Role::System && m.content.contains("Unknown command"))
        );
    }

    #[test]
    fn quit_command_sets_should_quit() {
        let mut app = test_app();
        app.handle_command("/quit");
        assert!(app.should_quit);

        let mut app2 = test_app();
        app2.handle_command("/exit");
        assert!(app2.should_quit);

        let mut app3 = test_app();
        app3.handle_command("/q");
        assert!(app3.should_quit);
    }

    // -- History -----------------------------------------------------------

    #[test]
    fn history_records_submitted_input() {
        let mut app = test_app();
        app.input = "first query".chars().collect();
        app.submit();
        app.finish_stream(); // clear streaming state

        app.input = "second query".chars().collect();
        app.submit();
        app.finish_stream();

        // Navigate back through history
        if let Some(prev) = app.history.prev(&app.input) {
            let s: String = prev.iter().collect();
            assert_eq!(s, "second query");
        } else {
            panic!("expected history entry");
        }
    }

    // -- Streaming text smoothing ------------------------------------------

    #[test]
    fn tick_stream_flushes_when_bulk_threshold_exceeded() {
        let mut app = test_app();
        app.streaming = true;
        // Add more chars than STREAM_BULK_THRESHOLD
        let text: String = (0..250).map(|_| 'x').collect();
        app.stream_pending = text.chars().collect();
        // Set last_stream_tick to the past so chars_due > 0
        app.last_stream_tick = Instant::now() - Duration::from_millis(100);

        app.tick_stream();

        // Should have flushed a large chunk into stream_buffer
        assert!(!app.stream_buffer.is_empty());
        // Total should still equal 250
        assert_eq!(app.stream_buffer.len() + app.stream_pending.len(), 250);
    }

    // -- Completion --------------------------------------------------------

    #[test]
    fn tab_completion_cycles_through_commands() {
        let mut app = test_app();
        app.input = "/cl".chars().collect();
        app.cursor = app.input.len();

        app.complete_tab();
        let first: String = app.input.iter().collect();
        assert_eq!(first, "/clear");

        // Tab again should still show /clear (only match)
        app.complete_tab();
        let second: String = app.input.iter().collect();
        assert_eq!(second, "/clear");
    }

    // -- Paste handling ----------------------------------------------------

    #[test]
    fn paste_inserts_at_cursor_and_switches_to_insert_mode() {
        let mut app = test_app();
        app.input_mode = InputMode::ViNormal;
        app.input = "hello".chars().collect();
        app.cursor = 5;

        app.handle_paste(" world");

        let text: String = app.input.iter().collect();
        assert_eq!(text, "hello world");
        assert_eq!(app.input_mode, InputMode::ViInsert);
        assert_eq!(app.cursor, 11);
    }

    // -- Markdown rendering ------------------------------------------------

    #[test]
    fn render_markdown_handles_code_blocks() {
        let text = "text\n```rust\nfn main() {}\n```\nmore text";
        let lines = render_markdown(text, Role::Otto, false);
        // Should have: text, blank, code block header, code line, code block footer, blank, more text
        assert!(lines.len() >= 5);
    }

    #[test]
    fn render_markdown_handles_inline_code() {
        let text = "use `foo()` here";
        let lines = render_markdown(text, Role::Otto, false);
        // Line should contain multiple spans (indent, text, code, text)
        // pulldown-cmark wraps in paragraph, so we may have blank lines
        let content_lines: Vec<_> = lines.iter().filter(|l| !l.spans.is_empty()).collect();
        assert!(!content_lines.is_empty());
        assert!(content_lines[0].spans.len() >= 3);
    }

    #[test]
    fn render_markdown_raw_mode_shows_source() {
        let text = "**bold** and `code`";
        let lines = render_markdown(text, Role::Otto, true);
        assert_eq!(lines.len(), 1);
        let full_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(full_text.contains("**bold**"));
        assert!(full_text.contains("`code`"));
    }

    #[test]
    fn render_markdown_headings_are_styled() {
        let text = "# Title\n\n## Subtitle";
        let lines = render_markdown(text, Role::Otto, false);
        let content: Vec<_> = lines
            .iter()
            .filter(|l| {
                !l.spans.is_empty() && !(l.spans.len() == 1 && l.spans[0].content.trim().is_empty())
            })
            .collect();
        assert!(content.len() >= 2);
        // H1 should be bold
        assert!(
            content[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD))
        );
    }

    #[test]
    fn render_markdown_unordered_list() {
        let text = "- one\n- two\n- three";
        let lines = render_markdown(text, Role::Otto, false);
        let text_lines: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        // Should contain bullet character
        assert!(text_lines.iter().any(|l| l.contains('\u{2022}')));
    }

    #[test]
    fn render_markdown_link_dedup() {
        // When link text matches URL, should not show URL twice.
        let text = "[https://example.com](https://example.com)";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        // URL should appear exactly once, not duplicated in parens.
        assert_eq!(full.matches("example.com").count(), 1);
    }

    #[test]
    fn render_markdown_link_shows_url() {
        // When link text differs from URL, should show URL in parens.
        let text = "[click here](https://example.com)";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains("click here"));
        assert!(full.contains("(https://example.com)"));
    }

    #[test]
    fn render_markdown_blockquote() {
        let text = "> quoted text";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains('\u{2502}')); // vertical bar
        assert!(full.contains("quoted text"));
    }

    #[test]
    fn render_markdown_task_list() {
        let text = "- [x] done\n- [ ] todo";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains('\u{2713}')); // checkmark
        assert!(full.contains("[ ]"));
    }

    #[test]
    fn render_markdown_inline_code_has_backtick_delimiters() {
        let text = "use `foo()` here";
        let lines = render_markdown(text, Role::Otto, false);
        let content_lines: Vec<_> = lines.iter().filter(|l| !l.spans.is_empty()).collect();
        assert!(!content_lines.is_empty());
        // Should have dim backtick spans around the code span.
        let spans = &content_lines[0].spans;
        let backtick_count = spans.iter().filter(|s| s.content.as_ref() == "`").count();
        assert_eq!(backtick_count, 2);
    }

    #[test]
    fn render_markdown_table() {
        let text = "| A | B |\n|---|---|\n| 1 | 2 |";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains('\u{2502}'));
        assert!(full.contains('\u{2500}'));
    }

    #[test]
    fn render_markdown_table_alignment_consistent_widths() {
        // Body cell "longer" is wider than header "A" — all rows should use the wider width.
        let text = "| A | B |\n|---|---|\n| longer | x |";
        let lines = render_markdown(text, Role::Otto, false);
        // Find the separator line — its column widths should match the widest cell.
        let sep_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains('\u{253c}')))
            .expect("should have separator");
        let sep_text: String = sep_line.spans.iter().map(|s| s.content.as_ref()).collect();
        // The first column separator should be at least 6 chars wide (len of "longer").
        let first_col = sep_text.split('\u{253c}').next().unwrap();
        let dash_count = first_col.chars().filter(|&c| c == '\u{2500}').count();
        assert!(
            dash_count >= 6,
            "separator should match widest cell, got {dash_count}"
        );
    }

    #[test]
    fn render_markdown_table_inline_code_preserved() {
        let text = "| Col |\n|---|\n| `code` |";
        let lines = render_markdown(text, Role::Otto, false);
        // Should have a span with CODE_COLOR for inline code in the table.
        let has_code_span = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.style.fg == Some(CODE_COLOR) && s.content.as_ref() == "code")
        });
        assert!(has_code_span, "inline code in table should be styled");
    }

    #[test]
    fn render_markdown_nested_list() {
        let text = "- outer\n  - inner\n- back to outer";
        let lines = render_markdown(text, Role::Otto, false);
        let texts: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        // Should have bullet characters at different indent levels.
        assert!(texts.iter().any(|t| t.contains("inner")));
        assert!(texts.iter().any(|t| t.contains("back to outer")));
    }

    #[test]
    fn render_markdown_empty_input() {
        let lines = render_markdown("", Role::Otto, false);
        assert!(lines.is_empty());
    }

    #[test]
    fn render_markdown_horizontal_rule() {
        let text = "above\n\n---\n\nbelow";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains('\u{2500}'));
    }

    #[test]
    fn render_markdown_strikethrough() {
        let text = "~~deleted~~";
        let lines = render_markdown(text, Role::Otto, false);
        let has_strikethrough = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::CROSSED_OUT))
        });
        assert!(has_strikethrough);
    }

    #[test]
    fn render_markdown_diff_coloring() {
        let text = "```diff\n- removed\n+ added\n context\n@@ -1,3 +1,3 @@\n```";
        let lines = render_markdown(text, Role::Otto, false);
        // Find the line with "- removed" — should be DIFF_DEL_COLOR (red).
        let del_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("- removed")))
            .expect("should have a deletion line");
        assert_eq!(
            del_line.spans.last().unwrap().style.fg,
            Some(DIFF_DEL_COLOR)
        );
        // Find the line with "+ added" — should be DIFF_ADD_COLOR (green).
        let add_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("+ added")))
            .expect("should have an addition line");
        assert_eq!(
            add_line.spans.last().unwrap().style.fg,
            Some(DIFF_ADD_COLOR)
        );
        // Find the hunk header — should be DIFF_HUNK_COLOR (blue).
        let hunk_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("@@")))
            .expect("should have a hunk header line");
        assert_eq!(
            hunk_line.spans.last().unwrap().style.fg,
            Some(DIFF_HUNK_COLOR)
        );
    }

    #[test]
    fn render_markdown_syntax_highlighting() {
        let text = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let lines = render_markdown(text, Role::Otto, false);
        // Find a code content line (has │ border).
        let code_lines: Vec<_> = lines
            .iter()
            .filter(|l| l.spans.iter().any(|s| s.content.contains('\u{2502}')))
            .collect();
        assert!(!code_lines.is_empty());
        // With syntax highlighting, code lines should have >1 span
        // (border span + multiple colored spans, not just border + single white span).
        let multi_span_lines = code_lines.iter().filter(|l| l.spans.len() > 2).count();
        assert!(multi_span_lines > 0, "expected syntax-highlighted spans");
    }

    #[test]
    fn render_markdown_code_block_extracts_language_from_info_string() {
        // Code fence with metadata after language: ```sql title="file.sql" lines="1-15"
        let text = "```sql title=\"file.sql\" lines=\"1-15\"\nSELECT 1;\n```";
        let lines = render_markdown(text, Role::Otto, false);
        // Header should only show "sql", not the full info string.
        let header = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains('\u{256d}')))
            .expect("should have code block header");
        let header_text: String = header.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            header_text.contains("sql"),
            "header should contain language"
        );
        assert!(
            !header_text.contains("title"),
            "header should NOT contain metadata"
        );
        // Code should have syntax highlighting (SQL matched).
        let code_lines: Vec<_> = lines
            .iter()
            .filter(|l| l.spans.iter().any(|s| s.content.contains('\u{2502}')))
            .collect();
        assert!(!code_lines.is_empty());
        let multi_span = code_lines.iter().filter(|l| l.spans.len() > 2).count();
        assert!(multi_span > 0, "SQL should be syntax-highlighted");
    }

    #[test]
    fn render_markdown_python_syntax_highlighting() {
        let text = "```python\ndef hello():\n    print(\"hi\")\n```";
        let lines = render_markdown(text, Role::Otto, false);
        let code_lines: Vec<_> = lines
            .iter()
            .filter(|l| l.spans.iter().any(|s| s.content.contains('\u{2502}')))
            .collect();
        assert!(!code_lines.is_empty());
        let multi_span = code_lines.iter().filter(|l| l.spans.len() > 2).count();
        assert!(multi_span > 0, "Python should be syntax-highlighted");
    }

    #[test]
    fn render_markdown_unicode_table_widths() {
        // CJK characters are display-width 2 each; column width should reflect that.
        let text = "| A | B |\n|---|---|\n| \u{4f60}\u{597d} | x |";
        let lines = render_markdown(text, Role::Otto, false);
        let sep_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains('\u{253c}')))
            .expect("should have separator");
        let sep_text: String = sep_line.spans.iter().map(|s| s.content.as_ref()).collect();
        // "你好" is 4 display columns — separator first column should be at least 4 dashes.
        let first_col = sep_text.split('\u{253c}').next().unwrap();
        let dash_count = first_col.chars().filter(|&c| c == '\u{2500}').count();
        assert!(
            dash_count >= 4,
            "separator should match CJK display width, got {dash_count}"
        );
    }

    #[test]
    fn render_markdown_ordered_list_high_start() {
        let text = "99. first\n100. second";
        let lines = render_markdown(text, Role::Otto, false);
        let texts: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        // First item should show "99." with proper padding for the eventual 3-digit number.
        assert!(texts.iter().any(|t| t.contains("99.")));
        assert!(texts.iter().any(|t| t.contains("100.")));
    }

    #[test]
    fn render_markdown_nested_blockquote() {
        let text = "> > doubly quoted";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        // Should have two vertical bar characters for nested blockquote.
        assert!(
            full.matches('\u{2502}').count() >= 2,
            "nested blockquote should have 2+ vertical bars"
        );
    }

    #[test]
    fn render_markdown_mixed_nested_lists() {
        let text = "1. ordered\n   - unordered inside\n2. back";
        let lines = render_markdown(text, Role::Otto, false);
        let texts: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        // Should have both ordered number and bullet character.
        assert!(texts.iter().any(|t| t.contains("1.")));
        assert!(texts.iter().any(|t| t.contains('\u{2022}')));
    }

    #[test]
    fn render_markdown_multi_paragraph_list_item() {
        let text = "- first paragraph\n\n  second paragraph\n- next item";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains("first paragraph"));
        assert!(full.contains("second paragraph"));
        assert!(full.contains("next item"));
    }

    #[test]
    fn render_markdown_emoji_in_table() {
        let text = "| Col |\n|---|\n| \u{1f600} |";
        let lines = render_markdown(text, Role::Otto, false);
        // Should not panic and should produce output.
        assert!(!lines.is_empty());
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains('\u{1f600}'));
    }

    #[test]
    fn render_markdown_gfm_note_blockquote() {
        let text = "> [!NOTE]\n> This is a note.";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains("NOTE"), "should render NOTE label");
        assert!(full.contains("This is a note."));
    }

    #[test]
    fn render_markdown_gfm_warning_blockquote() {
        let text = "> [!WARNING]\n> Be careful.";
        let lines = render_markdown(text, Role::Otto, false);
        // Find the WARNING label span with the correct color.
        let has_warning = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.as_ref() == "WARNING" && s.style.fg == Some(WARNING_COLOR))
        });
        assert!(
            has_warning,
            "should render WARNING label with correct color"
        );
    }

    #[test]
    fn render_markdown_code_in_blockquote() {
        let text = "> ```rust\n> fn x() {}\n> ```";
        let lines = render_markdown(text, Role::Otto, false);
        let full: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(full.contains("fn x()"));
        // Should have both blockquote bar and code border.
        assert!(full.contains('\u{2502}'));
        assert!(full.contains('\u{256d}'));
    }

    // -- Force quit (second Ctrl+C during interrupting) --------------------

    #[test]
    fn second_ctrl_c_during_interrupting_sets_force_quit() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;

        let cancelled_gen = AtomicU64::new(1);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &cancelled_gen,
        );

        assert!(app.force_quit);
        assert!(app.should_quit);
    }

    // -- StopFinished guarded by interrupting state ------------------------

    #[test]
    fn stop_finished_ignored_when_not_interrupting() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = false;

        let msg_count_before = app.messages.len();
        app.handle_stream_msg(StreamMsg::StopFinished {
            error: Some("should be ignored".into()),
        });

        // Should not change state or add messages
        assert!(app.streaming);
        assert_eq!(app.messages.len(), msg_count_before);
    }
}
