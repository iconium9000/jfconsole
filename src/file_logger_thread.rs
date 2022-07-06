use crate::byte_process_thread::DATE_TIME_FMT;
use chrono::{DateTime, Utc};
use std::{
    fs::{create_dir, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, Sender},
    thread::{self, yield_now, JoinHandle},
};

pub enum LogLine {
    Exit,
    Line {
        processor_name: String,
        read_write: &'static str,
        instant: DateTime<Utc>,
        line: String,
    },
}

pub struct FileLoggerThread {
    sender: Sender<LogLine>,
    join_handle: JoinHandle<Result<(), Box<dyn std::any::Any + Send>>>,
}

impl FileLoggerThread {
    pub fn line_sender(&self) -> Sender<LogLine> {
        self.sender.clone()
    }
    pub fn spawn(project_name: String) -> Result<FileLoggerThread, Box<dyn std::error::Error>> {
        let path = Path::new(&project_name);
        let _ = create_dir(path);

        let fmt = "%y%m%d_%H%M%S";
        let now = Utc::now();
        let file_name = format!("{}_{}.log", project_name, now.format(fmt));
        let file_path = path.join(Path::new(&file_name));

        let (sender, receiver) = channel();
        let task = move || file_logger_task(file_path, receiver);
        let join_handle = thread::spawn(task);
        Ok(FileLoggerThread {
            join_handle,
            sender,
        })
    }
    pub fn join(self) {
        let _ = self.sender.send(LogLine::Exit);
        let _ = self.join_handle.join();
    }
}

fn file_logger_task(
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
            println!("> [file_logger_task] opened {:?}", file_path);
        }
        Err(e) => {
            println!("> [file_logger_task] error {:?}", e);
            return Err(Box::new(e));
        }
    };

    loop {
        let mut exit = false;
        let mut sync = false;
        loop {
            match receiver.try_recv() {
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
                        println!("> [file_logger_task] write error {:#?}", e);
                        return Err(Box::new(e));
                    }
                }
                _ => break,
            }
            yield_now();
        }
        if sync {
            if let Err(e) = file.sync_all() {
                println!("> [file_logger_task] sync error {:#?}", e);
                return Err(Box::new(e));
            }
        }
        if exit {
            return Ok(println!("> [file_logger_task] end {:?}", file_path));
        }
    }
}
