use crate::{
    line_printer::LinePrinter,
    main_thread::ProcessorInfo,
    ring_buf_queue::RingBufQConsumer,
    sync_flag::{new_sync_flag, SyncFlagAssassin, SyncFlagVictim},
    user_io::{BoxErr, BoxResult},
};
use serialport::SerialPort;
use std::{
    thread,
    thread::{yield_now, JoinHandle},
};

pub struct SerialConsoleThread<const SIZE: usize> {
    assassin: SyncFlagAssassin,
    join_handle: JoinHandle<BoxResult<()>>,
}

impl<const SIZE: usize> SerialConsoleThread<SIZE> {
    pub fn spawn(
        line_printer: LinePrinter,
        processor_info: &ProcessorInfo,
        write_consumers: Vec<RingBufQConsumer<SIZE, u8>>,
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
                serial_console_task(victim, serial_port, write_consumers, line_printer)
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
    mut write_consumers: Vec<RingBufQConsumer<SIZE, u8>>,
    mut line_printer: LinePrinter,
) -> BoxResult<()> {
    let mut read_buf =  [0u8; SIZE];

    while victim.is_alive() {
        for write_consumer in write_consumers.iter_mut() {
            loop {
                let write_buf = write_consumer.pop();
                if write_buf.is_empty() {
                    break;
                } else {
                    for b in write_buf.iter() {
                        let _ = serial_port.write_all(&[*b]);
                        let dur = core::time::Duration::from_millis(1);
                        thread::sleep(dur);
                    }
                }
            }

            if let Ok(count) = serial_port.read(&mut read_buf) {
                line_printer.push_bytes(&read_buf[..count]);
            }
        }
        yield_now();
    }

    Ok(())
}
