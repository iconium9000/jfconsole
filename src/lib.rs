extern crate serde;
extern crate serde_json;

use read_config::UserSelectFileRes;
use rustyline::error::ReadlineError;
use serialport::{available_ports, SerialPortType, UsbPortInfo};
use std::{num::ParseIntError, path::PathBuf};
use user_io::RaisedError;

pub mod file_logger_thread;
pub mod byte_process_thread;
pub mod custom_config;
pub mod main_thread;
pub mod read_config;
pub mod serial_port_thread;
pub mod user_io;
pub mod user_console_thread;

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
                let port_name = serial_port_info.port_name;
                procs.push(ProcessorInfo::new(port_name, usb_port_info));
            }
        }
        Ok(procs)
    }
}

pub enum UserSelectConfigRes {
    Proc(ProcessorInfo),
    NoneRemaining,
    EntryOutOfRange,
    EmptyEntry,
    ParseErr {
        e: ParseIntError,
        user_entry: String,
    },
    IOErr(std::io::Error),
    ReadErr(ReadlineError),
}

impl Config {
    pub fn user_select() -> Result<Config, Box<dyn std::error::Error>> {
        let procs = ProcessorInfo::available_processors()?;
        if procs.is_empty() {
            return Err(RaisedError::new("> No com ports found"));
        }
        loop {
            match Config::user_select_file(&procs) {
                UserSelectFileRes::Select(cfg) => return Ok(cfg),
                UserSelectFileRes::NoConfigs => break,
                UserSelectFileRes::SelectCustom => break,
                UserSelectFileRes::InvalidEntry => continue,
                UserSelectFileRes::Err(e) => return Err(e),
            }
        }
        Config::user_create_custom(procs)
    }
}
