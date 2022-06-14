use crate::serial::Connection;
use crate::wisun_module::errors::Result;

pub struct WiSunCLient<T: Connection> {
    serial_connection: T,
}

impl<T: Connection> WiSunCLient<T> {
    fn new(serial_connection: T) -> Self {
        WiSunCLient {
            serial_connection: serial_connection,
        }
    }

    fn ensure_echoback_off() -> Result<()> {}
}
