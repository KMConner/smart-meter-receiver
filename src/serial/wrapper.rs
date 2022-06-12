use super::traits::ReadWrite;
use serialport::SerialPort;
use std::io::{Read, Result, Write};

pub struct Wrapper {
    port: Box<dyn SerialPort>,
}

impl ReadWrite for Wrapper {}

impl Read for Wrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.port.read(buf)
    }
}

impl Write for Wrapper {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.port.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.port.flush()
    }
}

impl Wrapper {
    pub fn new(conn: Box<dyn SerialPort>) -> Wrapper {
        Wrapper { port: conn }
    }
}
