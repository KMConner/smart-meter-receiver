use std::net::Ipv6Addr;
use crate::parser::messages::ParseResult;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum WiSunEvent {
    RxUdp(UdpPacket),
    Event(EventBody),
}

#[derive(Debug, PartialEq)]
pub struct UdpPacket {
    pub sender: Ipv6Addr,
    pub dest: Ipv6Addr,
    pub source_port: u16,
    pub dest_port: u16,
    // TODO: add mac address field
    pub data: Vec<u8>,
}

#[repr(u8)]
#[derive(Debug, PartialEq, TryFromPrimitive)]
pub enum EventKind {
    FinishedUdpSend = 0x21,
    FinishedActiveScan = 0x22,
    ErrorOnPanaConnection = 0x24,
    EstablishedPanaConnection = 0x25,
}

#[derive(Debug, PartialEq)]
pub struct EventBody {
    pub kind: EventKind,
    pub sender: Ipv6Addr,
    // TODO: Add param if necessary
}

impl WiSunEvent {
    fn parse_event(data: &str, parts: Vec<&str>) -> ParseResult<Self> {
        let event_num = match u8::from_str_radix(parts[1], 16) {
            Ok(i) => i,
            Err(_) => return ParseResult::Err(format!("Malformed event number. Line: {}", data))
        };
        match (EventKind::try_from(event_num), parts[2].parse()) {
            (Ok(k), Ok(ip)) => ParseResult::Ok(WiSunEvent::Event(EventBody { kind: k, sender: ip })),
            _ => ParseResult::Err(String::from(data))
        }
    }

    fn parse_rx_udp(data: &str, parts: Vec<&str>) -> ParseResult<Self> {
        if parts.len() != 9 {
            return ParseResult::Err(String::from(data));
        }

        let (sender, dest) = match (parts[1].parse(), parts[2].parse()) {
            (Ok(s), Ok(d)) => (s, d),
            _ => return ParseResult::Err(String::from(data)),
        };

        let (source_port, dest_port) = match (u16::from_str_radix(parts[3], 16), u16::from_str_radix(parts[4], 16)) {
            (Ok(s), Ok(d)) => (s, d),
            _ => return ParseResult::Err(String::from(data)),
        };

        let data_len = match u16::from_str_radix(parts[7], 16) {
            Ok(l) => l,
            _ => return ParseResult::Err(String::from(data)),
        };

        if data_len as usize * 2 != parts[8].len() {
            return ParseResult::Err(String::from(data));
        }
        let body = match hex::decode(parts[8]) {
            Ok(b) => b,
            _ => return ParseResult::Err(String::from(data)),
        };

        ParseResult::Ok(WiSunEvent::RxUdp(UdpPacket {
            sender,
            dest,
            source_port,
            dest_port,
            data: body,
        }))
    }


    pub fn parse(data: &str) -> ParseResult<Self> {
        if data.len() == 0 {
            return ParseResult::Empty;
        }

        let parts: Vec<&str> = data.trim().split(" ").map(|s| s.trim()).collect();
        if parts.len() < 2 {
            return ParseResult::Err(format!("Malformed event line: {}", data));
        }

        match parts[0] {
            "EVENT" => WiSunEvent::parse_event(data, parts),
            "ERXUDP" => WiSunEvent::parse_rx_udp(data, parts),
            _ => ParseResult::Err(format!("Unknown event name. line: {}", data))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parser::messages::ParseResult;
    use super::*;

    #[test]
    fn parse_empty() {
        assert_eq!(WiSunEvent::parse(""), ParseResult::Empty);
    }

    #[test]
    fn parse_rx_udp() {
        let udp_packet = UdpPacket {
            sender: "FE80:0000:0000:0000:1234:5678:1234:5678".parse().unwrap(),
            source_port: 0x0E1A,
            dest: "FE80:0000:0000:0000:1234:5678:90AB:CDEF".parse().unwrap(),
            dest_port: 0x0E1A,
            data: vec![
                0x10, 0x81, 0x00, 0x00, 0x0E, 0xF0, 0x01, 0x0E, 0xF0, 0x01, 0x73, 0x01, 0xD5,
                0x04, 0x01, 0x02, 0x88, 0x01,
            ],
        };
        assert_eq!(
            WiSunEvent::parse("ERXUDP FE80:0000:0000:0000:1234:5678:1234:5678 FE80:0000:0000:0000:1234:5678:90AB:CDEF 0E1A 0E1A C0F9450040213077 1 0012 108100000EF0010EF0017301D50401028801"),
            ParseResult::Ok(WiSunEvent::RxUdp(udp_packet))
        );
    }

    #[test]
    fn parse_udp_sent() {
        let even_body = EventBody {
            kind: EventKind::FinishedUdpSend,
            sender: "FE80:0000:0000:0000:1234:5678:90AB:CDEF".parse().unwrap(),
        };
        assert_eq!(
            WiSunEvent::parse("EVENT 21 FE80:0000:0000:0000:1234:5678:90AB:CDEF 02"),
            ParseResult::Ok(WiSunEvent::Event(even_body))
        );
    }

    #[test]
    fn parse_finished_active_scan() {
        let even_body = EventBody {
            kind: EventKind::FinishedActiveScan,
            sender: "FE80:0000:0000:0000:1234:5678:90AB:CDEF".parse().unwrap(),
        };
        assert_eq!(
            WiSunEvent::parse("EVENT 22 FE80:0000:0000:0000:1234:5678:90AB:CDEF 02"),
            ParseResult::Ok(WiSunEvent::Event(even_body))
        );
    }

    #[test]
    fn parse_pana_connection_error() {
        let even_body = EventBody {
            kind: EventKind::ErrorOnPanaConnection,
            sender: "FE80:0000:0000:0000:1234:5678:90AB:CDEF".parse().unwrap(),
        };
        assert_eq!(
            WiSunEvent::parse("EVENT 24 FE80:0000:0000:0000:1234:5678:90AB:CDEF"),
            ParseResult::Ok(WiSunEvent::Event(even_body))
        );
    }

    #[test]
    fn parse_pana_connection_established() {
        let even_body = EventBody {
            kind: EventKind::EstablishedPanaConnection,
            sender: "FE80:0000:0000:0000:1234:5678:90AB:CDEF".parse().unwrap(),
        };
        assert_eq!(
            WiSunEvent::parse("EVENT 25 FE80:0000:0000:0000:1234:5678:90AB:CDEF"),
            ParseResult::Ok(WiSunEvent::Event(even_body))
        );
    }
}