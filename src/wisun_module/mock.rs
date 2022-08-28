#[cfg(test)]
use mockall::mock;

#[cfg(test)]
use crate::serial::Connection;

#[cfg(test)]
use crate::parser::Parser;

#[cfg(test)]
use crate::parser::{ParseResult, SerialMessage};

#[cfg(test)]
mock! {
    pub Serial{}

    impl Connection for Serial {
        fn write_line(&mut self, line: &str) -> crate::serial::errors::Result<()>;
        fn read_line(&mut self) -> crate::serial::errors::Result<String>;
    }
}

#[cfg(test)]
mock! {
    pub SerialParser{}

    impl Parser for SerialParser{
            fn add_line(&mut self, line: &str) -> ParseResult<SerialMessage>;
    }
}
