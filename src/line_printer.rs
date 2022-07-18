use std::sync::mpsc::Sender;

use crate::main_thread::DATE_TIME_FMT;
use chrono::Utc;

pub struct LinePrinter {
    timestamp: String,
    buffer: String,
    prefix: String,
    complete: char,
    last_char: Option<char>,
    line_width: usize,
    line_sender: Sender<String>,
}

fn timestamp_now() -> String {
    Utc::now().format(DATE_TIME_FMT).to_string()
}

macro_rules! send_split {
    ($self: ident, $buffer: expr) => {
        let fmt = format!(
            "{} {} {} {}",
            $self.prefix, $self.timestamp, $self.complete, $buffer
        );
        println!("{}", fmt);
        let _ = $self.line_sender.send(fmt);
    };
}

impl LinePrinter {
    pub fn new(prefix: String, line_width: usize, line_sender: Sender<String>) -> Self {
        Self {
            prefix,
            timestamp: timestamp_now(),
            buffer: String::new(),
            complete: '>',
            line_width,
            last_char: None,
            line_sender,
        }
    }

    pub fn push_str(&mut self, lines: &str) {
        let mut old_timestamp = true;
        for ch in lines.chars() {
            if let '\r' | '\n' = ch {
                let last_char = self.last_char;
                self.last_char = Some(ch);
                if let Some(last_char) = last_char {
                    if last_char != ch {
                        continue;
                    }
                }
                send_split!(self, &self.buffer);
                self.complete = '|';
                self.buffer.clear();
            } else {
                if self.buffer.len() >= self.line_width {
                    let mut last_space_idx = None;
                    for (i, ch) in self.buffer.char_indices() {
                        if ch == ' ' {
                            last_space_idx = Some(i);
                        }
                    }
                    match last_space_idx {
                        Some(0) => {}
                        Some(last_space_idx) => {
                            send_split!(self, &self.buffer[..last_space_idx]);
                            self.buffer = self.buffer[last_space_idx..].to_string();
                            self.complete = ' ';
                            self.last_char = None;
                        }
                        _ => (),
                    }
                }
                if self.buffer.is_empty() && old_timestamp {
                    old_timestamp = false;
                    self.timestamp = timestamp_now();
                }
                self.buffer.push(ch);
                self.last_char = None;
            }
        }
    }
    pub fn push_bytes(&mut self, buffer: &[u8]) {
        self.push_str(&String::from_utf8_lossy(buffer))
    }
}
