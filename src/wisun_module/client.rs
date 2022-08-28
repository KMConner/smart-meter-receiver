use crate::serial::{Connection, Error as SerialError};
use crate::wisun_module::errors::{Error, Result};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

pub struct WiSunClient<T: Connection> {
    serial_connection: T,
}

impl<T: Connection> WiSunClient<T> {
    pub fn new(serial_connection: T) -> Result<Self> {
        let mut client = WiSunClient {
            serial_connection: serial_connection,
        };
        client.ensure_echoback_off()?;
        Ok(client)
    }

    fn wait_ok(&mut self) -> Result<()> {
        loop {
            match self.serial_connection.read_line() {
                Ok(line) => {
                    if line == "OK" {
                        return Ok(());
                    }
                    if line.starts_with("FAIL") {
                        return Err(Error::CommandError(line));
                    }
                }
                Err(SerialError::IoError(ioe)) => {
                    if ioe.kind() == IoErrorKind::TimedOut {
                        continue;
                    }
                    return Err(Error::SerialError(SerialError::IoError(ioe)));
                }
                Err(e) => {
                    return Err(crate::wisun_module::errors::Error::SerialError(e));
                }
            }
        }
    }

    fn ensure_echoback_off(&mut self) -> Result<()> {
        self.serial_connection.write_line("SKSREG SFE 0")?;
        self.wait_ok()
    }
}

#[cfg(test)]
mod test {
    use super::WiSunClient;
    use crate::wisun_module::mock::MockSerial;

    fn new_client<F>(mut prepare_mock: F) -> WiSunClient<MockSerial>
    where
        F: FnMut(&mut MockSerial),
    {
        let mut mock = MockSerial::new();
        prepare_mock(&mut mock);
        WiSunClient {
            serial_connection: mock,
        }
    }

    mod wait_ok_test {
        use super::*;
        use crate::serial::Error as SerialError;
        use mockall::Sequence;
        use std::io::{Error as IoError, ErrorKind as IoErrorKind};

        #[test]
        fn ok_when_read_ok() {
            let mut cli = new_client(|mock| -> () {
                mock.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn read_again_when_not_ok() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|mock| -> () {
                mock.expect_read_line()
                    .times(2)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));
                mock.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("OK")));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn read_again_when_timeout() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|mock| -> () {
                mock.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| {
                        Err(SerialError::IoError(IoError::new(
                            IoErrorKind::TimedOut,
                            "timeout",
                        )))
                    });
                mock.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("OK")));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn error_when_fail() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|mock| -> () {
                mock.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));
                mock.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("FAIL ER04")));
            });
            assert_eq!(cli.wait_ok().is_err(), true);
        }
    }

    mod ensure_echoback_off_test {
        use super::*;
        use mockall::{predicate, Sequence};

        #[test]
        fn ok() {
            let mut seq = Sequence::new();

            let mut cli = new_client(|mock| -> () {
                mock.expect_write_line()
                    .with(predicate::eq("SKSREG SFE 0"))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|_| Ok(()));
                mock.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("OK")));
            });
            cli.ensure_echoback_off().unwrap();
        }
    }
}