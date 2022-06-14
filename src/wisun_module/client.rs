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

}
#[cfg(test)]
mod test {
    use super::WiSunCLient;
    use crate::wisun_module::mock::MockSerial;

    fn new_client<F>(prepare_mock: F) -> WiSunCLient<MockSerial>
    where
        F: Fn(&mut MockSerial),
    {
        let mut mock = MockSerial::new();
        prepare_mock(&mut mock);
        WiSunCLient {
            serial_connection: mock,
        }
    }
}
