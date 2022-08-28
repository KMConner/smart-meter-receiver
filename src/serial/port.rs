use super::traits::ReadWrite;
use crate::serial::buffer::Buffer;
use crate::serial::errors::Result;
use crate::serial::wrapper::Wrapper;
use crate::serial::Connection;
use std::time::Duration;

struct ConnectionImpl<T: ReadWrite> {
    pub(in crate::serial::port) connection: T,
    read_buffer: Buffer,
}

fn trim_line_end(text_u8: &[u8]) -> &[u8] {
    let mut end = 0;
    for i in (0..text_u8.len()).rev() {
        if text_u8[i] != '\r' as u8 && text_u8[i] != '\n' as u8 {
            end = i + 1;
            break;
        }
    }
    &text_u8[0..end]
}

impl<T: ReadWrite> Connection for ConnectionImpl<T> {
    fn write_line(&mut self, line: &str) -> Result<()> {
        let binary = line.as_bytes();
        self.connection.write(binary)?;
        self.connection.write(b"\r\n")?;
        self.connection.flush()?;
        log::trace!("Serial Input: {}", line);
        Ok(())
    }

    fn read_line(&mut self) -> Result<String> {
        let mut txt = Vec::new();
        loop {
            if !self.read_buffer.has_left() {
                let num = self.read_buffer.fill_buf(&mut self.connection)?;
                if num == 0 {
                    continue;
                }
            }
            match self.read_buffer.read_to_lf() {
                Some(bin) => {
                    txt.append(&mut bin.to_vec());
                    let text = String::from_utf8(trim_line_end(&txt).to_vec())?;
                    log::trace!("Serial Output: {}", text);
                    return Ok(text);
                }
                None => match self.read_buffer.get_remain() {
                    Some(rest) => {
                        txt.append(&mut rest.to_vec());
                    }
                    None => continue,
                },
            }
        }
    }
}

pub fn new(path: &str, baud_rate: u32) -> Result<impl Connection> {
    let connection = serialport::new(path, baud_rate)
        .timeout(Duration::from_millis(100))
        .open()?;

    Ok(ConnectionImpl {
        connection: Wrapper::new(connection),
        read_buffer: Buffer::new(128),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serial::mock_serial::MockReadWrite;

    fn new_conn<'a>(
        buf_size: usize,
        read_data: Vec<&'a [u8]>,
    ) -> ConnectionImpl<MockReadWrite<'a>> {
        let mock = MockReadWrite::new(read_data);

        ConnectionImpl {
            connection: mock,
            read_buffer: Buffer::new(buf_size),
        }
    }

    mod read_test {
        use super::*;

        #[test]
        fn read_once() {
            let mut conn = new_conn(16, vec![b"123\r\n456\r\n789\r\n"]);
            assert_eq!(String::from("123"), conn.read_line().unwrap());
        }

        #[test]
        fn read_multiple() {
            let mut conn = new_conn(16, vec![b"123\r\n456\r\n789\r\n"]);
            assert_eq!(String::from("123"), conn.read_line().unwrap());
            assert_eq!(String::from("456"), conn.read_line().unwrap());
            assert_eq!(String::from("789"), conn.read_line().unwrap());
        }

        #[test]
        fn read_again_when_multiple() {
            let mut conn = new_conn(16, vec![b"", b"", b"123\r\n456\r\n789\r\n"]);
            assert_eq!(String::from("123"), conn.read_line().unwrap());
            assert_eq!(String::from("456"), conn.read_line().unwrap());
            assert_eq!(String::from("789"), conn.read_line().unwrap());
        }

        #[test]
        fn read_concat() {
            let mut conn = new_conn(16, vec![b"12", b"3\r\n456\r\n789\r\n"]);
            assert_eq!(String::from("123"), conn.read_line().unwrap());
            assert_eq!(String::from("456"), conn.read_line().unwrap());
            assert_eq!(String::from("789"), conn.read_line().unwrap());
        }

        #[test]
        fn read_concat_multi() {
            let mut conn = new_conn(16, vec![b"1", b"2", b"3\r\n456\r\n"]);
            assert_eq!(String::from("123"), conn.read_line().unwrap());
            assert_eq!(String::from("456"), conn.read_line().unwrap());
        }

        #[test]
        fn split_cr_lf() {
            let mut conn = new_conn(16, vec![b"12", b"3\r", b"\n456", b"\r\n"]);
            assert_eq!(String::from("123"), conn.read_line().unwrap());
            assert_eq!(String::from("456"), conn.read_line().unwrap());
        }
    }

    mod write_test {
        use super::*;

        #[test]
        fn begin_empty() {
            let conn = new_conn(0, Vec::new());
            assert_eq!(0, conn.connection.write_buf.len());
        }

        #[test]
        fn write_once() {
            let mut conn = new_conn(0, Vec::new());
            conn.write_line("abc").unwrap();
            assert_eq!(b"abc\r\n".to_vec(), conn.connection.write_buf);
        }

        #[test]
        fn write_many() {
            let mut conn = new_conn(0, Vec::new());
            conn.write_line("abc").unwrap();
            conn.write_line("def").unwrap();
            conn.write_line("ghi").unwrap();
            assert_eq!(b"abc\r\ndef\r\nghi\r\n".to_vec(), conn.connection.write_buf);
        }
    }

    mod trim_line_end_test {
        use super::*;

        #[test]
        fn empty() {
            assert_eq!(b"", trim_line_end(b""));
        }

        #[test]
        fn cr_lf_only() {
            assert_eq!(b"", trim_line_end(b"\r\n"));
        }

        #[test]
        fn multi_cr_lf_only() {
            assert_eq!(b"", trim_line_end(b"\r\n\r\n"));
        }

        #[test]
        fn text_ends_with_lf() {
            assert_eq!(b"foobar", trim_line_end(b"foobar\n"));
        }

        #[test]
        fn text_ends_with_cr_lf() {
            assert_eq!(b"foobar", trim_line_end(b"foobar\r\n"));
        }

        #[test]
        fn inner_lf() {
            assert_eq!(b"foo\nbar", trim_line_end(b"foo\nbar"));
        }

        #[test]
        fn inner_and_end_lf() {
            assert_eq!(b"foo\r\nbar", trim_line_end(b"foo\r\nbar\r\n"));
        }
    }
}
