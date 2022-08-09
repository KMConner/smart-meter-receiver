use crate::parser::messages::{ParseResult, SerialMessage};

struct WiSunModuleParser {
    pending_message: Option<String>,
}

impl WiSunModuleParser {
    pub fn new() -> Self {
        WiSunModuleParser {
            pending_message: None
        }
    }

    pub fn add_line(&mut self, line: &str) -> ParseResult<SerialMessage> {
        if self.pending_message.is_none() && line.len() == 0 {
            return ParseResult::Empty;
        }

        let all_line = match &self.pending_message {
            Some(l) => format!("{}\n{}", l, line),
            None => line.to_string(),
        };

        self.pending_message = None;

        match SerialMessage::parse(all_line.as_str()) {
            ParseResult::Ok(m) => ParseResult::Ok(m),
            ParseResult::Err(s) => ParseResult::Err(s),
            ParseResult::More => {
                self.pending_message = Some(all_line);
                ParseResult::More
            }
            ParseResult::Empty => ParseResult::Empty,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parser::event::{EventBody, EventKind, PanDescBody, WiSunEvent};
    use super::*;

    #[test]
    fn add_line_empty() {
        let mut parser = WiSunModuleParser::new();
        assert_eq!(parser.add_line(""), ParseResult::Empty);
    }

    #[test]
    fn add_line_ok() {
        let mut parser = WiSunModuleParser::new();
        assert_eq!(parser.add_line("OK"), ParseResult::Ok(SerialMessage::Ok));
    }

    #[test]
    fn add_line_fail() {
        let mut parser = WiSunModuleParser::new();
        assert_eq!(parser.add_line("FAIL 01"), ParseResult::Ok(SerialMessage::Fail(String::from("01"))));
    }

    #[test]
    fn add_line_single_message() {
        let mut parser = WiSunModuleParser::new();
        let even_body = EventBody {
            kind: EventKind::EstablishedPanaConnection,
            sender: "FE80:0000:0000:0000:1234:5678:90AB:CDEF".parse().unwrap(),
        };
        assert_eq!(
            parser.add_line("EVENT 25 FE80:0000:0000:0000:1234:5678:90AB:CDEF"),
            ParseResult::Ok(SerialMessage::Event(WiSunEvent::Event(even_body)))
        );
    }

    #[test]
    fn add_line_unknown() {
        let mut parser = WiSunModuleParser::new();
        assert_eq!(parser.add_line("FOOBAR"), ParseResult::Err(String::from("FOOBAR")));
    }

    #[test]
    fn add_line_multiline_event() {
        let mut parser = WiSunModuleParser::new();
        assert_eq!(parser.add_line("EPANDESC"), ParseResult::More);
        assert_eq!(parser.add_line("  Channel:20"), ParseResult::More);
        assert_eq!(parser.add_line("  Channel Page:09"), ParseResult::More);
        assert_eq!(parser.add_line("  Pan ID:3077"), ParseResult::More);
        assert_eq!(parser.add_line("  Addr:1234567890ABCDEF"), ParseResult::More);
        assert_eq!(parser.add_line("  LQI:73"), ParseResult::More);
        assert_eq!(
            parser.add_line("  PairID:01234567"),
            ParseResult::Ok(
                SerialMessage::Event(
                    WiSunEvent::PanDesc(
                        PanDescBody {
                            channel: 0x20,
                            pan_id: 0x3077,
                            addr: [0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF],
                        }
                    )
                )
            )
        );
    }

    #[test]
    fn add_line_err() {
        let mut parser = WiSunModuleParser::new();
        assert_eq!(parser.add_line("EPANDESC"), ParseResult::More);
        match parser.add_line("OK") {
            ParseResult::Err(_) => {}
            _ => panic!("Err expected"),
        };
    }
}
