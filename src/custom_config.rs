use std::path::PathBuf;

use rustyline::Editor;

use crate::{
    user_io::{read_and_parse_user_entry, ReadAndParseUserEntryRes},
    Config, ProcessorInfo, UserSelectConfigRes,
};

impl ProcessorInfo {
    pub fn user_select(procs: &mut Vec<ProcessorInfo>) -> UserSelectConfigRes {
        println!("Serial Ports to select from:");
        for (idx, p) in procs.iter().enumerate() {
            println!("{}) {} {}", idx + 1, p.port_name, p.processor_name,);
        }

        let msg = "Enter index of port to add (or enter to stop)";
        match read_and_parse_user_entry::<usize>(msg) {
            ReadAndParseUserEntryRes::Ok(entered_idx) => {
                if 1 > entered_idx || entered_idx > procs.len() {
                    UserSelectConfigRes::EntryOutOfRange
                } else {
                    UserSelectConfigRes::Proc(procs.remove(entered_idx - 1))
                }
            }
            ReadAndParseUserEntryRes::IOErr(e) => UserSelectConfigRes::IOErr(e),
            ReadAndParseUserEntryRes::ParseErr { e, user_entry } => {
                UserSelectConfigRes::ParseErr { e, user_entry }
            }
            ReadAndParseUserEntryRes::ReadErr(e) => UserSelectConfigRes::ReadErr(e),
            ReadAndParseUserEntryRes::EmptyEntry => UserSelectConfigRes::EmptyEntry,
        }
    }
}

impl ProcessorInfo {
    pub fn user_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("> Selected {}\n", self.port_name);

        let mut editor = Editor::<()>::new();
        let prompt = "Enter nickname for processor: ";
        let op = |e| Err(Box::new(e));
        self.processor_name = editor.readline(prompt).or_else(op)?;
        println!(
            "> Nicknamed {} as {}\n",
            self.port_name, self.processor_name
        );

        self.baudrate = 0;
        while self.baudrate == 0 {
            println!(
                "Baud rates options for {:?} port {:?}:",
                self.processor_name, self.port_name,
            );
            println!("1) 115200");
            println!("2) 3000000");
            println!("_) custom value");

            match read_and_parse_user_entry("Enter 1, 2, or a custom value") {
                ReadAndParseUserEntryRes::IOErr(e) => {
                    return Err(Box::new(e));
                }
                ReadAndParseUserEntryRes::ReadErr(e) => {
                    return Err(Box::new(e));
                }
                ReadAndParseUserEntryRes::Ok(0) => {
                    println!("> Invalid Entry, try again\n");
                }
                ReadAndParseUserEntryRes::Ok(1) => {
                    self.baudrate = 115200;
                }
                ReadAndParseUserEntryRes::Ok(2) => {
                    self.baudrate = 3000000;
                }
                ReadAndParseUserEntryRes::Ok(e) => {
                    self.baudrate = e;
                }
                ReadAndParseUserEntryRes::EmptyEntry => {
                    println!("> Empty Entry, try again\n");
                }
                ReadAndParseUserEntryRes::ParseErr { e, user_entry } => {
                    println!("> Invalid Entry {:?} {:?}\n", user_entry, e);
                }
            }
        }
        println!(
            "> Set baudrate of {:?} as {}\n",
            self.processor_name, self.baudrate
        );

        Ok(())
    }
}

impl Config {
    pub fn user_create_custom(
        mut procs: Vec<ProcessorInfo>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut editor = Editor::<()>::new();
        let mut selected = vec![];
        loop {
            match ProcessorInfo::user_select(&mut procs) {
                UserSelectConfigRes::Proc(mut p) => {
                    if let Err(e) = p.user_config() {
                        println!("> error {:?}", e);
                        procs.push(p);
                    } else {
                        selected.push(p);
                        continue;
                    }
                }
                UserSelectConfigRes::EntryOutOfRange => {
                    println!("> Entry Out of Range\n");
                }
                UserSelectConfigRes::ParseErr { e, user_entry } => {
                    println!("> Invalid Entry {:?} {:?}\n", user_entry, e);
                }
                UserSelectConfigRes::EmptyEntry => break,
                UserSelectConfigRes::NoneRemaining => {
                    println!("> No Ports remaining\n");
                    break;
                }
                UserSelectConfigRes::IOErr(e) => return Err(Box::new(e)),
                UserSelectConfigRes::ReadErr(e) => return Err(Box::new(e)),
            }
            let _ = editor.readline("press enter to continue: ");
        }

        let prompt = "enter project name: ";
        let op = |e| Err(Box::new(e));
        let project_name = editor.readline(prompt).or_else(op)?;
        println!("> Set project name as {}", project_name);

        let project_path = PathBuf::from(format!("./{}.json", project_name));
        Self {
            processors: selected,
            project_name,
            project_path,
        }
        .save_config_file()
    }
}
