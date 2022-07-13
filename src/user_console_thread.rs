use std::path::{Path, PathBuf};

use rustyline::{error::ReadlineError, Editor};

use crate::{
    buf_iter::RingBufQProducer,
    line_printer::LinePrinter,
    main_thread::{set_thread_priority, ProcessorInfo, USER_CONSOLE_THREAD_PRIORITY},
    sync_flag::SyncFlagVictim,
};

pub struct ProcesserUserConsoleWriter {
    processor_name: String,
    history_path: PathBuf,
    editor: Editor<()>,
    write_producer: RingBufQProducer<u8>,
    line_printer: LinePrinter,
}

pub enum ReadLineRes {
    Line(String),
    NextProcessor,
    Exit,
}

impl ProcesserUserConsoleWriter {
    pub fn new(
        project_path: &Path,
        processor_info: &ProcessorInfo,
        line_printer: LinePrinter,
        write_producer: RingBufQProducer<u8>,
    ) -> Self {
        let history_filename = format!("{} cmd history.txt", processor_info.processor_name);
        let history_path = project_path.join(Path::new(&history_filename));
        let mut editor = Editor::new();
        if editor.load_history(&history_path).is_err() {
            println!(
                "> [user_console_task] no previous {} cmd history at {:?}",
                processor_info.processor_name, history_path,
            );
        } else {
            println!(
                "> [user_console_task] recovered {} cmd history from {:?}",
                processor_info.processor_name, history_path,
            );
        }
        Self {
            history_path,
            editor,
            write_producer,
            line_printer,
            processor_name: processor_info.processor_name.clone(),
        }
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
    pub fn save_history(mut self) {
        if let Err(e) = self.editor.save_history(&self.history_path) {
            println!(
                "> [user_console_task] saving {} cmd history failed with {:?}",
                self.processor_name, e
            );
        } else {
            println!(
                "> [user_console_task] saved {} cmd history to {:?}",
                self.processor_name, self.history_path
            )
        }
    }
}

pub fn user_console_task(victim: SyncFlagVictim, writers: &mut [ProcesserUserConsoleWriter]) {
    set_thread_priority::<USER_CONSOLE_THREAD_PRIORITY>();
    let mut processor_idx = 0;
    let mut writer = &mut writers[processor_idx];
    while victim.is_alive() {
        match writer.readline() {
            ReadLineRes::Line(mut line) => {
                writer.editor.add_history_entry(&line);
                line.push('\r');
                writer.write_producer.push(line.as_bytes());
                writer.line_printer.push_str(&line);
            }
            ReadLineRes::NextProcessor => {
                processor_idx += 1;
                processor_idx %= writers.len();
                writer = &mut writers[processor_idx];
                println!(
                    "> [user_console_task] switching to {:?}",
                    writer.processor_name,
                );
                continue;
            }
            ReadLineRes::Exit => {
                println!("> [user_console_task] ended");
                return;
            }
        }
    }
    println!("> [user_console_task] ended without user input");
}
