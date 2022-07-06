use crate::{
    byte_process_thread::Msg,
    main_thread::{set_thread_priority, ProcessorInfo, SERIAL_PORT_THREAD_PRIORITY},
};
use chrono::Utc;
use serialport::SerialPort;
use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::{self, JoinHandle},
};

pub enum WriteBuf {
    Buf(Box<[u8]>),
    Exit,
}

pub struct SerialConsoleThread {
    write_sender: Sender<WriteBuf>,
    join_handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send>>>,
}

impl SerialConsoleThread {
    pub fn join(self) {
        let _ = self.write_sender.send(WriteBuf::Exit);
        let _ = self.join_handle.join();
    }
    pub fn write_sender(&self) -> Sender<WriteBuf> {
        self.write_sender.clone()
    }
}

impl SerialConsoleThread {
    pub fn spawn(
        processor_info: &ProcessorInfo,
        processor_idx: usize,
        msg_sender: &Sender<Msg>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let duration = std::time::Duration::from_millis(10);
        let path = processor_info.port_name.clone();
        let baud_rate = processor_info.baudrate;
        let builder = serialport::new(path, baud_rate).timeout(duration);
        let serial_port = builder.open()?;
        let processor_name = processor_info.processor_name.clone();
        let msg_sender = msg_sender.clone();
        let (write_sender, write_receiver) = channel();

        Ok(SerialConsoleThread {
            write_sender: write_sender.clone(),
            join_handle: thread::spawn(move || {
                serial_port_task(
                    serial_port,
                    processor_idx,
                    processor_name,
                    msg_sender,
                    write_receiver,
                )
            }),
        })
    }
}

pub fn serial_port_task(
    mut serial_port: Box<dyn SerialPort>,
    processor_idx: usize,
    processor_name: String,
    msg_sender: Sender<Msg>,
    write_receiver: Receiver<WriteBuf>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    set_thread_priority::<SERIAL_PORT_THREAD_PRIORITY>();

    println!("> [serial_port_task] {} start", processor_name);
    let mut readbuf = [0u8; 0x1000];
    loop {
        let instant = Utc::now();
        loop {
            match serial_port.read(&mut readbuf) {
                Ok(0) => break,
                Ok(count) => {
                    let _ = msg_sender.send(Msg::Read {
                        processor_idx,
                        instant,
                        bytes: Box::from(&readbuf[..count]),
                    });
                }
                Err(e) => {
                    // println!("> [serial_port_task] {} read error {:?}", processor_name, e);
                    let _ = e; // always fails in first few seconds
                    break;
                }
            }
        }
        loop {
            match write_receiver.try_recv() {
                Ok(WriteBuf::Exit) => {
                    return Ok(println!("> [serial_port_task] {} end", processor_name));
                }
                Ok(WriteBuf::Buf(msg)) => {
                    let e = serial_port.write(&msg);
                    if e.is_err() {
                        println!(
                            "> [serial_port_task] {} write error {:?}",
                            processor_name, e
                        );
                    }
                }
                _ => break,
            }
        }
    }
}
