use crate::serial::errors::Result;
use std::io::{Read, Write};

pub trait ReadWrite: Read + Write {}

pub trait Connection {
    fn write_line(&mut self, line: &str) -> Result<()>;
    fn read_line(&mut self) -> Result<String>;
}
