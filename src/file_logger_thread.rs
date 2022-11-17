use std::{
    fs::{create_dir, File, OpenOptions},
    io::Write,
    path::Path,
    sync::mpsc::Receiver,
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::Utc;

use crate::{
    sync_flag::{new_sync_flag, SyncFlagAssassin, SyncFlagVictim},
    user_io::BoxResult,
};

pub struct FileLoggerThread {
    assasin: SyncFlagAssassin,
    join_handle: JoinHandle<BoxResult<()>>,
}

impl FileLoggerThread {
    pub fn spawn(
        project_name: &str,
        line_receiver: Receiver<String>,
        main_thread_assassin: SyncFlagAssassin,
    ) -> BoxResult<Self> {
        let path = Path::new(project_name);
        let _ = create_dir(path);

        let fmt = "%y%m%d_%H%M%S";
        let now = Utc::now();
        let file_name = format!("{}_{}.log", project_name, now.format(fmt));
        let file_path = path.join(Path::new(&file_name));
        let file = match OpenOptions::new()
            .create(true)
            .write(true)
            .open(file_path.clone())
        {
            Ok(opened_file) => {
                println!("> [file_logger_task] opened {:?}", file_path);
                opened_file
            }
            Err(e) => {
                println!("> [file_logger_task] error {:?}", e);
                return Err(Box::new(e));
            }
        };

        let (victim, assassin) = new_sync_flag();
        Ok(Self {
            assasin: assassin,
            join_handle: thread::spawn(move || {
                file_logger_task(victim, file, line_receiver, main_thread_assassin)
            }),
        })
    }

    pub fn join(self) -> BoxResult<()> {
        self.assasin.kill_victim();
        self.join_handle.join()?
    }
}

fn file_logger_task(
    victim: SyncFlagVictim,
    mut file: File,
    line_receiver: Receiver<String>,
    main_thread_assassin: SyncFlagAssassin,
) -> BoxResult<()> {
    let mut synced = true;
    Ok(while victim.is_alive() {
        let duration = Duration::from_millis(100);
        if let Ok(mut line) = line_receiver.recv_timeout(duration) {
            line.push('\n');
            if let Err(e) = file.write_all(line.as_bytes()) {
                println!("> [file_logger_task] write error {:#?}", e);
                main_thread_assassin.kill_victim();
                return Err(Box::new(e));
            } else {
                synced = false;
            }
        } else if synced {
        } else if let Err(e) = file.sync_all() {
            println!("> [file_logger_task] sync error {:#?}", e);
            main_thread_assassin.kill_victim();
            return Err(Box::new(e));
        } else {
            synced = true;
        }
    })
}
