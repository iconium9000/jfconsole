// python -m serial.tools.list_ports -v

// Common Baud Rates:
// 110, 300, 600, 1200, 2400, 4800, 9600, 14400,
// 19200, 38400, 57600, 115200, 128000 and 256000 (we use 3000000)

use jfconsole::Config;

pub fn main() {
    let op = |cfg: Config| cfg.main_task();
    match Config::user_select().and_then(op) {
        Ok(_) => println!("> [main] end"),
        Err(e) => println!("> [main] error {:#?}", e),
    }
}
