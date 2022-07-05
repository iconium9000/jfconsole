use std::{
    io::{stdin, stdout, Error as IOError, Write},
    str::FromStr,
};

#[macro_export]
macro_rules! raise_ioerr {
    ($msg:expr) => {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, $msg))
    };
}

pub fn read_user_entry(msg: &str) -> Result<String, IOError> {
    let mut user_entry = String::new();
    print!("{}: ", msg);
    stdout().flush()?;
    stdin().read_line(&mut user_entry)?;
    let l = user_entry.len() - 1;
    Ok(String::from(&user_entry[..l]))
}

pub enum ReadAndParseUserEntryRes<T>
where
    T: FromStr,
{
    Ok(T),
    EmptyEntry,
    IOErr(IOError),
    ParseErr(<T as FromStr>::Err),
}

pub fn read_and_parse_user_entry<T>(msg: &str) -> ReadAndParseUserEntryRes<T>
where
    T: FromStr,
{
    let mut user_entry = String::new();
    print!("{}: ", msg);
    if let Err(e) = stdout().flush() {
        return ReadAndParseUserEntryRes::IOErr(e);
    }
    if let Err(e) = stdin().read_line(&mut user_entry) {
        return ReadAndParseUserEntryRes::IOErr(e);
    }

    let l = user_entry.len() - 1;
    if l == 0 {
        return ReadAndParseUserEntryRes::EmptyEntry;
    }
    let ref entry = user_entry[..l];
    match entry.parse::<T>() {
        Ok(e) => ReadAndParseUserEntryRes::Ok(e),
        Err(e) => {
            println!("> Invalid Entry {}\n", entry);
            ReadAndParseUserEntryRes::ParseErr(e)
        }
    }
}
