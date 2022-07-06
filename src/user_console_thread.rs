use crate::{byte_process_thread::Msg, main_thread::ProcessorInfo, serial_port_thread::WriteBuf};
use chrono::Utc;
use rustyline::{error::ReadlineError, Editor};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

pub struct ProcesserUserConsoleWriter {
    editor: Editor<()>,
    history_path: PathBuf,
    processor_name: String,
    write_sender: Sender<WriteBuf>,
}

impl ProcesserUserConsoleWriter {
    pub fn new(
        project_path: &Path,
        processor_info: &ProcessorInfo,
        write_sender: Sender<WriteBuf>,
    ) -> Self {
        let history_filename = format!("{} cmd history.txt", processor_info.processor_name);
        let mut writer = ProcesserUserConsoleWriter {
            history_path: project_path.join(Path::new(&history_filename)),
            editor: Editor::new(),
            write_sender,
            processor_name: processor_info.processor_name.clone(),
        };
        if writer.editor.load_history(&writer.history_path).is_err() {
            println!(
                "> [user_console_task] no previous {} cmd history at {:?}",
                writer.processor_name, writer.history_path,
            );
        } else {
            println!(
                "> [user_console_task] recovered {} cmd history from {:?}",
                writer.processor_name, writer.history_path,
            );
        }
        writer
    }
}

impl ProcesserUserConsoleWriter {
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

pub fn user_console_task(writers: &mut Vec<ProcesserUserConsoleWriter>, msg_sender: &Sender<Msg>) {
    let mut processor_idx = 0;
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
                println!("> [user_console_task] switching to {:?}", w.processor_name);
            }
            Err(err) => {
                println!("> [user_console_task] error: {:#?}", err);
                break;
            }
        }
    }
    println!("> [user_console_task] end");
}
