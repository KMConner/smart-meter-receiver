use crate::serial::traits::ReadWrite;
use std::io::{Read, Write};

#[cfg(test)]
pub struct MockReadWrite<'a> {
    read_buf: Vec<&'a [u8]>,
    pub write_buf: Vec<u8>,
    pointer: usize,
}

#[cfg(test)]
pub fn new_mock<'a>(data: Vec<&'a [u8]>) -> MockReadWrite<'a> {
    MockReadWrite {
        read_buf: data,
        write_buf: Vec::new(),
        pointer: 0,
    }
}

#[cfg(test)]
impl<'a> Read for MockReadWrite<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        if self.read_buf.len() <= self.pointer {
            return Ok(0);
        }
        let data = self.read_buf[self.pointer];
        for i in 0..data.len() {
            buf[i] = data[i];
        }
        self.pointer += 1;
        Ok(data.len())
    }
}

#[cfg(test)]
impl<'a> Write for MockReadWrite<'a> {
    fn write(&mut self, bin: &[u8]) -> std::result::Result<usize, std::io::Error> {
        if bin.len() == 0 {
            return Ok(0);
        }

        if self.write_buf.capacity() < self.write_buf.len() + bin.len() {
            self.write_buf.reserve(bin.len());
        }

        for b in bin {
            self.write_buf.push(*b);
        }

        Ok(bin.len())
    }

    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        // Do nothing
        Ok(())
    }
}

#[cfg(test)]
impl<'a> ReadWrite for MockReadWrite<'a> {}

#[cfg(test)]
mod test {
    use super::*;

    mod read_test {
        use super::*;
        #[test]
        fn read_empty() {
            let mut mock = new_mock(Vec::new());
            let mut buf = vec![0u8; 128];
            assert_eq!(0, mock.read(&mut buf).unwrap());
            assert_eq!([0; 128].to_vec(), buf.to_vec());
        }

        #[test]
        fn read_once() {
            let mut mock = new_mock(vec![b"abc", b"123"]);
            let mut buf = vec![0u8; 4];
            assert_eq!(3, mock.read(&mut buf).unwrap());
            assert_eq!(b"abc\0".to_vec(), buf.to_vec());
        }

        #[test]
        fn read_all() {
            let mut mock = new_mock(vec![b"abc", b"123"]);
            let mut buf = vec![0u8; 4];
            let n = mock.read(&mut buf).unwrap();
            assert_eq!(3, n);
            assert_eq!(b"abc".to_vec(), buf[..n].to_vec());

            let n = mock.read(&mut buf).unwrap();
            assert_eq!(3, n);
            assert_eq!(b"123".to_vec(), buf[..n].to_vec());

            let n = mock.read(&mut buf).unwrap();
            assert_eq!(0, n);
        }
    }

    mod write_test {
        use super::*;
        #[test]
        fn begin_empty() {
            let mock = new_mock(Vec::new());
            assert_eq!(0, mock.write_buf.len());
        }

        #[test]
        fn write_nothing() {
            let mut mock = new_mock(Vec::new());
            assert_eq!(0, mock.write(b"").unwrap());
            assert_eq!(b"".to_vec(), mock.write_buf);
        }

        #[test]
        fn write_once() {
            let mut mock = new_mock(Vec::new());
            assert_eq!(3, mock.write(b"abc").unwrap());
            assert_eq!(b"abc".to_vec(), mock.write_buf);
        }

        #[test]
        fn write_multiple() {
            let mut mock = new_mock(Vec::new());

            assert_eq!(3, mock.write(b"abc").unwrap());
            assert_eq!(3, mock.write(b"123").unwrap());
            assert_eq!(4, mock.write(b"ABC\n").unwrap());

            assert_eq!(b"abc123ABC\n".to_vec(), mock.write_buf);
        }
    }

    mod flush_test {
        use super::*;
        #[test]
        fn always_ok() {
            let mut mock = new_mock(Vec::new());
            assert_eq!(true, mock.flush().is_ok())
        }
    }
}
