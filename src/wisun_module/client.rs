use crate::serial::Connection;

pub struct WiSunCLient {
    serial_connection: Connection,
}

impl WiSunCLient {
    fn new(serial_connection: Connection) -> Self {
        WiSunCLient {
            serial_connection: serial_connection,
        }
    }

    fn ensure_echoback_off() -> Result<()> {}
}
