use serialport::{UsbPortInfo, SerialPortType, available_ports};

use crate::{
    byte_process_thread::{ByteProcessThread, ProcessorByteCache},
    file_logger_thread::FileLoggerThread,
    read_config::UserSelectFileRes,
    serial_port_thread::SerialConsoleThread,
    user_console_thread::{user_console_task, ProcesserUserConsoleWriter},
};
use std::{sync::mpsc::channel, path::{PathBuf, Path}};

pub type BuadRate = u32;
pub const DEFAULT_BAUDRATE: BuadRate = 115_200;

pub struct ProcessorInfo {
    pub port_name: String,
    pub usb_port_info: UsbPortInfo,
    pub baudrate: BuadRate,
    pub processor_name: String,
}

impl ProcessorInfo {
    fn new(port_name: String, usb_port_info: UsbPortInfo) -> Self {
        Self {
            port_name,
            usb_port_info,
            baudrate: DEFAULT_BAUDRATE,
            processor_name: "".to_string(),
        }
    }
}

impl ProcessorInfo {
    pub fn duplicate(&self) -> Self {
        Self {
            port_name: self.port_name.clone(),
            usb_port_info: self.usb_port_info.clone(),
            baudrate: self.baudrate,
            processor_name: self.processor_name.clone(),
        }
    }
}

pub struct Config {
    pub processors: Vec<ProcessorInfo>,
    pub project_name: String,
    pub project_path: PathBuf,
}

impl ProcessorInfo {
    pub fn available_processors() -> Result<Vec<ProcessorInfo>, std::io::Error> {
        let mut procs = vec![];
        for serial_port_info in available_ports()? {
            if let SerialPortType::UsbPort(usb_port_info) = serial_port_info.port_type {
                procs.push(ProcessorInfo::new(
                    serial_port_info.port_name,
                    usb_port_info,
                ));
            }
        }
        Ok(procs)
    }
}

pub fn main_task() -> Result<(), Box<dyn std::error::Error>> {
    let procs = ProcessorInfo::available_processors()?;
    if procs.is_empty() {
        return Ok(println!("> [main_task] No com ports found"));
    }
    let cfg = loop {
        match Config::user_select_file(&procs) {
            UserSelectFileRes::Select(cfg) => break cfg,
            UserSelectFileRes::NoConfigs => break Config::user_create_custom(procs)?,
            UserSelectFileRes::SelectCustom => break Config::user_create_custom(procs)?,
            UserSelectFileRes::InvalidEntry => continue,
            UserSelectFileRes::Err(e) => return Err(e),
        }
    };
    if cfg.processors.is_empty() {
        return Ok(println!("> [main_task] no processors in config {:?}", cfg.project_path));
    }

    let mut serial_console_threads = vec![];
    let mut writers = vec![];
    let mut processor_byte_caches = vec![];

    let logger_thread = FileLoggerThread::spawn(&cfg.project_name)?;

    let (msg_sender, msg_receiver) = channel();
    let mut processor_idx = 0;
    for processor_info in cfg.processors {
        let serial_console_thread =
            SerialConsoleThread::spawn(&processor_info, processor_idx, &msg_sender)?;
        writers.push(ProcesserUserConsoleWriter::new(
            Path::new(&cfg.project_name),
            &processor_info,
            serial_console_thread.write_sender(),
        ));
        processor_byte_caches.push(ProcessorByteCache::new(
            &processor_info,
            logger_thread.logline_sender(),
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
