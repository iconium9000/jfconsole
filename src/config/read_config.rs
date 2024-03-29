use crate::{
    config::{BuadRate, Config, ProcessorInfo},
    utils::user_io::{
        read_and_parse_user_entry, BoxErr, BoxResult, RaisedError,
        ReadAndParseUserEntryRes,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, DirEntry, File},
    io::BufReader,
    path::PathBuf,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigDto {
    project_name: String,
    processors: Box<[ProcessorInfoDto]>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessorInfoDto {
    processor_name: String,
    baud_rate: BuadRate,
    port_name: String,
}

impl ProcessorInfo {
    fn to_dto(&self) -> ProcessorInfoDto {
        ProcessorInfoDto {
            processor_name: self.processor_name.clone(),
            baud_rate: self.baud_rate,
            port_name: self.port_name.clone(),
        }
    }
}

impl ProcessorInfo {
    pub fn duplicate_from_dto(&self, dto: ProcessorInfoDto) -> Self {
        Self {
            port_name: self.port_name.clone(),
            usb_port_info: self.usb_port_info.clone(),
            baud_rate: dto.baud_rate,
            processor_name: dto.processor_name,
        }
    }
}

impl Config {
    pub fn from_dto(
        project_path: PathBuf,
        cfg: ConfigDto,
        proc_info: &[ProcessorInfo],
    ) -> BoxResult<Self> {
        let mut processors = vec![];
        let cfg_processors_len = cfg.processors.len();
        for p_dto in cfg.processors.into_vec() {
            for p_rc in proc_info {
                if p_rc.port_name.eq(&p_dto.port_name) {
                    processors.push(p_rc.duplicate_from_dto(p_dto));
                    break;
                }
            }
        }

        if processors.len() == cfg_processors_len {
            Ok(Self {
                processors: processors.into(),
                project_name: cfg.project_name,
                project_path,
            })
        } else {
            Err(RaisedError::new("port not found"))
        }
    }
}

impl Config {
    pub fn save_config_file(self) -> Self {
        let ref value = ConfigDto {
            project_name: self.project_name.clone(),
            processors: self.processors.iter().map(|p| p.to_dto()).collect(),
        };

        let contents = serde_json::to_string_pretty(value).unwrap();
        fs::write(self.project_path.clone(), contents).unwrap();
        self
    }
}

impl Config {
    pub fn read_config_file(
        dir_entry: DirEntry,
        procs: &[ProcessorInfo],
    ) -> BoxResult<Config> {
        if dir_entry.file_type().box_err()?.is_dir() {
            return Err(RaisedError::new("path to dir"));
        }
        let project_path = dir_entry.path();
        match project_path.extension() {
            Some(ext) if ext == "json" => {
                let file = File::open(&project_path).box_err()?;
                let reader = BufReader::new(file);
                match serde_json::from_reader(reader) {
                    Ok(cfg) => Config::from_dto(project_path, cfg, procs),
                    Err(e) => Err(Box::new(e)),
                }
            }
            None => Err(RaisedError::new("no ext")),
            _ => Err(RaisedError::new("bad ext")),
        }
    }
}

pub enum UserSelectFileRes {
    Select(Config),
    NoConfigs,
    SelectCustom,
    InvalidEntry,
}

impl Config {
    pub fn user_select_file(procs: &[ProcessorInfo]) -> UserSelectFileRes {
        let mut config_vec = vec![];
        let paths = fs::read_dir("./config").unwrap();
        for dir_entry_res in paths {
            if let Ok(dir_entry) = dir_entry_res {
                match Self::read_config_file(dir_entry, &procs) {
                    Ok(cfg) => config_vec.push(cfg),
                    _ => {}
                }
            }
        }
        if config_vec.is_empty() {
            println!("> No config files found");
            return UserSelectFileRes::NoConfigs;
        }
        println!("Config Options:");
        for (idx, cfg) in config_vec.iter().enumerate() {
            println!("{}) {} ({:?})", idx + 1, cfg.project_name, cfg.project_path);
        }
        let msg = "Enter index of config to use (or enter to create new config)";
        match read_and_parse_user_entry(msg) {
            ReadAndParseUserEntryRes::Ok(0) => {
                println!("> Invalid entry\n");
                UserSelectFileRes::InvalidEntry
            }
            ReadAndParseUserEntryRes::Ok(e) => {
                let mut i = 0;
                for cfg in config_vec {
                    i += 1;
                    if i == e {
                        println!(
                            "> Selected {:?} from {:?}",
                            cfg.project_name, cfg.project_path
                        );
                        return UserSelectFileRes::Select(cfg);
                    }
                }
                println!("> Invalid entry\n");
                UserSelectFileRes::InvalidEntry
            }
            ReadAndParseUserEntryRes::ParseErr { e, user_entry } => {
                println!("> Invalid Entry {:?} {:?}\n", user_entry, e);
                UserSelectFileRes::InvalidEntry
            }
            ReadAndParseUserEntryRes::EmptyEntry => {
                println!("> New Custom config\n");
                UserSelectFileRes::SelectCustom
            }
            ReadAndParseUserEntryRes::IOErr(e) => panic!("IOErr({:?}", e),
            ReadAndParseUserEntryRes::ReadErr(e) => panic!("ReadErr({:?}", e),
        }
    }
}
