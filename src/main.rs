// python -m serial.tools.list_ports -v

// Common Baud Rates:
// 110, 300, 600, 1200, 2400, 4800, 9600, 14400,
// 19200, 38400, 57600, 115200, 128000 and 256000 (we use 3000000)

use jfconsole::threads::main_thread::main_task;

pub fn main() {
    main_task()
}
