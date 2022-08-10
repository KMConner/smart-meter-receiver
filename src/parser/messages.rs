use crate::parser::event::WiSunEvent;

#[derive(Debug, PartialEq)]
pub enum ParseResult<T> {
    Ok(T),
    Err(String),
    Empty,
    More,
}

#[derive(Debug, PartialEq)]
pub enum SerialMessage {
    Ok,
    Fail(String),
    Event(WiSunEvent),
    // Unknown(String),
}

impl SerialMessage {
    pub(in crate::parser) fn parse(data: &str) -> ParseResult<Self> {
        if data == "OK" {
            return ParseResult::Ok(SerialMessage::Ok);
        }

        if let Some(f) = data.strip_prefix("FAIL ") {
            return ParseResult::Ok(SerialMessage::Fail(f.trim().to_string()));
        }

        match WiSunEvent::parse(data) {
            ParseResult::Ok(ev) => ParseResult::Ok(SerialMessage::Event(ev)),
            ParseResult::Err(_) => ParseResult::Err(data.to_string()),
            ParseResult::More => ParseResult::More,
            ParseResult::Empty => ParseResult::Empty,
        }
    }
}

#[cfg(test)]
mod test {
    use std::mem::discriminant;
    use super::*;

    #[test]
    fn parse_empty() {
        assert_eq!(SerialMessage::parse(""), ParseResult::Empty);
    }

    #[test]
    fn parse_ok() {
        assert_eq!(SerialMessage::parse("OK"), ParseResult::Ok(SerialMessage::Ok));
    }

    #[test]
    fn parse_fail() {
        assert_eq!(SerialMessage::parse("FAIL 01"), ParseResult::Ok(SerialMessage::Fail(String::from("01"))));
    }

    #[test]
    fn parse_event_more() {
        assert_eq!(SerialMessage::parse("EPANDESC\n  Channel:2F\n  Channel Page:09"), ParseResult::More);
    }

    #[test]
    fn parse_event_err() {
        assert_eq!(discriminant(&SerialMessage::parse("EUNKNOWN")),
                   discriminant(&ParseResult::Err(String::default())));
    }
}
