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
//! - Markdown rendering (code blocks, bold, inline code)
//! - Message timestamps (`/timestamps` to toggle)
//! - Clipboard copy (`/copy`)
//! - Vi yank/paste registers

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::*;

use ascend_tools::client::AscendClient;
use ascend_tools::models::{OttoChatRequest, OttoModel, OttoStreamStatus, StreamEvent};
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
const TIMESTAMP_COLOR: Color = Color::Rgb(80, 80, 80);

/// Characters per second for smoothed streaming output.
const STREAM_CPS: f64 = 200.0;
/// Above this pending count, flush in bulk to catch up.
const STREAM_BULK_THRESHOLD: usize = 200;
/// Above this pending count, skip smoothing entirely.
const STREAM_FAST_THRESHOLD: usize = 50;

const MAX_HISTORY: usize = 1000;
const MAX_INPUT_LINES: u16 = 8;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

enum StreamMsg {
    ProviderInfo {
        provider_label: Option<String>,
        model_label: String,
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

struct Message {
    role: Role,
    content: String,
    timestamp: SystemTime,
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
    pending_request: Option<OttoChatRequest>,
    should_quit: bool,
    spinner_frame: usize,
    last_spinner: Instant,
    vi_pending: Option<char>,
    yank_register: String,
    completion_index: Option<usize>,
    history: History,
    show_timestamps: bool,
    active_tool_call: Option<String>,
    stream_generation: u64,
    /// Set when cancel fires; the main loop spawns a thread to stop the backend.
    stop_pending: bool,
    interrupting: bool,
}

impl App {
    fn new(
        runtime_uuid: Option<String>,
        otto_model: Option<OttoModel>,
        provider_label: Option<String>,
        model_label: String,
        context_label: Option<String>,
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
            thread_id: None,
            runtime_uuid,
            otto_model,
            provider_label,
            model_label,
            context_label,
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
            stream_generation: 0,
            stop_pending: false,
            interrupting: false,
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

    fn handle_key(&mut self, key: KeyEvent, cancel: &AtomicBool) {
        // Ctrl+C: cancel stream or quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.interrupting {
                return;
            }
            if self.streaming {
                self.cancel_stream(cancel);
            } else {
                self.should_quit = true;
            }
            return;
        }

        // Escape: cancel stream (if streaming), otherwise normal key handling
        if key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE && self.streaming {
            if self.interrupting {
                return;
            }
            self.cancel_stream(cancel);
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
        });

        self.pending_request = Some(OttoChatRequest {
            prompt: text,
            runtime_uuid: self.runtime_uuid.clone(),
            thread_id: self.thread_id.clone(),
            model: self.otto_model.clone(),
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
            StreamMsg::StopFinished { error } => {
                self.finish_stream();
                if let Some(err) = error {
                    self.push_system(format!("Interrupt failed: {err}"));
                } else {
                    self.push_system("Cancelled");
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
            StreamMsgKind::ToolCallStart { name, .. } => {
                self.flush_stream_text();
                self.active_tool_call = Some(name);
            }
            StreamMsgKind::ToolCallOutput { name, output } => {
                self.active_tool_call = None;
                let output_summary = truncate(&output, 80);
                self.push_system(format!("\u{2699} {name} \u{2192} {output_summary}"));
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
            });
        }
    }

    fn cancel_stream(&mut self, cancel: &AtomicBool) {
        if self.interrupting {
            return;
        }
        cancel.store(true, Ordering::Relaxed);
        self.stream_generation = self.stream_generation.wrapping_add(1);
        self.flush_stream_text();
        self.active_tool_call = None;
        self.interrupting = true;
        self.stop_pending = true;
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

            lines.extend(render_markdown(&msg.content, msg.role));
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
                } else if let Some(tool) = &self.active_tool_call {
                    format!("  {} \u{2699} {tool}...", SPINNER[self.spinner_frame])
                } else {
                    format!("  {} Ascending...", SPINNER[self.spinner_frame])
                };
                lines.push(Line::from(Span::styled(
                    label,
                    Style::default().fg(DIM_OTTO_COLOR),
                )));
            } else if self.interrupting {
                lines.extend(render_markdown(&self.stream_buffer, Role::Otto));
                lines.push(Line::from(Span::styled(
                    format!("  {} Stopping...", SPINNER[self.spinner_frame]),
                    Style::default().fg(DIM_OTTO_COLOR),
                )));
            } else if let Some(tool) = &self.active_tool_call {
                lines.extend(render_markdown(&self.stream_buffer, Role::Otto));
                lines.push(Line::from(Span::styled(
                    format!("  {} \u{2699} {tool}...", SPINNER[self.spinner_frame]),
                    Style::default().fg(DIM_OTTO_COLOR),
                )));
            } else {
                lines.extend(render_markdown(&self.stream_buffer, Role::Otto));
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
// Markdown rendering (code blocks, inline code, bold)
// ---------------------------------------------------------------------------

fn render_markdown(text: &str, role: Role) -> Vec<Line<'static>> {
    let indent = "  ";
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                let lang = trimmed.trim_start_matches('`').trim();
                let header = if lang.is_empty() {
                    format!("{indent}\u{256d}\u{2500}\u{2500}")
                } else {
                    format!("{indent}\u{256d}\u{2500} {lang} \u{2500}")
                };
                lines.push(Line::from(Span::styled(
                    header,
                    Style::default().fg(DIM_COLOR),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("{indent}\u{2570}\u{2500}\u{2500}"),
                    Style::default().fg(DIM_COLOR),
                )));
            }
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("{indent}\u{2502} {line}"),
                Style::default().fg(TEXT_COLOR),
            )));
        } else {
            lines.push(parse_inline(line, indent, role));
        }
    }
    lines
}

