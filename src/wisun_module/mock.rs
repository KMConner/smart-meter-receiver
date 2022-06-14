#[cfg(test)]
use mockall::mock;

#[cfg(test)]
mock! {
    pub Connection{}

    impl crate::serial::Connection for Connection {
        fn write_line(&mut self, line: &str) -> crate::serial::errors::Result<()>;
        fn read_line(&mut self) -> crate::serial::errors::Result<String>;
    }
}
