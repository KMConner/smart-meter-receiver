use crate::serial::errors::Result;

pub trait Connection {
    fn write_line(&mut self, line: &str) -> Result<()>;
    fn read_line(&mut self) -> Result<String>;
}
