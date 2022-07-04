// python -m serial.tools.list_ports -v

// Common Baud Rates:
// 110,& 300, 600, 1200, 2400, 4800, 9600, 14400, 19200, 38400, 57600, 115200, 128000 and 256000

use serialport::{available_ports, SerialPortType, UsbPortInfo};
use std::{
    cell::Cell,
    collections::HashMap,
    io::{stdin, stdout, Error, ErrorKind, Write},
    rc::Rc,
};

fn other_err(s: &'static str) -> Result<(), Error> {
    return Err(Error::new(ErrorKind::Other, s));
}

pub fn main() {
    get_port_pair().unwrap();
}

struct JfPort {
    port_name: String,
    group_name: String,
    _port: String,
    port_num: usize,
    usb_port_info: UsbPortInfo,
    baudrate: Cell<u32>,
}

impl JfPort {
    fn new(port_name: String, usb_port_info: UsbPortInfo) -> Option<Rc<Self>> {
        let mut i = port_name.len();
        for c in port_name.chars().rev() {
            if c.is_numeric() {
                i -= 1;
            } else {
                break;
            }
        }

        let group_name: String = port_name[..i].into();
        let port: String = port_name[i..].into();
        let port_num: usize;

        if let Ok(num) = port.parse() {
            port_num = num;
        } else {
            return None;
        }

        Some(Rc::new(JfPort {
            port_name,
            usb_port_info,
            group_name,
            _port: port,
            port_num,
            baudrate: Cell::new(115200),
        }))
    }
}

type JfGroupMap = HashMap<String, Vec<Rc<JfPort>>>;
fn get_groups() -> Result<JfGroupMap, Error> {
    let mut groups = JfGroupMap::new();
    for serial_port_info in available_ports()? {
        if let SerialPortType::UsbPort(usb_port_info) = serial_port_info.port_type {
            if let Some(p) = JfPort::new(serial_port_info.port_name, usb_port_info) {
                if let Some(g) = groups.get_mut(&p.group_name) {
                    g.push(p);
                } else {
                    groups.insert(p.group_name.clone(), vec![p]);
                }
            }
        }
    }
    Ok(groups)
}

type JfPair = (Rc<JfPort>, Rc<JfPort>);
type JfPairVec = Vec<JfPair>;
fn get_port_pairs(groups: JfGroupMap) -> Result<JfPairVec, Error> {
    let mut pairs = JfPairVec::new();
    for (group_name, group) in groups.iter() {
        for p1 in group {
            if let Some(p2) = group
                .iter()
                .find(|p2: &&Rc<JfPort>| p2.port_num == p1.port_num + 1)
            {
                pairs.push((Rc::clone(p1), Rc::clone(p2)));
            }
        }
    }
    Ok(pairs)
}

fn enumerate_pairs(pairs: &JfPairVec) {
    for (i, (p1, p2)) in pairs.iter().enumerate() {
        let ref inf = p1.usb_port_info;
        println!(
            "{}. {} {} vid{}:pid{}",
            i + 1,
            p1.port_name,
            p2.port_name,
            inf.vid,
            inf.pid
        );
    }
}

fn select_pairs(pairs: &JfPairVec) -> Result<JfPair, Error> {
    loop {
        println!();
        enumerate_pairs(&pairs);

        print!("Enter index of pair of ports to investigate: ");
        stdout().flush()?;

        let mut select_pair = String::new();
        stdin().read_line(&mut select_pair)?;

        let l = select_pair.len() - 1;
        let ref e = select_pair[..l];

        if let Ok(mut idx) = e.parse::<usize>() {
            idx -= 1;
            if idx < pairs.len() {
                let (p1, p2) = &pairs[idx];
                let ref inf = p1.usb_port_info;
                println!(
                    "> Selected {} {} vid{}:pid{}",
                    p1.port_name, p2.port_name, inf.vid, inf.pid
                );
                return Ok((Rc::clone(p1), Rc::clone(p2)));
            } else {
                println!("invalid index {}", idx + 1);
            }
        } else {
            println!("invalid entry '{}'", e);
        }
    }
}

fn select_baud(p: &JfPort) -> Result<(), Error> {
    let baudrate = loop {
        println!();

        println!("Baud rates for port '{}': ", p.port_name);
        println!("1. 115200");
        println!("2. 3000000");
        println!("_. Custom");

        print!("Enter 1, 2, or a custom value: ");
        stdout().flush()?;

        let mut select_pair = String::new();
        stdin().read_line(&mut select_pair)?;

        let l = select_pair.len() - 1;
        let ref e = select_pair[..l];

        match e.parse::<u32>() {
            Ok(1) => break 115200,
            Ok(2) => break 3000000,
            Ok(a) => break a,
            Err(_) => println!("invalid entry '{}'", e),
        }
    };

    println!("> Selected {} for {}", baudrate, p.port_name);
    p.baudrate.set(baudrate);

    Ok(())
}

fn enter_proj_name() -> Result<String, Error> {
    loop {
        println!();
        print!("Enter project name: ");
        stdout().flush()?;

        let mut select_pair = String::new();
        stdin().read_line(&mut select_pair)?;

        let l = select_pair.len() - 1;
        let s = select_pair[..l].into();
        println!("> Entered New Project Name: '{}'", s);
        break Ok(s);
    }
}

fn get_port_pair() -> Result<(), Error> {
    let groups = get_groups()?;
    let pairs = get_port_pairs(groups)?;

    if pairs.len() == 0 {
        other_err("No avalible port pairs")?;
    }

    let (p1, p2) = select_pairs(&pairs)?;

    select_baud(&p1)?;
    select_baud(&p2)?;
    let _proj = enter_proj_name()?;

    Ok(())
}
