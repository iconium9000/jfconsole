use chrono::{DateTime, Utc};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::mpsc::{channel, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::ProcessorConfig;
use rustyline::{error::ReadlineError, Editor};
use serialport::{Error, SerialPort};

enum Msg {
    SwitchToProcessor {
        processor_idx: usize,
    },
    Read {
        processor_idx: usize,
        instant: DateTime<Utc>,
        bytes: Box<[u8]>,
    },
    Write {
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
    Buf { instant: DateTime<Utc>, pre: String },
    Empty,
}

struct ProcessorWriter {
    editor: Editor<()>,
    history_path: String,
    processor_name: String,
    processor_idx: usize,
}

struct ProcessorCache {
    processor_name: String,
    next_line_buf: NextLineBuf,
    write_sender: Sender<WriteBuf>,
    join_handle: JoinHandle<Result<(), Error>>,
}

fn serial_port_task(
    mut serial_port: Box<dyn SerialPort>,
    processor_idx: usize,
    msg_sender: Sender<Msg>,
    write_receiver: Receiver<WriteBuf>,
) -> Result<(), Error> {
    println!("start serial_port_task");
    let mut readbuf = [0u8; 0x100];
    loop {

        if match serial_port.read(&mut readbuf) {
            Err(_) => true,
            Ok(0) => true,
            Ok(count) => {
                let _ = msg_sender.send(Msg::Read {
                    processor_idx,
                    instant: Utc::now(),
                    bytes: Box::from(&readbuf[..count]),
                });
                false
            }
        } {
            let _rcved = match write_receiver.try_recv() {
                Ok(WriteBuf::Exit) => {
                    println!("end serial_port_task for {}", processor_idx);
                    break Ok(());
                }
                Ok(WriteBuf::Buf(msg)) => {
                    let _ = serial_port.write(&msg);
                    true
                }
                _ => false,
            };
    
            // thread::sleep(Duration::from_millis(1));
        }
    }
}

fn byte_process_task(
    msg_receiver: Receiver<Msg>,
    mut processor_cache: Vec<ProcessorCache>,
) -> Result<(), Error> {
    if processor_cache.is_empty() {
        let kind = serialport::ErrorKind::Unknown;
        let description = "processor_cache is_empty";
        return Err(Error::new(kind, description));
    }

    let mut processor_idx = 0;
    let fmt = "%y-%m-%d %H:%M:%S%.3f";
    for msg in &msg_receiver {
        match msg {
            Msg::Exit => break,
            Msg::SwitchToProcessor {
                processor_idx: new_idx,
            } => {
                let ref mut p = processor_cache[processor_idx];
                match &p.next_line_buf {
                    NextLineBuf::Buf { instant, pre } => {
                        println!(
                            "{} r {} {}",
                            p.processor_name,
                            instant.format(fmt).to_string(),
                            pre
                        );
                        p.next_line_buf = NextLineBuf::Empty;
                    }
                    NextLineBuf::Empty => {}
                }
                processor_idx = new_idx;
            }
            Msg::Read {
                processor_idx,
                instant,
                bytes,
            } => {
                let ref mut p = processor_cache[processor_idx];
                let cow = String::from_utf8_lossy(&bytes)
                    .replace("\r\n", "\n")
                    .replace("\n\r", "\n")
                    .replace("\r", "\n");
                let mut q: VecDeque<&str> = cow.split("\n").collect();
                if let Some(first) = q.pop_front() {
                    match &mut p.next_line_buf {
                        NextLineBuf::Buf { instant: _, pre } => {
                            pre.push_str(first);
                        }
                        NextLineBuf::Empty => {
                            p.next_line_buf = NextLineBuf::Buf {
                                instant,
                                pre: first.into(),
                            };
                        }
                    };
                }
                let next_line_buf = match q.pop_back() {
                    Some(last) => NextLineBuf::Buf {
                        instant,
                        pre: last.into(),
                    },
                    None => NextLineBuf::Empty,
                };
                if let NextLineBuf::Buf { instant, pre } = &p.next_line_buf {
                    println!(
                        "{} r {} {}",
                        p.processor_name,
                        instant.format(fmt).to_string(),
                        pre
                    );
                }
                while let Some(next) = q.pop_front() {
                    println!(
                        "{} r {} {}",
                        p.processor_name,
                        instant.format(fmt).to_string(),
                        next
                    );
                }
                p.next_line_buf = next_line_buf;
            }
            Msg::Write { instant, bytes } => {
                let ref mut p = processor_cache[processor_idx];
                println!(
                    "{} w {} {}",
                    p.processor_name,
                    instant.format(fmt).to_string(),
                    String::from_utf8_lossy(&bytes),
                );
                let _ = p.write_sender.send(WriteBuf::Buf(bytes));
            }
        }
    }

    for p in &processor_cache {
        let _ = p.write_sender.send(WriteBuf::Exit);
    }
    for p in processor_cache {
        let _ = p.join_handle.join();
    }

    Ok(())
}

impl ProcessorConfig {
    pub fn start_threads(self) -> Result<(), Error> {
        let (msg_sender, msg_receiver) = channel();
        let mut processor_cache = vec![];
        let mut writers = vec![];

        let mut processor_idx = 0;
        for p in &self.processors {
            let duration = Duration::from_millis(10);
            let path = p.port_name.clone();
            let baud_rate = p.baudrate.get();
            let builder = serialport::new(path, baud_rate).timeout(duration);
            let serial_port = builder.open()?;
            let processor_name = p.processor_name.borrow().clone();
            let msg_sender = msg_sender.clone();

            let mut processor_writer = ProcessorWriter {
                history_path: format!("{} history.txt", processor_name),
                editor: Editor::new(),
                processor_name: processor_name.clone(),
                processor_idx,
            };

            if processor_writer
                .editor
                .load_history(&processor_writer.history_path)
                .is_err()
            {
                println!("No previous history at '{}'", processor_writer.history_path);
            }
            writers.push(processor_writer);

            let (write_sender, write_receiver) = channel();
            processor_cache.push(ProcessorCache {
                processor_name,
                next_line_buf: NextLineBuf::Empty,
                write_sender,
                join_handle: thread::spawn(move || {
                    serial_port_task(serial_port, processor_idx, msg_sender, write_receiver)
                }),
            });

            processor_idx += 1;
        }

        let byte_process_thread =
            thread::spawn(move || byte_process_task(msg_receiver, processor_cache));

        let mut proc_switcher_editor = Editor::<()>::new();
        let proc_switcher_history_path = "history.txt";
        if proc_switcher_editor
            .load_history(proc_switcher_history_path)
            .is_err()
        {
            println!("No previous history at '{}'", proc_switcher_history_path);
        }
        processor_idx = 0;
        loop {
            let ref mut writer = writers[processor_idx];
            match writer.editor.readline("") {
                Ok(mut line) => {
                    writer.editor.add_history_entry(line.as_str());
                    line.push_str("\r");
                    let _ = msg_sender.send(Msg::Write {
                        bytes: line.as_bytes().into(),
                        instant: Utc::now(),
                    });
                }
                Err(ReadlineError::Interrupted) => {
                    break;
                }
                Err(ReadlineError::Eof) => {
                    let prompt = "Enter nickname of processor to switch to: ";
                    match proc_switcher_editor.readline(prompt) {
                        Ok(processor_name) => {
                            if let Some(w) =
                                writers.iter().find(|w| w.processor_name == processor_name)
                            {
                                if processor_idx == w.processor_idx {
                                    continue;
                                }
                                processor_idx = w.processor_idx;
                                println!("Switching to '{}'", processor_name);
                                let _ = msg_sender.send(Msg::SwitchToProcessor { processor_idx });
                                proc_switcher_editor.add_history_entry(processor_name.as_str());
                            } else {
                                println!("No match for '{}'", processor_name);
                            }
                        }
                        Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                            break;
                        }
                        Err(err) => {
                            println!("Error: {:?}", err);
                            break;
                        }
                    }
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        let _ = msg_sender.send(Msg::Exit);
        for mut e in writers {
            let _ = e.editor.save_history(&e.history_path);
        }
        let _ = proc_switcher_editor.save_history(proc_switcher_history_path);
        let _ = byte_process_thread.join();

        Ok(())
    }
}
