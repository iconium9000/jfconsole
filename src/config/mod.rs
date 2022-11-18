pub mod custom_config;
pub mod read_config;

pub type BuadRate = u32;
pub const DEFAULT_BAUD_RATE: BuadRate = 115_200;

use serialport::UsbPortInfo;
use std::path::PathBuf;

pub struct ProcessorInfo {
    pub port_name: String,
    pub usb_port_info: UsbPortInfo,
    pub baud_rate: BuadRate,
    pub processor_name: String,
}

impl ProcessorInfo {
    pub fn new(port_name: String, usb_port_info: UsbPortInfo) -> Self {
        Self {
            port_name,
            usb_port_info,
            baud_rate: DEFAULT_BAUD_RATE,
            processor_name: String::new(),
        }
    }
}

pub struct Config {
    pub processors: Box<[ProcessorInfo]>,
    pub project_name: String,
    pub project_path: PathBuf,
}
