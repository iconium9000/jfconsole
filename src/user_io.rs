use std::{
    error::Error,
    str::FromStr,
};

use rustyline::Editor;

#[derive(Debug)]
pub struct RaisedError {
    msg: String,
}

impl RaisedError {
    pub fn new(msg: &str) -> Box<dyn std::error::Error> {
        Box::new(Self {
            msg: String::from(msg),
        })
    }
}

impl std::fmt::Display for RaisedError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for RaisedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        &self.msg
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

pub enum ReadAndParseUserEntryRes<T>
where
    T: FromStr,
{
    Ok(T),
    EmptyEntry,
    IOErr(std::io::Error),
    ParseErr(<T as FromStr>::Err, String),
    ReadErr(rustyline::error::ReadlineError),
}

pub fn read_and_parse_user_entry<T>(msg: &str) -> ReadAndParseUserEntryRes<T>
where
    T: FromStr,
{
    let mut editor = Editor::<()>::new();
    match editor.readline(&format!("{}: ", msg)) {
        Ok(user_entry) => {
            if user_entry.len() == 0 {
                ReadAndParseUserEntryRes::EmptyEntry
            } else {
                match user_entry.parse::<T>() {
                    Ok(e) => ReadAndParseUserEntryRes::Ok(e),
                    Err(e) => ReadAndParseUserEntryRes::ParseErr(e, user_entry),
                }
            }
        }
        Err(e) => ReadAndParseUserEntryRes::ReadErr(e),
    }
}
