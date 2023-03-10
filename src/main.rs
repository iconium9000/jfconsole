// python -m serial.tools.list_ports -v

// Common Baud Rates:
// 110, 300, 600, 1200, 2400, 4800, 9600, 14400,
// 19200, 38400, 57600, 115200, 128000 and 256000 (we use 3000000)

use futures::{future::select, stream::FuturesUnordered, StreamExt};

use jfconsole::config::{read_config::UserSelectFileRes, Config, ProcessorInfo};
use tokio_serial::SerialStream;

pub const BUFFER_SIZE: usize = 0x1000;
pub const LINE_WIDTH: usize = 800;
pub const BYTE_PROCESS_THREAD_PRIORITY: u8 = 0;
pub const SERIAL_PORT_THREAD_PRIORITY: u8 = 1;
pub const USER_CONSOLE_THREAD_PRIORITY: u8 = 2;
pub const FILE_LOGGER_THREAD_PRIORITY: u8 = 3;

#[tokio::main]
async fn main() {
    println!("Welcome!\n\n");

    let proc_v = ProcessorInfo::available_processors().unwrap();
    if proc_v.is_empty() {
        println!("> [main] No com ports found");
        return;
    }
    let cfg = loop {
        match Config::user_select_file(&proc_v) {
            UserSelectFileRes::Select(cfg) => break cfg,
            UserSelectFileRes::NoConfigs => break Config::user_create_custom(proc_v).unwrap(),
            UserSelectFileRes::SelectCustom => break Config::user_create_custom(proc_v).unwrap(),
            UserSelectFileRes::InvalidEntry => continue,
            UserSelectFileRes::Err(e) => {
                println!("> [main] error {:?}", e);
                return;
            }
        }
    };
    if cfg.processors.is_empty() {
        println!("> [main] no processors in config {:?}", cfg.project_path);
        return;
    }

    let futures = FuturesUnordered::new();
    for proc in cfg.processors {
        let duration = std::time::Duration::from_millis(10);
        let path = proc.port_name.clone();
        let baud_rate = proc.baud_rate;

        futures.push(async move {
            let builder: _ = serialport::new(path, baud_rate).timeout(duration);
            let mut serial_stream = SerialStream::open(&builder);
            let mut buf = [0u8; BUFFER_SIZE];
            while let Ok(ref mut serial_stream) = serial_stream {
                let _ = serial_stream.readable().await.and_then(move |()| {
                    serial_stream.try_read(&mut buf).and_then(|size| {
                        let k = String::from_utf8_lossy(if size < buf.len() {
                            &buf[..size]
                        } else {
                            &buf
                        });
                        println!("{:?}", k);
                        Ok(())
                    })
                });
            }
        })
    }

    let _ = futures.into_future().await;

    println!("> [main] end");
}
