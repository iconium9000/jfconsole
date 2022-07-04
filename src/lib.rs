extern crate serde;
extern crate serde_json;

use crate::user_io::{read_and_parse_user_entry, read_user_entry, ReadAndParseUserEntryRes};
use serde::{Deserialize, Serialize};
use serialport::{available_ports, SerialPortType, UsbPortInfo};
use std::{
    cell::{Cell, RefCell},
    fs::{self, DirEntry, File},
    io::{BufReader, Error as IOError, ErrorKind},
    num::ParseIntError,
    path::PathBuf,
    rc::Rc,
};

pub mod user_io;

pub type BuadRate = u32;
pub const DEFAULT_BAUDRATE: BuadRate = 115200;

pub struct Processor {
    pub port_name: String,
    pub usb_port_info: UsbPortInfo,
    pub baudrate: Cell<BuadRate>,
    pub processor_name: RefCell<String>,
    pub user_selected: Cell<bool>,
}

pub struct ProcessorConfig {
    pub processors: Vec<Rc<Processor>>,
    pub project_name: String,
    pub project_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigJson {
    project_name: String,
    processors: Vec<ProcessorConfigJson>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessorConfigJson {
    processor_name: String,
    baudrate: BuadRate,
    port_name: String,
}

impl ProcessorConfig {
    fn from_procs(
        project_path: PathBuf,
        cfg: ConfigJson,
        procs: &Vec<Rc<Processor>>,
    ) -> Result<Self, IOError> {
        let mut processors = vec![];
        let cfg_processors_len = cfg.processors.len();
        for p_json in cfg.processors {
            for p_rc in procs {
                if p_rc.port_name.eq(&p_json.port_name) {
                    processors.push(Rc::new(Processor {
                        port_name: p_rc.port_name.clone(),
                        usb_port_info: p_rc.usb_port_info.clone(),
                        baudrate: Cell::new(p_json.baudrate),
                        processor_name: RefCell::new(p_json.processor_name),
                        user_selected: Cell::new(true),
                    }));
                    break;
                }
            }
        }

        if processors.len() != cfg_processors_len {
            raise_ioerr!("port not found");
        }

        let project_name = cfg.project_name;
        Ok(Self {
            processors,
            project_name,
            project_path,
        })
    }
}

impl ProcessorConfig {
    fn read_config(
        dir_entry_res: Result<DirEntry, IOError>,
        procs: &Vec<Rc<Processor>>,
    ) -> Result<ProcessorConfig, IOError> {
        let dir_entry = dir_entry_res?;
        if dir_entry.file_type()?.is_dir() {
            raise_ioerr!("path to dir");
        }
        let project_path = dir_entry.path();
        if let Some(ext) = project_path.extension() {
            if !ext.eq("json") {
                raise_ioerr!("bad ext");
            }
        } else {
            raise_ioerr!("no ext");
        }

        let file = File::open(&project_path)?;
        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(cfg) => {
                return ProcessorConfig::from_procs(project_path, cfg, procs);
            }
            Err(_) => {
                raise_ioerr!("json parse error");
            }
        }
    }

    pub fn user_select() -> Result<Self, IOError> {
        let procs = Processor::list_processors()?;
        if procs.len() == 0 {
            raise_ioerr!("> No com ports found");
        }

        loop {
            let mut cfgs = vec![];
            for path_res in fs::read_dir("./")? {
                if let Ok(cfg) = Self::read_config(path_res, &procs) {
                    cfgs.push(cfg);
                }
            }
            if cfgs.len() <= 0 {
                println!("> No config files found");
                break;
            }
            println!("Config Options:");
            for (idx, cfg) in cfgs.iter().enumerate() {
                println!("{}) {} ({:?})", idx + 1, cfg.project_name, cfg.project_path);
            }
            let msg = "Enter index of config to use (or enter to create new config)";
            match read_and_parse_user_entry(msg) {
                ReadAndParseUserEntryRes::Ok(0) => {
                    println!("> Invalid entry\n");
                }
                ReadAndParseUserEntryRes::Ok(e) => {
                    let mut i = 0;
                    for cfg in cfgs {
                        i += 1;
                        if i == e {
                            println!("> Selected {} ({:?})", cfg.project_name, cfg.project_path);
                            return Ok(cfg);
                        }
                    }
                    println!("> Invalid entry\n");
                }
                ReadAndParseUserEntryRes::ParseErr(_) => {
                    println!("> Invalid entry\n");
                }
                ReadAndParseUserEntryRes::EmptyEntry => {
                    println!("> New Custom config\n");
                    break;
                }
                ReadAndParseUserEntryRes::IOErr(e) => return Err(e),
            }
        }

        for p in &procs {
            *p.processor_name.borrow_mut() = "".into();
            p.baudrate.set(DEFAULT_BAUDRATE);
            p.user_selected.set(false);
        }

        let mut selected = vec![];
        loop {
            match Processor::user_select(&procs) {
                UserSelectRes::Proc(p) => {
                    p.user_config()?;
                    selected.push(p);
                }
                UserSelectRes::AlreadySelected => {
                    println!("> Port already selected\n");
                    read_user_entry("press enter to continue")?;
                }
                UserSelectRes::NoneRemaining => {
                    println!("> No Ports remaining\n");
                    read_user_entry("press enter to continue")?;
                }
                UserSelectRes::EntryOutOfRange => {
                    println!("> Entry Out of Range\n");
                    read_user_entry("press enter to continue")?;
                }
                UserSelectRes::ParseErr(e) => {
                    println!("{:#?}", e);
                    read_user_entry("press enter to continue")?;
                }
                UserSelectRes::EmptyEntry => break,
                UserSelectRes::IOErr(e) => return Err(e),
            }
        }

        let project_name = read_user_entry("enter project name")?;
        println!("> Set project name as {}", project_name);

        let project_path = PathBuf::from(format!("./{}.json", project_name));
        let ref value = ConfigJson {
            project_name: project_name.clone(),
            processors: selected
                .iter()
                .map(|p| ProcessorConfigJson {
                    processor_name: p.processor_name.borrow().clone(),
                    baudrate: p.baudrate.get(),
                    port_name: p.port_name.clone(),
                })
                .collect(),
        };
        match serde_json::to_string_pretty(value) {
            Ok(contents) => {
                fs::write(project_path.clone(), contents)?;
                return Ok(Self {
                    processors: selected,
                    project_name,
                    project_path,
                });
            }
            Err(e) => {
                println!("{:#?}", e);
                raise_ioerr!("Serialize ProcessorConfigJson failed");
            }
        }
    }
}

impl Processor {
    pub fn list_processors() -> Result<Vec<Rc<Processor>>, IOError> {
        let mut procs = vec![];

        for serial_port_info in available_ports()? {
            if let SerialPortType::UsbPort(usb_port_info) = serial_port_info.port_type {
                let port_name = serial_port_info.port_name;
                if let Some(p) = Processor::new(port_name, usb_port_info) {
                    procs.push(Rc::new(p));
                }
            }
        }

        Ok(procs)
    }

