use crate::{
    read_config::UserSelectFileRes,
    serial_console_thread::SerialConsoleThread,
    user_console_thread::{user_console_task, ProcFmt, ProcesserUserConsoleWriter},
};
use chrono::Utc;
use serialport::{available_ports, SerialPortType, UsbPortInfo};
use std::{any::Any, path::PathBuf};
use thread_priority::ThreadPriority;

pub type BuadRate = u32;
pub const DEFAULT_BAUDRATE: BuadRate = 115_200;
pub const DATE_TIME_FMT: &'static str = "%y-%m-%d %H:%M:%S%.3f";

pub const BYTE_PROCESS_THREAD_PRIORITY: u8 = 0;
pub const SERIAL_PORT_THREAD_PRIORITY: u8 = 1;
pub const USER_CONSOLE_THREAD_PRIORITY: u8 = 2;
pub const FILE_LOGGER_THREAD_PRIORITY: u8 = 3;

pub trait BoxErr<T> {
    fn box_err(self) -> Result<T, Box<dyn Any + Send>>;
}

impl<T, E: Any + Send> BoxErr<T> for Result<T, E> {
    fn box_err(self) -> Result<T, Box<dyn Any + Send>> {
        match self {
            Ok(ok) => Ok(ok),
            Err(e) => Err(Box::new(e)),
        }
    }
}

pub fn set_thread_priority<const PRIORITY: u8>() {
    ThreadPriority::Crossplatform(PRIORITY.try_into().unwrap())
        .set_for_current()
        .unwrap();
}

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
    pub processors: Box<[ProcessorInfo]>,
    pub project_name: String,
    pub project_path: PathBuf,
}

impl ProcessorInfo {
    pub fn available_processors() -> Result<Vec<ProcessorInfo>, Box<dyn Any + Send>> {
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

pub fn main_task() -> Result<(), Box<dyn Any + Send>> {
    println!("Welcome!\n\n");

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
        return Ok(println!(
            "> [main_task] no processors in config {:?}",
            cfg.project_path
        ));
    }
    let mut writers = vec![];
    let mut serial_console_threads = vec![];
    for processor_info in cfg.processors.into_vec() {
        let start_date_time = Utc::now();
        let nick = processor_info.processor_name.clone();
        let proc_fmt = ProcFmt::new(nick, start_date_time);
        let serial_console_thread = SerialConsoleThread::spawn(proc_fmt.clone(), &processor_info)?;
        let writer = ProcesserUserConsoleWriter::new(proc_fmt.clone());

        serial_console_threads.push(serial_console_thread);
        writers.push(writer);
    }

    user_console_task(&mut writers);

    for serial_console_thread in serial_console_threads {
        serial_console_thread.join();
    }

    Ok(println!("> [main_task] end"))
}
