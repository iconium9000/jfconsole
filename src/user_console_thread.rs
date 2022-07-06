use std::sync::mpsc::Sender;

use chrono::Utc;
use rustyline::{error::ReadlineError, Editor};

use crate::{byte_process_thread::Msg, serial_port_thread::WriteBuf, ProcessorInfo};

pub struct ProcesserUserConsoleWriter {
    editor: Editor<()>,
    history_path: String,
    processor_name: String,
    write_sender: Sender<WriteBuf>,
}

impl ProcesserUserConsoleWriter {
    pub fn new(processor_info: &ProcessorInfo, write_sender: Sender<WriteBuf>) -> Self {
        let mut writer = ProcesserUserConsoleWriter {
            history_path: format!("{} history.txt", processor_info.processor_name),
            editor: Editor::new(),
            write_sender,
            processor_name: processor_info.processor_name.clone(),
        };
        if writer.editor.load_history(&writer.history_path).is_err() {
            println!(
                "> [main_task] no previous history at {:?}",
                writer.history_path
            );
        }
        writer
    }
}

impl ProcesserUserConsoleWriter {
    pub fn save_history(mut self) {
        let _ = self.editor.save_history(&self.history_path);
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
