use crate::parser::messages::{ParseResult, SerialMessage};

pub trait Parser {
    fn add_line(&mut self, line: &str) -> ParseResult<SerialMessage>;
}
