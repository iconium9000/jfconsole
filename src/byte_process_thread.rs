use crate::{file_logger_thread::LogLine, main_thread::ProcessorInfo};
use chrono::{DateTime, Utc};
use std::{
    collections::VecDeque,
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
};

pub enum NextLineBuf {
    Buf {
        instant: DateTime<Utc>,
        line: String,
    },
    Empty,
}

pub enum Msg {
    Read {
        processor_idx: usize,
        instant: DateTime<Utc>,
        bytes: Box<[u8]>,
    },
    Write {
        processor_idx: usize,
        bytes: Box<[u8]>,
        instant: DateTime<Utc>,
    },
    Exit,
}

pub const DATE_TIME_FMT: &'static str = "%y-%m-%d %H:%M:%S%.3f";

pub struct ProcessorByteCache {
    processor_name: String,
    next_line_buf: NextLineBuf,
    logline_sender: Sender<LogLine>,
}

impl ProcessorByteCache {
    pub fn new(processor_info: &ProcessorInfo, logline_sender: Sender<LogLine>) -> Self {
        Self {
            processor_name: processor_info.processor_name.clone(),
            next_line_buf: NextLineBuf::Empty,
            logline_sender,
        }
    }
}

impl ProcessorByteCache {
    fn send_msg(&self, read_write: &'static str, instant: &DateTime<Utc>, line: &str) {
        let processor_name = self.processor_name.clone();
        println!(
            "{} {} {} {}",
            self.processor_name,
            read_write,
            instant.format(DATE_TIME_FMT).to_string(),
            line,
        );
        let _ = self.logline_sender.send(LogLine::Line {
            processor_name,
            read_write,
            instant: instant.clone(),
            line: line.to_string(),
        });
    }

    fn write_msg(&self, instant: DateTime<Utc>, bytes: Box<[u8]>) {
        let line = String::from_utf8_lossy(&bytes);
        self.send_msg("w", &instant, &line);
    }

    fn read_msg(&mut self, instant: DateTime<Utc>, bytes: Box<[u8]>) {
        let payload = String::from_utf8_lossy(&bytes)
            .replace("\r\n", "\n")
            .replace("\n\r", "\n")
            .replace("\r", "\n");
        let mut q: VecDeque<&str> = payload.split("\n").collect();
        if let Some(first) = q.pop_front() {
            if let NextLineBuf::Buf { instant: _, line } = &mut self.next_line_buf {
                line.push_str(first)
            } else {
                self.next_line_buf = NextLineBuf::Buf {
                    instant,
                    line: first.to_string(),
                }
            }
        }
        if let Some(last) = q.pop_back() {
            if let NextLineBuf::Buf { instant, line } = &self.next_line_buf {
                self.send_msg("r", instant, line);
            }
            self.next_line_buf = NextLineBuf::Buf {
                instant,
                line: last.to_string(),
            };
        }
        while let Some(next) = q.pop_front() {
            self.send_msg("r", &instant, next);
        }
    }
}

pub struct ByteProcessThread {
    join_handle: JoinHandle<()>,
    msg_sender: Sender<Msg>,
}

impl ByteProcessThread {
    pub fn spawn(
        msg_sender: &Sender<Msg>,
        msg_receiver: Receiver<Msg>,
        processor_byte_caches: Vec<ProcessorByteCache>,
    ) -> Self {
        ByteProcessThread {
            msg_sender: msg_sender.clone(),
            join_handle: thread::spawn(move || {
                byte_process_task(msg_receiver, processor_byte_caches)
            }),
        }
    }
    pub fn join(self) {
        let _ = self.msg_sender.send(Msg::Exit);
        let _ = self.join_handle.join();
    }
}

pub fn byte_process_task(
    msg_receiver: Receiver<Msg>,
    mut processor_byte_caches: Vec<ProcessorByteCache>,
) {
    for msg in &msg_receiver {
        match msg {
            Msg::Exit => break,
            Msg::Read {
                processor_idx,
                instant,
                bytes,
            } => processor_byte_caches[processor_idx].read_msg(instant, bytes),
            Msg::Write {
                instant,
                bytes,
                processor_idx,
            } => processor_byte_caches[processor_idx].write_msg(instant, bytes),
        }
    }
    println!("> [byte_process_task] end")
}
