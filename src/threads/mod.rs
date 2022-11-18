use crate::{
    config::read_config::UserSelectFileRes,
    threads::{file_logger_thread::FileLoggerThread},
    serial_console_thread::SerialConsoleThread,
    user_console_thread::{user_console_task, ProcessorUserConsoleWriter},
    utils::{
        line_printer::LinePrinter,
        ring_buf_queue::new_ring_buf_q,
        sync_flag::new_sync_flag,
        user_io::{BoxErr, BoxResult},
    },
};
use serialport::{available_ports, SerialPortType, UsbPortInfo};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::channel,
};
use thread_priority::{set_current_thread_priority, ThreadPriority};

pub mod file_logger_thread;

pub type BuadRate = u32;
pub const DEFAULT_BAUD_RATE: BuadRate = 115_200;
pub const DATE_TIME_FMT: &'static str = "%y-%m-%d %H:%M:%S%.3f";

pub const BUFFER_SIZE: usize = 0x1000;
pub const LINE_WIDTH: usize = 80;
pub const BYTE_PROCESS_THREAD_PRIORITY: u8 = 0;
pub const SERIAL_PORT_THREAD_PRIORITY: u8 = 1;
pub const USER_CONSOLE_THREAD_PRIORITY: u8 = 2;
pub const FILE_LOGGER_THREAD_PRIORITY: u8 = 3;

pub fn set_thread_priority<const PRIORITY: u8>() {
    let priority = ThreadPriority::Crossplatform(PRIORITY.try_into().unwrap());
    if let Err(e) = set_current_thread_priority(priority) {
        panic!("set_current_thread_priority => {:#?}", e);
    }
}

pub struct ProcessorInfo {
    pub port_name: String,
    pub usb_port_info: UsbPortInfo,
    pub baud_rate: BuadRate,
    pub processor_name: String,
}

impl ProcessorInfo {
    fn new(port_name: String, usb_port_info: UsbPortInfo) -> Self {
        Self {
            port_name,
            usb_port_info,
            baud_rate: DEFAULT_BAUD_RATE,
            processor_name: String::new(),
        }
    }
}

impl ProcessorInfo {
    pub fn duplicate(&self) -> Self {
        Self {
            port_name: self.port_name.clone(),
            usb_port_info: self.usb_port_info.clone(),
            baud_rate: self.baud_rate,
            processor_name: self.processor_name.clone(),
        }
    }
}

pub struct Config {
    pub processors: Box<[ProcessorInfo]>,
    pub project_name: String,
    pub project_path: PathBuf,
}

impl ProcessorInfo {
    pub fn available_processors() -> BoxResult<Vec<ProcessorInfo>> {
        let ports = available_ports().box_err()?;

        let mut procs = vec![];
        for serial_port_info in ports {
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

pub fn main_task() -> BoxResult<()> {
    println!("Welcome!\n\n");

    let proc_v = ProcessorInfo::available_processors()?;
    if proc_v.is_empty() {
        return Ok(println!("> [main_task] No com ports found"));
    }
    let cfg = loop {
        match Config::user_select_file(&proc_v) {
            UserSelectFileRes::Select(cfg) => break cfg,
            UserSelectFileRes::NoConfigs => break Config::user_create_custom(proc_v)?,
            UserSelectFileRes::SelectCustom => break Config::user_create_custom(proc_v)?,
            UserSelectFileRes::InvalidEntry => continue,
            UserSelectFileRes::Err(e) => return Err(e),
        }
    };
    if cfg.processors.is_empty() {
        return Ok(println!(
            "> [main_task] no processors in config {:?}",
            cfg.project_path
        ));
    }

    let (main_thread_victim, main_thread_assassin) = new_sync_flag();

    let (line_sender, line_receiver) = channel();
    let file_logger_thread =
        FileLoggerThread::spawn(&cfg.project_name, line_receiver, main_thread_assassin)?;

    let mut writer_v = vec![];
    let mut serial_console_thread_v = vec![];
    for processor_info in cfg.processors.into_vec() {
        let (write_producer, write_consumer) = new_ring_buf_q();
        let mut write_consumers = vec![write_consumer];

        let mut line_write_producer = None;
        if processor_info.processor_name == "f4" {
            let (write_producer, write_consumer) = new_ring_buf_q();
            line_write_producer = Some(write_producer);
            write_consumers.push(write_consumer);
        }
        serial_console_thread_v.push(SerialConsoleThread::<BUFFER_SIZE>::spawn(
            LinePrinter::new(
                format!("{} r", processor_info.processor_name),
                LINE_WIDTH,
                line_sender.clone(),
                line_write_producer,
            ),
            &processor_info,
            write_consumers,
        )?);
        writer_v.push(ProcessorUserConsoleWriter::new(
            Path::new(&cfg.project_name),
            &processor_info,
            LinePrinter::new(
                format!("{} w", processor_info.processor_name),
                LINE_WIDTH,
                line_sender.clone(),
                None,
            ),
            write_producer,
        ));
    }
    user_console_task(main_thread_victim, &mut writer_v);

    for serial_console_thread in serial_console_thread_v {
        let _ = serial_console_thread.join();
    }
    let _ = file_logger_thread.join();

    for writer in writer_v {
        writer.save_history();
    }

    Ok(println!("> [main_task] end"))
}
