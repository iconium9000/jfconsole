use crate::{
    buf_iter::RingBufQConsumer,
    line_printer::LinePrinter,
    main_thread::ProcessorInfo,
    user_io::{BoxErr, BoxResult}, sync_flag::{SyncFlagAssassin, new_sync_flag, SyncFlagVictim},
};
use serialport::SerialPort;
use std::{
    thread,
    thread::JoinHandle,
};

pub struct SerialConsoleThread<const SIZE: usize> {
    assassin: SyncFlagAssassin,
    join_handle: JoinHandle<BoxResult<()>>,
}

impl<const SIZE: usize> SerialConsoleThread<SIZE> {
    pub fn spawn(
        line_printer: LinePrinter,
        processor_info: &ProcessorInfo,
        write_consumer: RingBufQConsumer<SIZE, u8>,
    ) -> BoxResult<Self> {
        let duration = std::time::Duration::from_millis(10);
        let path = processor_info.port_name.clone();
        let baud_rate = processor_info.baudrate;
        let builder = serialport::new(path, baud_rate).timeout(duration);
        let serial_port = builder.open().box_err()?;
        let (victim, assassin) = new_sync_flag();

        Ok(Self {
            assassin,
            join_handle: thread::spawn(move || {
                serial_console_task(victim, serial_port, write_consumer, line_printer)
            }),
        })
    }
    pub fn join(self) -> BoxResult<()> {
        self.assassin.kill_victim();
        self.join_handle.join()?
    }
}

fn serial_console_task<const SIZE: usize>(
    victim: SyncFlagVictim,
    mut serial_port: Box<dyn SerialPort>,
    mut write_consumer: RingBufQConsumer<SIZE, u8>,
    mut line_printer: LinePrinter,
) -> BoxResult<()> {
    let ref mut read_buf = [0u8; SIZE];

    while victim.is_alive() {
        loop {
            let write_buf = write_consumer.pop();
            if write_buf.is_empty() {
                break;
            } else {
                let _ = serial_port.write_all(&write_buf);
            }
        }

        if let Ok(count) = serial_port.read(read_buf) {
            line_printer.push_bytes(&read_buf[..count]);
        }
    }

    Ok(())
}
