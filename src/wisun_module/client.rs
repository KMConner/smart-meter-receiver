use crate::serial::{Connection, Error as SerialError};
use crate::wisun_module::errors::{Error, Result};
use std::io::{ErrorKind as IoErrorKind};
use crate::parser::{Parser, ParseResult, SerialMessage, WiSunEvent, WiSunModuleParser};

pub struct WiSunClient<T: Connection, S: Parser> {
    serial_connection: T,
    serial_parser: S,
    message_buffer: Vec<SerialMessage>,
}

impl<T: Connection> WiSunClient<T, WiSunModuleParser> {
    pub fn new(serial_connection: T) -> Result<Self> {
        let mut client = WiSunClient {
            serial_connection,
            serial_parser: WiSunModuleParser::new(),
            message_buffer: Vec::new(),
        };
        client.ensure_echoback_off()?;
        Ok(client)
    }
}

impl<T: Connection, S: Parser> WiSunClient<T, S> {
    fn get_message(&mut self) -> Result<bool> {
        loop {
            match self.serial_connection.read_line() {
                Ok(line) => {
                    match self.serial_parser.add_line(line.as_str()) {
                        ParseResult::Ok(m) => {
                            self.message_buffer.push(m);
                            return Ok(true);
                        }
                        ParseResult::Empty => {
                            return Ok(false);
                        }
                        ParseResult::More => {
                            continue;
                        }
                        ParseResult::Err(s) => {
                            // TODO: logging
                            continue;
                        }
                    }
                }
                Err(SerialError::IoError(ioe)) => {
                    if ioe.kind() == IoErrorKind::TimedOut {
                        continue;
                    }
                    return Err(Error::SerialError(SerialError::IoError(ioe)));
                }
                Err(e) => {
                    return Err(Error::SerialError(e));
                }
            }
        }
    }

    pub fn flush_messages(&mut self) {
        self.message_buffer.clear();
    }

    fn wait_fn<F, H>(&mut self, pred: F, err_if: H) -> Result<SerialMessage>
        where F: Fn(&SerialMessage) -> bool, H: Fn(&SerialMessage) -> Option<String> {

        // Search on message_buffer
        let mut delete_idx = usize::MAX;
        for i in 0..self.message_buffer.len() {
            if let Some(m) = self.message_buffer.get(i) {
                if pred(m) {
                    delete_idx = i;
                    break;
                }
            }
        }
        if delete_idx < usize::MAX {
            return Ok(self.message_buffer.remove(delete_idx));
        }

        // get new message from console
        loop {
            if self.get_message()? {
                if let Some(m) = self.message_buffer.last() {
                    if pred(m) {
                        return Ok(self.message_buffer.remove(self.message_buffer.len() - 1));
                    }
                    if let Some(e) = err_if(m) {
                        return Err(Error::CommandError(e));
                    }
                }
            }
        }
    }

    fn wait_ok(&mut self) -> Result<()> {
        let result = self.wait_fn(|m| *m == SerialMessage::Ok, err_when_fail)?;
        Ok(())
    }

    fn ensure_echoback_off(&mut self) -> Result<()> {
        self.serial_connection.write_line("SKSREG SFE 0")?;
        self.wait_ok()
    }

    fn get_version(&mut self) -> Result<String> {
        self.serial_connection.write_line("SKVER")?;
        self.wait_ok()?;
        let msg = self.wait_fn(|m| -> bool{
            match m {
                SerialMessage::Event(WiSunEvent::Version(_)) => true,
                _ => false,
            }
        }, err_when_fail)?;
        if let SerialMessage::Event(WiSunEvent::Version(ver)) = msg {
            return Ok(ver);
        }
        Err(Error::CommandError("Unexpected msg".to_string()))
    }
}

fn err_when_fail(m: &SerialMessage) -> Option<String> {
    match m {
        SerialMessage::Fail(s) => Some(s.clone()),
        _ => None
    }
}

#[cfg(test)]
mod test {
    use super::WiSunClient;
    use crate::wisun_module::mock::{MockSerialParser, MockSerial};

    fn new_client<F>(mut prepare_mock: F) -> WiSunClient<MockSerial, MockSerialParser>
        where
            F: FnMut(&mut MockSerial, &mut MockSerialParser),
    {
        let mut mock_serial = MockSerial::new();
        let mut mock_parser = MockSerialParser::new();
        prepare_mock(&mut mock_serial, &mut mock_parser);
        WiSunClient {
            serial_connection: mock_serial,
            serial_parser: mock_parser,
            message_buffer: Vec::new(),
        }
    }

    mod wait_ok_test {
        use super::*;
        use crate::serial::Error as SerialError;
        use mockall::Sequence;
        use std::io::{Error as IoError, ErrorKind as IoErrorKind};
        use crate::parser::{ParseResult, SerialMessage};

        #[test]
        fn ok_when_read_ok() {
            let mut cli = new_client(|s, p| -> () {
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
                p.expect_add_line()
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Ok));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn read_again_when_not_ok() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s, p| -> () {
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));
                p.expect_add_line()
                    .times(1)
                    .returning(|_| ParseResult::Err(String::from("SKVER")));
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));
                p.expect_add_line()
                    .times(1)
                    .returning(|_| ParseResult::Err(String::from("SKVER")));

                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("OK")));
                p.expect_add_line()
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Ok));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn read_again_when_timeout() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s, p| -> () {
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| {
                        Err(SerialError::IoError(IoError::new(
                            IoErrorKind::TimedOut,
                            "timeout",
                        )))
                    });
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("OK")));
                p.expect_add_line()
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Ok));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn error_when_fail() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s, p| -> () {
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));
                p.expect_add_line()
                    .times(1)
                    .returning(|_| ParseResult::Err(String::from("SKVER")));

                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("FAIL ER04")));
                p.expect_add_line()
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Fail("ER04".to_string())));
            });
            assert_eq!(cli.wait_ok().is_err(), true);
        }
    }

    mod get_version_test {
        use super::*;
        use mockall::{predicate, Sequence};
        use crate::parser::{ParseResult, SerialMessage, WiSunEvent};

        #[test]
        fn ok_before_ever() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s, p| -> () {
                s.expect_write_line()
                    .with(predicate::eq("SKVER"))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|_| Ok(()));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
                p.expect_add_line()
                    .with(predicate::eq("OK"))
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Ok));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("EVER 1.2.3")));
                p.expect_add_line()
                    .with(predicate::eq("EVER 1.2.3"))
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Event(WiSunEvent::Version("1.2.3".to_string()))));
            });
            let ver = cli.get_version().unwrap();
            assert_eq!(ver, "1.2.3".to_string());
        }

        #[test]
        fn ever_before_ok() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s, p| -> () {
                s.expect_write_line()
                    .with(predicate::eq("SKVER"))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|_| Ok(()));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("EVER 2.3.4")));
                p.expect_add_line()
                    .with(predicate::eq("EVER 2.3.4"))
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Event(WiSunEvent::Version("2.3.4".to_string()))));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
                p.expect_add_line()
                    .with(predicate::eq("OK"))
                    .times(1)
                    .returning(|_| ParseResult::Ok(SerialMessage::Ok));
            });
            let ver = cli.get_version().unwrap();
            assert_eq!(ver, "2.3.4".to_string());
        }
    }
}
