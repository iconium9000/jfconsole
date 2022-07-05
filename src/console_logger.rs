use std::{
    fs::{create_dir, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, Sender},
    thread::{self, JoinHandle},
};

use chrono::{DateTime, Utc};

use crate::console_threads::DATE_TIME_FMT;

pub enum LogLine {
    Exit,
    Line {
        processor_name: String,
        read_write: &'static str,
        instant: DateTime<Utc>,
        line: String,
    },
}

pub struct ConsoleLogger {
    sender: Sender<LogLine>,
    join_handle: JoinHandle<Result<(), Box<dyn std::any::Any + Send>>>,
}

fn console_logger_task(
    file_path: PathBuf,
    receiver: Receiver<LogLine>,
) -> Result<(), Box<dyn std::any::Any + Send>> {
    let mut file;
    match OpenOptions::new()
        .create(true)
        .write(true)
        .open(file_path.clone())
    {
        Ok(opened_file) => {
            file = opened_file;
            println!("> [console_logger_task] opened {:?}", file_path);
        }
        Err(e) => {
            println!("> [console_logger_task] error {:?}", e);
            return Err(Box::new(e));
        }
    };

    loop {
        let mut exit = false;
        let mut sync = false;
        loop {
            match receiver.try_recv() {
                Err(_) => break,
                Ok(LogLine::Exit) => {
                    sync = true;
                    exit = true;
                }
                Ok(LogLine::Line {
                    processor_name,
                    read_write,
                    instant,
                    line,
                }) => {
                    sync = true;
                    let line = format!(
                        "{} {} {} {}\r\n",
                        processor_name,
                        read_write,
                        instant.format(DATE_TIME_FMT).to_string(),
                        line
                    );
                    let buf = line.as_bytes();
                    if let Err(e) = file.write_all(buf) {
                        println!("> [console_logger_task] write error {:#?}", e);
                        return Err(Box::new(e));
                    }
                }
            }
        }
        if sync {
            if let Err(e) = file.sync_all() {
                println!("> [console_logger_task] sync error {:#?}", e);
                return Err(Box::new(e));
            }
        }
        if exit {
            return Ok(println!("> [console_logger_task] end {:?}", file_path));
        }
    }
}

impl ConsoleLogger {
    pub fn sender(&self) -> Sender<LogLine> {
        self.sender.clone()
    }
    pub fn exit(&self) {
        let _ = self.sender.send(LogLine::Exit);
    }
    pub fn join(self) -> Result<(), Box<dyn std::any::Any + Send>> {
        self.join_handle.join()?
    }
    pub fn new(project_name: String) -> Result<ConsoleLogger, Box<dyn std::error::Error>> {
        let path = Path::new(&project_name);
        let _ = create_dir(path);

        let fmt = "%y%m%d_%H%M%S";
        let now = Utc::now();
        let file_name = format!("{}_{}.log", project_name, now.format(fmt));
        let file_path = path.join(Path::new(&file_name));

        let (sender, receiver) = channel();
        let task = move || console_logger_task(file_path, receiver);
        let join_handle = thread::spawn(task);
        Ok(ConsoleLogger {
            join_handle,
            sender,
        })
    }
}
