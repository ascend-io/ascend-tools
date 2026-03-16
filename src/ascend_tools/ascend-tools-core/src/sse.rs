//! Minimal SSE (Server-Sent Events) line parser.
//!
//! Reads from any `BufRead` and yields `SseEvent` structs.

use std::io::BufRead;

use crate::error::{Error, Result};

/// Safety limit: max data lines per SSE event before erroring.
const MAX_DATA_LINES: usize = 100_000;

/// A single SSE event.
#[derive(Debug, Clone)]
pub(crate) struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
}

/// Iterator over SSE events from a buffered reader.
pub(crate) struct SseReader<R> {
    reader: R,
    /// Accumulated event type for the current event block.
    current_event_type: Option<String>,
    /// Accumulated data lines for the current event block.
    current_data: Vec<String>,
    done: bool,
}

impl<R: BufRead> SseReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            current_event_type: None,
            current_data: Vec::new(),
            done: false,
        }
    }
}

impl<R: BufRead> Iterator for SseReader<R> {
    type Item = Result<SseEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF — dispatch any pending event
                    self.done = true;
                    if !self.current_data.is_empty() {
                        let event = SseEvent {
                            event_type: self.current_event_type.take(),
                            data: self.current_data.drain(..).collect::<Vec<_>>().join("\n"),
                        };
                        return Some(Ok(event));
                    }
                    return None;
                }
                Ok(_) => {
                    let line = line.trim_end_matches('\n').trim_end_matches('\r');

                    if line.is_empty() {
                        // Blank line dispatches the event
                        if !self.current_data.is_empty() {
                            let event = SseEvent {
                                event_type: self.current_event_type.take(),
                                data: self.current_data.drain(..).collect::<Vec<_>>().join("\n"),
                            };
                            return Some(Ok(event));
                        }
                        // No data accumulated, just reset
                        self.current_event_type = None;
                        continue;
                    }

                    if let Some(value) = line.strip_prefix("event:") {
                        self.current_event_type =
                            Some(value.strip_prefix(' ').unwrap_or(value).to_string());
                    } else if let Some(value) = line.strip_prefix("data:") {
                        self.current_data
                            .push(value.strip_prefix(' ').unwrap_or(value).to_string());
                        if self.current_data.len() > MAX_DATA_LINES {
                            self.done = true;
                            return Some(Err(Error::SseParseError {
                                context: format!("SSE event exceeded {MAX_DATA_LINES} data lines"),
                            }));
                        }
                    }
                    // Ignore id:, retry:, and comment lines (starting with :)
                }
                Err(e) => {
                    self.done = true;
                    return Some(Err(Error::SseParseError {
                        context: format!("failed to read SSE line: {e}"),
                    }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_basic_events() {
        let input = "event: message\ndata: hello\n\nevent: done\ndata: world\n\n";
        let reader = SseReader::new(Cursor::new(input));
        let events: Vec<SseEvent> = reader.map(|r| r.unwrap()).collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type.as_deref(), Some("message"));
        assert_eq!(events[0].data, "hello");
        assert_eq!(events[1].event_type.as_deref(), Some("done"));
        assert_eq!(events[1].data, "world");
    }

    #[test]
    fn parse_multiline_data() {
        let input = "data: line1\ndata: line2\n\n";
        let reader = SseReader::new(Cursor::new(input));
        let events: Vec<SseEvent> = reader.map(|r| r.unwrap()).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn parse_eof_dispatch() {
        let input = "data: final";
        let reader = SseReader::new(Cursor::new(input));
        let events: Vec<SseEvent> = reader.map(|r| r.unwrap()).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "final");
    }

    #[test]
    fn skip_empty_blocks() {
        let input = "\n\ndata: hello\n\n\n\n";
        let reader = SseReader::new(Cursor::new(input));
        let events: Vec<SseEvent> = reader.map(|r| r.unwrap()).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }
}
