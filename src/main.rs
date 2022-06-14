mod serial;
mod wisun_module;
use serial::Connection;

fn main() {
    let mut conn = serial::new("/dev/ttyS0", 115200).unwrap();
    let line = String::from("SKVER");
    conn.write_line(&line).unwrap();

    let l: String = conn.read_line().unwrap();
    println!("{:?}", l);

    let l = conn.read_line().unwrap();
    println!("{:?}", l);

    let l = conn.read_line().unwrap();
    println!("{:?}", l);

    let l = conn.read_line().unwrap();
    println!("{:?}", l);
}
