use crate::serial::Error as SerialError;
use std::io::Read;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum BufError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("this buffer has data left")]
    DataLeftError,
}

impl From<BufError> for SerialError {
    fn from(err: BufError) -> Self {
        match err {
            BufError::IoError(e) => SerialError::IoError(e),
            BufError::DataLeftError => {
                SerialError::UnknownError(String::from("this buffer has data left"))
            }
        }
    }
}

pub struct Buffer {
    pub(self) data: Vec<u8>,
    pub(self) pointer: usize,
    pub(self) end: usize,
}

impl Buffer {
    pub fn new(buf_size: usize) -> Buffer {
        Buffer {
            data: vec![0u8; buf_size],
            pointer: 0,
            end: 0,
        }
    }

    pub fn has_left(&self) -> bool {
        self.pointer < self.end
    }

    pub fn fill_buf<R: Read>(&mut self, reader: &mut R) -> Result<usize, BufError> {
        if self.has_left() {
            return Err(BufError::DataLeftError);
        }

        let result = reader.read(&mut self.data)?;
        self.pointer = 0;
        self.end = result;
        Ok(result)
    }

    pub fn read_to_lf(&mut self) -> Option<&[u8]> {
        if !self.has_left() {
            return None;
        }

        for i in self.pointer..self.end {
            if self.data[i] == '\n' as u8 {
                let pointer = self.pointer;
                self.pointer = i + 1;
                return Some(&self.data[pointer..self.pointer]);
            }
        }

        None
    }

    pub fn get_remain(&mut self) -> Option<&[u8]> {
        if !self.has_left() {
            return None;
        }
        let begin = self.pointer;
        self.pointer = self.end;
        Some(&self.data[begin..self.end])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod fill_test {
        use super::*;
        use crate::serial::mock_serial::MockReadWrite;

        #[test]
        fn read_empty() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(Vec::new());
            let result = b.fill_buf(&mut m).unwrap();
            assert_eq!(0, result);
            assert_eq!(0, b.pointer);
            assert_eq!(0, b.end);
        }

        #[test]
        fn read_once() {
            let mut b = Buffer::new(8);
            let mut m = MockReadWrite::new(vec![b"abcd", b"egfh"]);
            let result = b.fill_buf(&mut m).unwrap();
            assert_eq!(4, result);
            assert_eq!(0, b.pointer);
            assert_eq!(4, b.end);
            assert_eq!(b"abcd", &b.data[0..4]);
        }

        #[test]
        fn error_read_when_data_left() {
            let mut b = Buffer::new(8);
            let mut m = MockReadWrite::new(vec![b"abcd", b"egfh"]);

            let result = b.fill_buf(&mut m).unwrap();
            assert_eq!(4, result);
            assert_eq!(0, b.pointer);
            assert_eq!(4, b.end);
            assert_eq!(b"abcd", &b.data[0..4]);

            let result = b.fill_buf(&mut m);
            assert_eq!(true, result.is_err());
        }
    }

    mod read_to_lf_test {
        use super::*;
        use crate::serial::mock_serial::MockReadWrite;

        #[test]
        fn none_when_empty() {
            let mut b = Buffer::new(8);
            assert_eq!(true, b.read_to_lf().is_none());
        }

        #[test]
        fn none_without_lf() {
            let mut b = Buffer::new(8);
            let mut m = MockReadWrite::new(vec![b"abcdegfh"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(true, b.read_to_lf().is_none());
        }

        #[test]
        fn read_once() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\ndegf\r\nh\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
        }

        #[test]
        fn read_multiple() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\ndefg\r\nijkl"]);
            b.fill_buf(&mut m).unwrap();
            assert_eq!(15, b.end);

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(b"defg\r\n", b.read_to_lf().unwrap());
            assert_eq!(11, b.pointer);
        }

        #[test]
        fn read_ends_with_lf() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\ndefg\r\nij\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(b"defg\r\n", b.read_to_lf().unwrap());
            assert_eq!(11, b.pointer);
            assert_eq!(b"ij\r\n", b.read_to_lf().unwrap());
        }

        #[test]
        fn none_after_read_all() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(true, b.read_to_lf().is_none());
        }

        #[test]
        fn consequtive_cr_lf() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\n\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(b"\r\n", b.read_to_lf().unwrap());
            assert_eq!(7, b.pointer);

            assert_eq!(true, b.read_to_lf().is_none());
        }

        #[test]
        fn fill_multiple() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\ndef\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(b"def\r\n", b.read_to_lf().unwrap());
            assert_eq!(10, b.pointer);

            assert_eq!(true, b.read_to_lf().is_none());

            let mut m = MockReadWrite::new(vec![b"123\r\n"]);
            b.fill_buf(&mut m).unwrap();
            assert_eq!(b"123\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(true, b.read_to_lf().is_none());
        }
    }

    mod get_remain_test {

        use super::*;
        use crate::serial::mock_serial::MockReadWrite;

        #[test]
        fn none_when_empty() {
            let mut b = Buffer::new(8);
            assert_eq!(true, b.get_remain().is_none());
        }

        #[test]
        fn none_after_read_all() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(true, b.get_remain().is_none());
        }

        #[test]
        fn rest_all() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.get_remain().unwrap());
            assert_eq!(5, b.pointer);
        }

        #[test]
        fn rest_all_after_read_to_lf() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\ndef"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(b"def", b.get_remain().unwrap());
            assert_eq!(8, b.pointer);
        }

        #[test]
        fn none_after_read_to_lf_all() {
            let mut b = Buffer::new(16);
            let mut m = MockReadWrite::new(vec![b"abc\r\ndef\r\n"]);
            b.fill_buf(&mut m).unwrap();

            assert_eq!(b"abc\r\n", b.read_to_lf().unwrap());
            assert_eq!(5, b.pointer);

            assert_eq!(b"def\r\n", b.read_to_lf().unwrap());
            assert_eq!(10, b.pointer);

            assert_eq!(true, b.get_remain().is_none());
        }
    }
}
