use chrono::{DateTime, Utc};
use rustyline::{error::ReadlineError, Editor};

use crate::main_thread::{set_thread_priority, DATE_TIME_FMT, USER_CONSOLE_THREAD_PRIORITY};

#[derive(Clone)]
pub struct ProcFmt {
    nick: String,
    timestamp: String,
}

impl ProcFmt {
    pub fn new(nick: String, start_date_time: DateTime<Utc>) -> Self {
        let timestamp = Self::timestamp(start_date_time);
        Self { nick, timestamp }
    }
    pub fn nick(&self) -> &str {
        &self.nick
    }
    fn timestamp(date_time: DateTime<Utc>) -> String {
        date_time.format(DATE_TIME_FMT).to_string()
    }
    pub fn set_time(&mut self, date_time: DateTime<Utc>) {
        self.timestamp = Self::timestamp(date_time)
    }
    pub fn fmt_write_line(&self, line: &str) -> String {
        format!("{} w {} {}", self.nick, self.timestamp, line)
    }
    pub fn fmt_read_line(&self, line: &str) -> String {
        format!("{} r {} {}", self.nick, self.timestamp, line)
    }
}

pub struct ProcesserUserConsoleWriter {
    proc_fmt: ProcFmt,
    editor: Editor<()>,
}

pub enum ReadLineRes {
    Line(String),
    NextProcessor,
    Exit,
}

impl ProcesserUserConsoleWriter {
    pub fn new(proc_fmt: ProcFmt) -> Self {
        let editor = Editor::new();
        Self { proc_fmt, editor }
    }
    pub fn readline(&mut self) -> ReadLineRes {
        match self.editor.readline("") {
            Ok(line) => ReadLineRes::Line(line),
            Err(ReadlineError::Interrupted) => ReadLineRes::Exit,
            Err(ReadlineError::Eof) => ReadLineRes::NextProcessor,
            Err(err) => {
                println!("> [user_console_task] error: {:#?}", err);
                ReadLineRes::Exit
            }
        }
    }
}

pub fn user_console_task(writers: &mut [ProcesserUserConsoleWriter]) {
    set_thread_priority::<USER_CONSOLE_THREAD_PRIORITY>();

    let n_writers = writers.len();
    let mut processor_idx = 0;
    let mut writer = &mut writers[processor_idx];
    loop {
        match writer.readline() {
            ReadLineRes::Line(line) => {
                writer.proc_fmt.set_time(Utc::now());
                let fmt = writer.proc_fmt.fmt_write_line(&line);
                println!("{}", fmt);
            }
            ReadLineRes::NextProcessor => {
                processor_idx += 1;
                processor_idx %= n_writers;
                writer = &mut writers[processor_idx];
                println!(
                    "> [user_console_task] switching to {:?}",
                    writer.proc_fmt.nick()
                );
                continue;
            }
            ReadLineRes::Exit => break,
        }
    }
    println!("> [user_console_task] end")
}
