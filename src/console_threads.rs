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
use serialport::{Error, TTYPort};

enum Msg {
    SwitchToProcessor {
        processor_name: String,
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

struct ProcessorCache {
    processor_name: String,
    processor_idx: usize,
    next_line_buf: NextLineBuf,
    write_sender: Sender<WriteBuf>,
    join_handle: JoinHandle<Result<(), Error>>,
}

fn serial_port_task(
    mut serial_port: TTYPort,
    processor_idx: usize,
    msg_sender: Sender<Msg>,
    write_receiver: Receiver<WriteBuf>,
) -> Result<(), Error> {
    println!("start serial_port_task");
    let mut readbuf = [0u8; 0x100];
    // let cr = "\r".as_bytes();
    loop {
        // println!("hello there {}", processor_idx);
        // let _ = serial_port.write(cr);
        // thread::sleep(Duration::from_secs(1));

        let rcved = match write_receiver.try_recv() {
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

        if match serial_port.read(&mut readbuf) {
            Err(_) => true,
            Ok(0) => true,
            Ok(count) => {
                let _ = msg_sender.send(Msg::Read {
                    processor_idx,
                    instant: Utc::now(),
                    bytes: Box::from(&readbuf[..count]),
                });
                // println!("serial_port_task send {:#?}", e);
                !rcved
            }
        } {
            thread::sleep(Duration::from_millis(1));
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
            Msg::SwitchToProcessor { processor_name } => {
                let p = processor_cache
                    .iter()
                    .find(|p| p.processor_name == processor_name);
                if let Some(p) = p {
                    processor_idx = p.processor_idx;
                    println!("Switching to '{}'", processor_name);
                } else {
                    println!("No match for '{}'", processor_name);
                }
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

        let mut processor_idx = 0;
        for p in &self.processors {
            let duration = Duration::from_millis(10);
            let path = p.port_name.clone();
            let baud_rate = p.baudrate.get();
            let builder = serialport::new(path, baud_rate).timeout(duration);
            let serial_port = TTYPort::open(&builder)?;
            let processor_name = p.processor_name.borrow().clone();
            let msg_sender = msg_sender.clone();

            let (write_sender, write_receiver) = channel();
            processor_cache.push(ProcessorCache {
                processor_name,
                processor_idx,
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

        let mut rl = Editor::<()>::new();
        if rl.load_history("history.txt").is_err() {
            println!("No previous history.");
        }
        loop {
            match rl.readline("") {
                Ok(mut line) => {
                    rl.add_history_entry(line.as_str());
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
                    match rl.readline("Enter nickname of processor to switch to: ") {
                        Ok(processor_name) => {
                            rl.add_history_entry(processor_name.as_str());
                            let _ = msg_sender.send(Msg::SwitchToProcessor { processor_name });
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
        rl.save_history("history.txt").unwrap();

        let _ = msg_sender.send(Msg::Exit);
        let _ = byte_process_thread.join();

        Ok(())
    }
}
