use crate::{file_logger_thread::LogLine, ProcessorInfo};
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
    line_sender: Sender<LogLine>,
}

impl ProcessorByteCache {
    pub fn new(processor_info: &ProcessorInfo, line_sender: Sender<LogLine>) -> Self {
        Self {
            processor_name: processor_info.processor_name.clone(),
            next_line_buf: NextLineBuf::Empty,
            line_sender,
        }
    }
}

impl ProcessorByteCache {
    pub fn write_msg(&mut self, instant: DateTime<Utc>, bytes: Box<[u8]>) {
        println!(
            "{} w {} {}",
            self.processor_name,
            instant.format(DATE_TIME_FMT).to_string(),
            String::from_utf8_lossy(&bytes),
        );
        let _ = self.line_sender.send(LogLine::Line {
            processor_name: self.processor_name.clone(),
            read_write: "w",
            instant,
            line: String::from_utf8_lossy(&bytes).to_string(),
        });
    }
}

impl ProcessorByteCache {
    pub fn read_msg(&mut self, instant: DateTime<Utc>, bytes: Box<[u8]>) {
        let print_line = |instant: &DateTime<Utc>, line: &str| {
            println!(
                "{} r {} {}",
                self.processor_name,
                instant.format(DATE_TIME_FMT).to_string(),
                line
            );
            let _ = self.line_sender.send(LogLine::Line {
                processor_name: self.processor_name.clone(),
                read_write: "r",
                instant: instant.clone(),
                line: line.to_string(),
            });
        };
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
                print_line(instant, line);
            }
            self.next_line_buf = NextLineBuf::Buf {
                instant,
                line: last.to_string(),
            };
        }
        while let Some(next) = q.pop_front() {
            print_line(&instant, next);
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
}

impl ByteProcessThread {
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
