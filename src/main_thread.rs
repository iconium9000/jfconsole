use crate::{
    byte_process_thread::{ByteProcessThread, ProcessorByteCache},
    file_logger_thread::FileLoggerThread,
    serial_port_thread::SerialConsoleThread,
    user_console_thread::{user_console_task, ProcesserUserConsoleWriter},
    Config,
};
use std::sync::mpsc::channel;

impl Config {
    pub fn main_task(self) -> Result<(), Box<dyn std::error::Error>> {
        if self.processors.is_empty() {
            return Ok(println!("> [main_task] no processors"));
        }

        let (msg_sender, msg_receiver) = channel();
        let mut serial_console_threads = vec![];
        let mut writers = vec![];
        let mut processor_byte_caches = vec![];

        let project_name = String::clone(&self.project_name);
        let logger_thread = FileLoggerThread::spawn(project_name)?;

        let mut processor_idx = 0;
        for processor_info in self.processors {
            let serial_console_thread = SerialConsoleThread::spawn(
                &processor_info,
                processor_idx,
                &msg_sender,
            )?;
            writers.push(ProcesserUserConsoleWriter::new(
                &processor_info,
                serial_console_thread.write_sender(),
            ));
            processor_byte_caches.push(ProcessorByteCache::new(
                &processor_info,
                logger_thread.line_sender(),
            ));
            serial_console_threads.push(serial_console_thread);
            processor_idx += 1;
        }

        let byte_process_thread =
            ByteProcessThread::spawn(&msg_sender, msg_receiver, processor_byte_caches);

        user_console_task(&mut writers, &msg_sender);

        for writer in writers {
            writer.save_history();
        }

        for serial_console_thread in serial_console_threads {
            serial_console_thread.join();
        }

        byte_process_thread.join();
        logger_thread.join();

        Ok(println!("> [main_task] end"))
    }
}
