use std::{
    any::Any,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use serialport::SerialPort;

use crate::{
    main_thread::{BoxErr, ProcessorInfo},
    user_console_thread::ProcFmt,
};

pub struct SerialConsoleThread {
    proc_fmt: ProcFmt,
    join_handle: JoinHandle<()>,
    alive: Arc<AtomicBool>,
}

impl SerialConsoleThread {
    pub fn spawn(proc_fmt: ProcFmt, info: &ProcessorInfo) -> Result<Self, Box<dyn Any + Send>> {
        let alive = Arc::new(AtomicBool::new(true));

        let duration = std::time::Duration::from_millis(10);
        let path = info.port_name.clone();
        let baud_rate = info.baudrate;

        let serial_port = serialport::new(path, baud_rate)
            .timeout(duration)
            .open()
            .box_err()?;

        let thread_alive = Arc::clone(&alive);
        let join_handle = thread::spawn(|| {
            serial_console_task(thread_alive, serial_port);
        });

        Ok(Self {
            proc_fmt,
            join_handle,
            alive,
        })
    }
    pub fn join(self) {
        self.alive.store(false, Ordering::Relaxed);
        let _ = self.join_handle.join();
    }
}

fn serial_console_task(alive: Arc<AtomicBool>, mut serial_port: Box<dyn SerialPort>) {
    let mut working_buf = [0u8; 0x1000];
    const NL: u8 = 0xa; // '\n'
    const CR: u8 = 0xd; // '\r'
    let mut last_bit = 0x0;
    let pred = move |b: &u8| {
        let p = last_bit;
        last_bit = *b;
        return match (p, *b) {
            (NL, CR) | (CR, NL) => false,
            (NL, NL) | (CR, CR) => true,
            (_, CR) | (_, NL) => true,
            (CR, _) | (NL, _) => false,
            (_, _) => false
        }
    };

    while alive.load(Ordering::Relaxed) {
        if let Ok(n_bytes) = serial_port.read(&mut working_buf) {
            let slice = &working_buf[..n_bytes];
            slice.split(pred);
        }
    }
}
