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
use std::sync::mpsc;
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
use ascend_tools::models::{OttoChatRequest, OttoModel, StreamEvent};
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

const USER_COLOR: Color = Color::Rgb(80, 120, 200); // dark blue
const OTTO_COLOR: Color = Color::Rgb(232, 67, 67); // ascend red
const SYSTEM_COLOR: Color = Color::Rgb(160, 120, 200); // purple
const VI_NORMAL_COLOR: Color = Color::Rgb(255, 140, 80); // orange
const CODE_COLOR: Color = Color::Rgb(255, 140, 80); // orange (matches vi normal)
const DIM_COLOR: Color = Color::Rgb(100, 100, 100);
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
    ToolCallStart { name: String },
    ToolCallOutput { name: String, output: String },
    Done,
    Error(String),
}

#[derive(Clone, Copy, PartialEq)]
enum InputMode {
    Emacs,
    ViInsert,
    ViNormal,
}

#[derive(Clone, Copy, PartialEq)]
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
        }
    }

    // -- Input helpers ------------------------------------------------------

    fn input_line_count(&self) -> u16 {
        let count = 1 + self.input.iter().filter(|c| **c == '\n').count();
        (count as u16).min(MAX_INPUT_LINES)
    }

    fn handle_paste(&mut self, text: &str) {
        if self.input_mode == InputMode::ViNormal {
            self.input_mode = InputMode::ViInsert;
        }
        for (i, ch) in text.chars().enumerate() {
            self.input.insert(self.cursor + i, ch);
        }
        self.cursor += text.chars().count();
        self.completion_index = None;
    }

    // -- Key handling -------------------------------------------------------

    fn handle_key(&mut self, key: KeyEvent, cancel: &AtomicBool) {
        // Ctrl+C: cancel stream or quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.streaming {
                cancel.store(true, Ordering::Relaxed);
                self.finish_stream();
                self.push_system("Cancelled");
                self.stream_start = None;
            } else {
                self.should_quit = true;
            }
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
                    for (i, ch) in self.yank_register.chars().enumerate() {
                        self.input.insert(pos + i, ch);
                    }
                    self.cursor = pos + self.yank_register.len() - 1;
                }
            }
            // Paste before cursor
            (KeyModifiers::SHIFT, KeyCode::Char('P')) => {
                if !self.yank_register.is_empty() {
                    for (i, ch) in self.yank_register.chars().enumerate() {
                        self.input.insert(self.cursor + i, ch);
                    }
                    self.cursor += self.yank_register.len().saturating_sub(1);
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
            StreamMsgKind::Done => {
                self.finish_stream();
                // Bell if response took >3s
                if self
                    .stream_start
                    .is_some_and(|s| s.elapsed() > Duration::from_secs(3))
                {
                    let _ = crossterm::execute!(std::io::stderr(), crossterm::style::Print("\x07"));
                }
                self.stream_start = None;
            }
            StreamMsgKind::Error(err) => {
                self.finish_stream();
                self.push_system(format!("Error: {err}"));
                self.stream_start = None;
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

    fn finish_stream(&mut self) {
        self.flush_stream_text();
        self.streaming = false;
        self.active_tool_call = None;
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

        let input_height = self.input_line_count();
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
                let label = if let Some(tool) = &self.active_tool_call {
                    format!("  {} \u{2699} {tool}...", SPINNER[self.spinner_frame])
                } else {
                    format!("  {} Ascending...", SPINNER[self.spinner_frame])
                };
                lines.push(Line::from(Span::styled(
                    label,
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
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(clamped_scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .style(Style::default().fg(DIM_COLOR)),
                area,
                &mut scrollbar_state,
            );
        }
    }

    fn render_splash(&self, frame: &mut Frame, area: Rect) {
        let splash_height = SPLASH.len() as u16;
        let y_offset = area.height.saturating_sub(splash_height) / 2;

        let lines: Vec<Line<'_>> = SPLASH
            .iter()
            .map(|&line| {
                let display_width = line.chars().count();
                let pad = (area.width as usize).saturating_sub(display_width) / 2;
                let padded = format!("{:>width$}{}", "", line, width = pad);
                if line.contains("/help") {
                    Line::from(Span::styled(padded, Style::default().fg(DIM_COLOR)))
                } else {
                    Line::from(Span::styled(padded, Style::default().fg(OTTO_COLOR)))
                }
            })
            .collect();

        let clamped_height = splash_height.min(area.height);
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
        let prompt_len = 3;

        let prompt_color = match self.input_mode {
            InputMode::ViNormal => VI_NORMAL_COLOR,
            _ if self.streaming => DIM_COLOR,
            _ => OTTO_COLOR,
        };

        // Multi-line: split input on \n
        let input_str: String = self.input.iter().collect();
        let input_lines: Vec<&str> = input_str.split('\n').collect();

        // Find which line the cursor is on and the column within that line
        let mut cursor_line = 0;
        let mut cursor_col = 0;
        let mut chars_so_far = 0;
        for (i, line) in input_lines.iter().enumerate() {
            let line_len = line.chars().count();
            let is_last = i == input_lines.len() - 1;
            if self.cursor <= chars_so_far + line_len || is_last {
                cursor_line = i;
                cursor_col = self.cursor.saturating_sub(chars_so_far);
                break;
            }
            chars_so_far += line_len + 1; // +1 for \n
        }

        let mut render_lines: Vec<Line<'_>> = Vec::new();
        for (i, line) in input_lines.iter().enumerate() {
            let p = if i == 0 { prompt } else { "   " };
            let p_style = if i == 0 {
                Style::default().fg(prompt_color)
            } else {
                Style::default().fg(DIM_COLOR)
            };
            render_lines.push(Line::from(vec![
                Span::styled(p, p_style),
                Span::raw((*line).to_string()),
            ]));
        }

        frame.render_widget(Paragraph::new(render_lines), area);

        if !self.streaming {
            let cx = area.x + prompt_len as u16 + cursor_col as u16;
            let cy = area.y + cursor_line as u16;
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
        Some(OttoModel::Name(name)) => {
            let friendly = providers
                .iter()
                .flat_map(|p| {
                    p.models
                        .iter()
                        .filter(|m| m.id == *name)
                        .map(|m| m.name.clone())
                })
                .next()
                .unwrap_or_else(|| name.clone());
            (None, friendly)
        }
        Some(OttoModel::ProviderModel {
            provider_id,
            model_id,
        }) => {
            let p = providers.iter().find(|p| p.id == *provider_id);
            let provider_name = p
                .map(|p| p.name.clone())
                .unwrap_or_else(|| provider_id.clone());
            let model_name = p
                .and_then(|p| p.models.iter().find(|m| m.id == *model_id))
                .map(|m| m.name.clone())
                .unwrap_or_else(|| model_id.clone());
            (Some(provider_name), model_name)
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
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableBracketedPaste,
            SetCursorStyle::DefaultUserShape
        );
        original_hook(info);
    }));

    let (stream_tx, stream_rx) = mpsc::channel::<StreamMsg>();
    let cancel = AtomicBool::new(false);
    let gen_counter = AtomicU64::new(0);

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

            // Launch streaming request if pending
            if let Some(request) = app.take_pending_request() {
                let generation = gen_counter.fetch_add(1, Ordering::Relaxed) + 1;
                app.stream_generation = generation;
                cancel.store(false, Ordering::Relaxed);
                let tx = stream_tx.clone();
                let cancel_ref = &cancel;
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
                            let _ = tx2.send(StreamMsg::Stream {
                                generation,
                                kind: StreamMsgKind::ThreadId(tid.to_string()),
                            });
                        },
                    );
                    match result {
                        Ok(_) => send(StreamMsgKind::Done),
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
                let _ = std::panic::take_hook();

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
    let _ = std::panic::take_hook();

    result
}
