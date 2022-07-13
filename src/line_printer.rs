use std::sync::mpsc::Sender;

use crate::main_thread::DATE_TIME_FMT;
use chrono::Utc;

pub struct LinePrinter {
    timestamp: String,
    buffer: String,
    prefix: String,
    last_char: Option<char>,
    line_width: usize,
    line_sender: Sender<String>,
}

fn timestamp_now() -> String {
    Utc::now().format(DATE_TIME_FMT).to_string()
}

impl LinePrinter {
    pub fn new(prefix: String, line_width: usize, line_sender: Sender<String>) -> Self {
        Self {
            prefix,
            timestamp: timestamp_now(),
            buffer: String::new(),
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
                let fmt = format!("{} {} | {}", self.prefix, self.timestamp, self.buffer);
                println!("{}", fmt);
                let _ = self.line_sender.send(fmt);
                self.buffer.clear();
            } else {
                if self.buffer.len() >= self.line_width {
                    let fmt = format!("{} {}   {}", self.prefix, self.timestamp, self.buffer);
                    println!("{}", fmt);
                    let _ = self.line_sender.send(fmt);
                    self.buffer.clear();
                }
                if self.buffer.is_empty() {
                    if old_timestamp {
                        old_timestamp = false;
                        self.timestamp = timestamp_now();
                    }
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
