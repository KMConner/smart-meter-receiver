mod echonet;
mod parser;
mod serial;
mod wisun_module;

use crate::wisun_module::WiSunClient;
use std::env;
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )]).unwrap();
    let conn = serial::new("/dev/ttyS0", 115200).unwrap();
    let mut cli = WiSunClient::new(conn).unwrap();
    let version = cli.get_version().unwrap();
    println!("Version: {}", version);
    let bid = env::var("WISUN_BID").expect("BID MUST BE specified with WISUN_BID");
    let password = env::var("WISUN_PASSWORD").expect("Password MUST BE specified with WISUN_PASSWORD");
    cli.connect(bid.as_str(), password.as_str()).unwrap();
}
