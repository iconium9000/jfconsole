use crate::utils::ring_buf_queue::RingBufQProducer;
use chrono::Utc;
use std::sync::mpsc::Sender;

pub const DATE_TIME_FMT: &'static str = "%y-%m-%d %H:%M:%S%.3f";

pub struct LinePrinter {
    console_timestamp: String,
    log_timestamp: String,
    buffer: String,
    prefix: String,
    complete: char,
    last_char: Option<char>,
    line_width: usize,
    line_sender: Sender<String>,
    write_producer: Option<RingBufQProducer<u8>>,
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
    pub fn new(
        prefix: String,
        line_width: usize,
        line_sender: Sender<String>,
        write_producer: Option<RingBufQProducer<u8>>,
    ) -> Self {
        let mut line_printer = Self {
            prefix,
            log_timestamp: String::new(),
            console_timestamp: String::new(),
            buffer: String::new(),
            complete: '|',
            line_width,
            last_char: None,
            line_sender,
            write_producer,
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

        let wp: &mut Option<_> = &mut self.write_producer;
        if let Some(wp) = wp {
            if lines.contains("IPC Comm Failure") {
                println!("> [line_printer] ipc comm failure");
                wp.push("t ipcwdg\r".as_bytes())
            }
        }
    }
    pub fn push_bytes(&mut self, buffer: &[u8]) {
        self.push_str(&String::from_utf8_lossy(buffer))
    }
}