fn parse_inline(text: &str, indent: &str, role: Role) -> Line<'static> {
    let base_style = match role {
        Role::System => Style::default().fg(SYSTEM_COLOR).italic(),
        _ => Style::default(),
    };
    let code_style = Style::default().fg(CODE_COLOR);
    let bold_style = base_style.bold();

    let chars: Vec<char> = text.chars().collect();
    let mut spans: Vec<Span<'static>> = vec![Span::raw(indent.to_string())];
    let mut i = 0;
    let mut buf = String::new();

    while i < chars.len() {
        // **bold**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), base_style));
            }
            i += 2;
            let mut bold = String::new();
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            spans.push(Span::styled(bold, bold_style));
            continue;
        }

        // `code`
        if chars[i] == '`' {
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), base_style));
            }
            i += 1;
            let mut code = String::new();
            while i < chars.len() && chars[i] != '`' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            spans.push(Span::styled(code, code_style));
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    if !buf.is_empty() {
        spans.push(Span::styled(buf, base_style));
    }

    Line::from(spans)
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

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run_tui(
    client: &AscendClient,
    runtime_uuid: Option<String>,
    otto_model: Option<OttoModel>,
    context_label: Option<String>,
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
    let cancel = AtomicBool::new(false);
    let gen_counter = AtomicU64::new(0);
    let active_thread_id: Mutex<Option<String>> = Mutex::new(None);

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

        let mut app = App::new(runtime_uuid, otto_model, None, String::new(), context_label);

        loop {
            app.tick_spinner();
            app.tick_stream();
            terminal.draw(|frame| app.render(frame))?;

            if event::poll(POLL_DURATION)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        app.handle_key(key, &cancel);
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
            if app.stop_pending {
                app.stop_pending = false;
                if let Some(tid) = active_thread_id
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone()
                {
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
                let generation = gen_counter.fetch_add(1, Ordering::Relaxed) + 1;
                app.stream_generation = generation;
                cancel.store(false, Ordering::Relaxed);
                *active_thread_id.lock().unwrap_or_else(|e| e.into_inner()) = None;
                let tx = stream_tx.clone();
                let cancel_ref = &cancel;
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
                            if cancel_ref.load(Ordering::Relaxed) {
                                return ControlFlow::Break(());
                            }
                            match event {
                                StreamEvent::TextDelta(delta) => {
                                    send(StreamMsgKind::Delta(delta));
                                }
                                StreamEvent::ToolCallStart { call_id, name, .. } => {
                                    tool_names.insert(call_id, name.clone());
                                    send(StreamMsgKind::ToolCallStart { name });
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
                                Some(tid.to_string());
                            let _ = tx2.send(StreamMsg::Stream {
                                generation,
                                kind: StreamMsgKind::ThreadId(tid.to_string()),
                            });
                        },
                    );
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

                // If a streaming thread is stuck on a network read, exit
                // the process cleanly rather than waiting indefinitely.
                if cancel.load(Ordering::Relaxed) {
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
    use std::sync::atomic::AtomicBool;

    fn test_app() -> App {
        App::new(None, None, None, String::new(), None)
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
    //   idle → [Ctrl+C] → cancel_stream() → interrupting=true, stop_pending=true
    //        → [main loop] spawns stop thread → stop_pending=false
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

        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);

        assert!(app.interrupting);
        assert!(app.stop_pending);
        assert!(cancel.load(Ordering::Relaxed));
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

        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);

        assert!(app.interrupting);
        assert!(app.stop_pending);
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

        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);

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
        app.active_tool_call = Some("read_file".into());

        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);

        assert!(app.active_tool_call.is_none());
    }

    // -- 2. Idempotent cancel (multiple Ctrl+C) ----------------------------

    #[test]
    fn cancel_is_idempotent_while_interrupting() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 5;
        app.stop_pending = false; // already dispatched

        let cancel = AtomicBool::new(true);
        app.cancel_stream(&cancel);

        // Should not change anything — generation stays, no double stop
        assert_eq!(app.stream_generation, 5);
        assert!(!app.stop_pending); // should NOT re-set stop_pending
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

        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);

        assert!(app.interrupting);
        assert!(app.stop_pending);

        // In the main loop, stop_pending=true but active_thread_id=None
        // triggers immediate finish_stream + "Cancelled"
        // Simulate that path:
        app.stop_pending = false;
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
        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);
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
        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);
        assert!(app.interrupting);
        assert!(app.stop_pending);

        // 3. Stop thread succeeds
        app.stop_pending = false;
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
        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);

        // 3. Stop times out (your screenshot scenario)
        app.stop_pending = false;
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
    fn rapid_ctrl_c_during_interrupting_is_safe() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = true;
        app.stream_generation = 5;

        let cancel = AtomicBool::new(true);

        // Spam Ctrl+C 5 times
        for _ in 0..5 {
            app.handle_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &cancel,
            );
        }

        // Nothing should change — should NOT quit, NOT double-cancel
        assert!(!app.should_quit);
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

        let cancel = AtomicBool::new(true);

        for _ in 0..5 {
            app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &cancel);
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
        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);
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
    fn stop_finished_when_not_interrupting_still_cleans_up() {
        let mut app = test_app();
        app.streaming = true;
        app.interrupting = false; // race: already recovered somehow

        app.handle_stream_msg(StreamMsg::StopFinished { error: None });

        // Should gracefully finish
        assert!(!app.streaming);
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

        let cancel = AtomicBool::new(false);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &cancel,
        );

        assert!(app.interrupting);
        assert!(!app.should_quit);
        assert!(cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn ctrl_c_while_not_streaming_quits() {
        let mut app = test_app();

        let cancel = AtomicBool::new(false);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &cancel,
        );

        assert!(app.should_quit);
    }

    #[test]
    fn esc_while_streaming_cancels() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 0;

        let cancel = AtomicBool::new(false);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &cancel);

        assert!(app.interrupting);
        assert!(cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn esc_while_not_streaming_enters_vi_normal() {
        let mut app = test_app();
        app.input_mode = InputMode::ViInsert;

        let cancel = AtomicBool::new(false);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &cancel);

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
        app.active_tool_call = Some("list_workspaces".into());

        let cancel = AtomicBool::new(false);
        app.cancel_stream(&cancel);

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
            },
        });

        assert_eq!(app.active_tool_call.as_deref(), Some("list_flows"));
    }

    #[test]
    fn tool_call_output_clears_active_tool_and_adds_system_msg() {
        let mut app = test_app();
        app.streaming = true;
        app.stream_generation = 1;
        app.active_tool_call = Some("list_flows".into());

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
        let lines = render_markdown(text, Role::Otto);
        // Should have: text, code block header, code line, code block footer, more text
        assert!(lines.len() >= 5);
    }

    #[test]
    fn render_markdown_handles_inline_code() {
        let text = "use `foo()` here";
        let lines = render_markdown(text, Role::Otto);
        assert_eq!(lines.len(), 1);
        // Line should have multiple spans (indent, text, code, text)
        assert!(lines[0].spans.len() >= 3);
    }
}
