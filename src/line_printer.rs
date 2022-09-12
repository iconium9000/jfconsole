use std::sync::mpsc::Sender;

use crate::main_thread::DATE_TIME_FMT;
use chrono::Utc;

pub struct LinePrinter {
    console_timestamp: String,
    log_timestamp: String,
    buffer: String,
    prefix: String,
    complete: char,
    last_char: Option<char>,
    line_width: usize,
    line_sender: Sender<String>,
}

impl LinePrinter {
    fn timestamp_now(&mut self) {
        let now = Utc::now();
        self.log_timestamp = now.format(DATE_TIME_FMT).to_string();
        self.console_timestamp = now.format("%M:%S%.3f").to_string();
    }
}

macro_rules! send_split {
    ($self: ident, $buffer: expr) => {
        $self.timestamp_now();
        println!(
            "{} {} {} {}",
            $self.prefix, $self.console_timestamp, $self.complete, $buffer
        );
        let _ = $self.line_sender.send(format!(
            "{} {} {} {}",
            $self.prefix, $self.log_timestamp, $self.complete, $buffer
        ));
    };
}

impl LinePrinter {
    pub fn new(prefix: String, line_width: usize, line_sender: Sender<String>) -> Self {
        let mut line_printer = Self {
            prefix,
            log_timestamp: String::new(),
            console_timestamp: String::new(),
            buffer: String::new(),
            complete: '|',
            line_width,
            last_char: None,
            line_sender,
        };
        line_printer.timestamp_now();
        line_printer
    }

    pub fn push_str(&mut self, lines: &str) {
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
                self.buffer.push(ch);
                self.last_char = None;
            }
        }
    }
    pub fn push_bytes(&mut self, buffer: &[u8]) {
        self.push_str(&String::from_utf8_lossy(buffer))
    }
}
