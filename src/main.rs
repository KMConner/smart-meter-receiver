mod serial;
mod parser;
mod wisun_module;

use crate::wisun_module::WiSunClient;

fn main() {
    let conn = serial::new("/dev/ttyS0", 115200).unwrap();
    let mut cli = WiSunClient::new(conn).unwrap();
    let version = cli.get_version().unwrap();
    println!("Version: {}", version);
}