    fn new(port_name: String, usb_port_info: UsbPortInfo) -> Option<Processor> {
        Some(Processor {
            port_name,
            usb_port_info,
            baudrate: Cell::new(DEFAULT_BAUDRATE),
            processor_name: RefCell::new("".into()),
            user_selected: Cell::new(false),
        })
    }
}

pub enum UserSelectRes {
    Proc(Rc<Processor>),
    NoneRemaining,
    AlreadySelected,
    EntryOutOfRange,
    EmptyEntry,
    ParseErr(ParseIntError),
    IOErr(IOError),
}

impl Processor {
    pub fn user_select(procs: &Vec<Rc<Processor>>) -> UserSelectRes {
        println!("Serial Ports to select from:");
        for (idx, p) in procs.iter().enumerate() {
            println!(
                "{}) {} {} {}",
                idx + 1,
                p.port_name,
                p.processor_name.borrow(),
                if p.user_selected.get() {
                    "(selected)"
                } else {
                    ""
                }
            );
        }

        let msg = "Enter index of port to add (or enter to stop)";
        match read_and_parse_user_entry::<usize>(msg) {
            ReadAndParseUserEntryRes::Ok(entered_idx) => {
                if 1 > entered_idx || entered_idx > procs.len() {
                    UserSelectRes::EntryOutOfRange
                } else {
                    let ref p = procs[entered_idx - 1];
                    if p.user_selected.get() {
                        UserSelectRes::AlreadySelected
                    } else {
                        UserSelectRes::Proc(Rc::clone(p))
                    }
                }
            }
            ReadAndParseUserEntryRes::IOErr(e) => UserSelectRes::IOErr(e),
            ReadAndParseUserEntryRes::ParseErr(e) => UserSelectRes::ParseErr(e),
            ReadAndParseUserEntryRes::EmptyEntry => UserSelectRes::EmptyEntry,
        }
    }

    pub fn user_config(&self) -> Result<(), IOError> {
        println!("> Selected {}\n", self.port_name);
        self.user_selected.set(true);

        let s = read_user_entry("Enter nickname for processor")?;
        println!("> Nicknamed {} as {}\n", self.port_name, s);
        *self.processor_name.borrow_mut() = s;

        let mut baudrate = 0;
        while baudrate == 0 {
            println!(
                "Baud rates options for {} port '{}':",
                self.processor_name.borrow(),
                self.port_name,
            );
            println!("1) 115200");
            println!("2) 3000000");
            println!("_) custom value");

            match read_and_parse_user_entry("Enter 1, 2, or a custom value") {
                ReadAndParseUserEntryRes::IOErr(e) => return Err(e),
                ReadAndParseUserEntryRes::Ok(1) => {
                    baudrate = 115200;
                }
                ReadAndParseUserEntryRes::Ok(2) => {
                    baudrate = 3000000;
                }
                ReadAndParseUserEntryRes::Ok(0) => {
                    println!("> Empty Entry, try again\n");
                }
                ReadAndParseUserEntryRes::Ok(e) => {
                    baudrate = e;
                }
                ReadAndParseUserEntryRes::EmptyEntry => {
                    println!("> Empty Entry, try again\n");
                }
                ReadAndParseUserEntryRes::ParseErr(_) => {
                    println!("> Parse Error, try again\n");
                }
            }
        }
        println!(
            "> Set baudrate of {} as {}\n",
            self.processor_name.borrow(),
            baudrate
        );

        Ok(self.baudrate.set(baudrate))
    }
}
