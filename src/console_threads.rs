use chrono::{DateTime, Utc};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::mpsc::{channel, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    console_logger::{ConsoleLogger, LogLine},
    ProcessorConfig,
};
use rustyline::{error::ReadlineError, Editor};
use serialport::{Error, SerialPort};

enum Msg {
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

enum WriteBuf {
    Buf(Box<[u8]>),
    Exit,
}

enum NextLineBuf {
    Buf {
        instant: DateTime<Utc>,
        line: String,
    },
    Empty,
}

struct ProcessorWriter {
    editor: Editor<()>,
    history_path: String,
    processor_name: String,
    write_sender: Sender<WriteBuf>,
}

struct ProcessorCache {
    processor_name: String,
    next_line_buf: NextLineBuf,
    write_sender: Sender<WriteBuf>,
    join_handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send>>>,
}

fn serial_port_task(
    mut serial_port: Box<dyn SerialPort>,
    processor_idx: usize,
    processor_name: String,
    msg_sender: Sender<Msg>,
    write_receiver: Receiver<WriteBuf>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    println!("> [serial_port_task] start {:?}", processor_name);
    let mut readbuf = [0u8; 0x100];
    loop {
        let instant = Utc::now();
        loop {
            match serial_port.read(&mut readbuf) {
                Err(e) => {
                    let _ = e; // always fails in first few seconds
                    break;
                }
                Ok(0) => break,
                Ok(count) => {
                    let _ = msg_sender.send(Msg::Read {
                        processor_idx,
                        instant,
                        bytes: Box::from(&readbuf[..count]),
                    });
                }
            }
        }
        loop {
            match write_receiver.try_recv() {
                Ok(WriteBuf::Exit) => {
                    return Ok(println!("> [serial_port_task] {:?} end", processor_name));
                }
                Ok(WriteBuf::Buf(msg)) => {
                    let _ = serial_port.write(&msg);
                }
                _ => break,
            }
        }
    }
}

pub const DATE_TIME_FMT: &'static str = "%y-%m-%d %H:%M:%S%.3f";

fn read_msg(
    processor_cache: &mut Vec<ProcessorCache>,
    line_sender: &Sender<LogLine>,
    processor_idx: usize,
    instant: DateTime<Utc>,
    bytes: Box<[u8]>,
) {
    let ref mut p = processor_cache[processor_idx];
    let print_line = |instant: &DateTime<Utc>, line: &str| {
        println!(
            "{} r {} {}",
            p.processor_name,
            instant.format(DATE_TIME_FMT).to_string(),
            line
        );
        let _ = line_sender.send(LogLine::Line {
            processor_name: p.processor_name.clone(),
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
        if let NextLineBuf::Buf { instant: _, line } = &mut p.next_line_buf {
            line.push_str(first)
        } else {
            p.next_line_buf = NextLineBuf::Buf {
                instant,
                line: first.to_string(),
            }
        }
    }
    if let Some(last) = q.pop_back() {
        if let NextLineBuf::Buf { instant, line } = &p.next_line_buf {
            print_line(instant, line);
        }
        p.next_line_buf = NextLineBuf::Buf {
            instant,
            line: last.to_string(),
        };
    }
    while let Some(next) = q.pop_front() {
        print_line(&instant, next);
    }
}

fn byte_process_task(
    msg_receiver: Receiver<Msg>,
    line_sender: Sender<LogLine>,
    mut processor_cache: Vec<ProcessorCache>,
) -> Result<(), Error> {
    for msg in &msg_receiver {
        match msg {
            Msg::Exit => break,
            Msg::Read {
                processor_idx,
                instant,
                bytes,
            } => {
                read_msg(
                    &mut processor_cache,
                    &line_sender,
                    processor_idx,
                    instant,
                    bytes,
                );
            }
            Msg::Write {
                instant,
                bytes,
                processor_idx,
            } => {
                let ref mut p = processor_cache[processor_idx];
                println!(
                    "{} w {} {}",
                    p.processor_name,
                    instant.format(DATE_TIME_FMT).to_string(),
                    String::from_utf8_lossy(&bytes),
                );
                let _ = line_sender.send(LogLine::Line {
                    processor_name: p.processor_name.clone(),
                    read_write: "w",
                    instant,
                    line: String::from_utf8_lossy(&bytes).to_string(),
                });
            }
        }
    }

    for p in &processor_cache {
        let _ = p.write_sender.send(WriteBuf::Exit);
    }
    for p in processor_cache {
        let _ = p.join_handle.join();
    }

    Ok(println!("> [byte_process_task] end"))
}

impl ProcessorConfig {
    pub fn main_task(self) -> Result<(), Box<dyn std::error::Error>> {
        if self.processors.len() == 0 {
            return Ok(println!("> [main_task] no processors"));
        }

        let (msg_sender, msg_receiver) = channel();
        let mut processor_cache = vec![];
        let mut writers = vec![];

        let project_name = String::clone(&self.project_name);
        let logger = ConsoleLogger::new(project_name)?;

        let mut processor_idx = 0;
        for p in &self.processors {
            let duration = Duration::from_millis(10);
            let path = p.port_name.clone();
            let baud_rate = p.baudrate.get();
            let builder = serialport::new(path, baud_rate).timeout(duration);
            let serial_port = builder.open()?;
            let processor_name = p.processor_name.borrow().clone();
            let msg_sender = msg_sender.clone();
            let (write_sender, write_receiver) = channel();

            let mut processor_writer = ProcessorWriter {
                history_path: format!("{} history.txt", processor_name),
                editor: Editor::new(),
                write_sender: write_sender.clone(),
                processor_name: processor_name.clone(),
            };

            if processor_writer
                .editor
                .load_history(&processor_writer.history_path)
                .is_err()
            {
                println!(
                    "> [main_task] no previous history at {:?}",
                    processor_writer.history_path
                );
            }
            writers.push(processor_writer);

            processor_cache.push(ProcessorCache {
                processor_name: processor_name.clone(),
                next_line_buf: NextLineBuf::Empty,
                write_sender,
                join_handle: thread::spawn(move || {
                    serial_port_task(
                        serial_port,
                        processor_idx,
                        processor_name,
                        msg_sender,
                        write_receiver,
                    )
                }),
            });

            processor_idx += 1;
        }

        let line_sender = logger.sender();
        let byte_process_thread =
            thread::spawn(move || byte_process_task(msg_receiver, line_sender, processor_cache));

        let mut proc_switcher_editor = Editor::<()>::new();
        let proc_switcher_history_path = "history.txt";
        if proc_switcher_editor
            .load_history(proc_switcher_history_path)
            .is_err()
        {
            println!(
                "> [main_task] no previous history at {:?}",
                proc_switcher_history_path
            );
        }
        processor_idx = 0;
        loop {
            let ref mut writer = writers[processor_idx];
            match writer.editor.readline("") {
                Ok(mut line) => {
                    writer.editor.add_history_entry(line.as_str());
                    line.push_str("\r");
                    let _ = writer
                        .write_sender
                        .send(WriteBuf::Buf(line.as_bytes().into()));
                    let _ = msg_sender.send(Msg::Write {
                        bytes: line.as_bytes().into(),
                        instant: Utc::now(),
                        processor_idx,
                    });
                }
                Err(ReadlineError::Interrupted) => {
                    break;
                }
                Err(ReadlineError::Eof) => {
                    processor_idx += 1;
                    processor_idx %= writers.len();
                    let ref w = writers[processor_idx];
                    println!("> [main_task] switching to {:?}", w.processor_name);
                }
                Err(err) => {
                    println!("> [main_task] error: {:#?}", err);
                    break;
                }
            }
        }

        println!("> [main_task] send exit commands");

        let _ = logger.exit();
        let _ = msg_sender.send(Msg::Exit);
        for mut e in writers {
            let _ = e.editor.save_history(&e.history_path);
        }
        let _ = proc_switcher_editor.save_history(proc_switcher_history_path);

        let _ = byte_process_thread.join();
        let _ = logger.join();

        Ok(println!("> [main_task] end"))
    }
}
