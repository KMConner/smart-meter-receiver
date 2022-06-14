#[cfg(test)]
use mockall::mock;

#[cfg(test)]
use crate::serial::Connection;

#[cfg(test)]
mock! {
    pub Serial{}

    impl Connection for Serial {
        fn write_line(&mut self, line: &str) -> crate::serial::errors::Result<()>;
        fn read_line(&mut self) -> crate::serial::errors::Result<String>;
    }
}
