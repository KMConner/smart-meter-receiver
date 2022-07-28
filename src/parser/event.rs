use std::collections::HashMap;
use std::net::Ipv6Addr;
use crate::parser::messages::ParseResult;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum WiSunEvent {
    PanDesc(PanDescBody),
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

#[derive(Debug, PartialEq)]
pub struct PanDescBody {
    channel: u8,
    pan_id: u16,
    addr: [u8; 8],
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

    fn parse_pan_desc(data: &str) -> ParseResult<Self> {
        let lines: Vec<&str> = data.split('\n').collect();
        if lines.len() != 7 {
            return ParseResult::More;
        }

        let mut pan_data = HashMap::<&str, &str>::new();
        for l in &lines[1..] {
            let kv = l.split(':').map(|s| s.trim()).collect::<Vec<&str>>();
            if kv.len() != 2 {
                return ParseResult::Err(format!("Malformed line in EPANDESC: {}", l));
            }
            pan_data.insert(kv[0], kv[1]);
        }

        let pan_data = pan_data;
        let channel = match pan_data.get("Channel") {
            Some(c) => c,
            None => return ParseResult::Err(format!("failed to get channel id."))
        };
        let channel = match u8::from_str_radix(channel, 16) {
            Ok(c) => c,
            Err(e) => return ParseResult::Err(format!("failed to parse channel: {}", e))
        };

        let pan_id = match pan_data.get("Pan ID") {
            Some(c) => c,
            None => return ParseResult::Err(format!("failed to get pan id."))
        };
        let pan_id = match u16::from_str_radix(pan_id, 16) {
            Ok(c) => c,
            Err(e) => return ParseResult::Err(format!("failed to parse pan id: {}", e)),
        };

        let addr_str = match pan_data.get("Addr") {
            Some(a) => a,
            None => return ParseResult::Err(format!("failed to get addr."))
        };
        let addr = match hex::decode(addr_str) {
            Ok(h) => h,
            Err(e) => return ParseResult::Err(format!("failed to parse addr: {}", e)),
        };
        let addr: [u8; 8] = match addr.try_into() {
            Ok(h) => h,
            Err(_) => return ParseResult::Err(format!("malformed addr: {}", addr_str)),
        };

        ParseResult::Ok(WiSunEvent::PanDesc(PanDescBody {
            channel,
            pan_id,
            addr,
        }))
    }


    pub fn parse(data: &str) -> ParseResult<Self> {
        if data.len() == 0 {
            return ParseResult::Empty;
        }

        let parts: Vec<&str> = data.trim().split(&[' ', '\n']).map(|s| s.trim()).collect();
        if parts.len() < 1 {
            return ParseResult::Err(format!("Malformed event line: {}", data));
        }

        match parts[0] {
            "EVENT" => WiSunEvent::parse_event(data, parts),
            "ERXUDP" => WiSunEvent::parse_rx_udp(data, parts),
            "EPANDESC" => WiSunEvent::parse_pan_desc(data),
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

    #[test]
    fn parse_pan_desc_single_line() {
        assert_eq!(WiSunEvent::parse("EPANDESC"), ParseResult::More);
    }

    #[test]
    fn parse_pan_desc_2_lines() {
        assert_eq!(WiSunEvent::parse("EPANDESC\n  Channel:2F"), ParseResult::More);
    }

    #[test]
    fn parse_pan_desc_3_lines() {
        assert_eq!(WiSunEvent::parse("EPANDESC\n  Channel:2F\n  Channel Page:09"), ParseResult::More);
    }

    #[test]
    fn parse_pan_desc_4_lines() {
        assert_eq!(WiSunEvent::parse("EPANDESC\n  Channel:2F\n  Channel Page:09\n  Pan ID:3077"), ParseResult::More);
    }

    #[test]
    fn parse_pan_desc_5_lines() {
        assert_eq!(WiSunEvent::parse("EPANDESC\n  Channel:2F\n  Channel Page:09\n  Pan ID:3077\n  Addr:C0F9450040213077"), ParseResult::More);
    }

    #[test]
    fn parse_pan_desc_6_lines() {
        assert_eq!(WiSunEvent::parse("EPANDESC\n  Channel:2F\n  Channel Page:09\n  Pan ID:3077\n  Addr:C0F9450040213077\n  LQI:73"), ParseResult::More);
    }

    #[test]
    fn parse_pan_desc_7_lines() {
        assert_eq!(WiSunEvent::parse("EPANDESC\n  Channel:20\n  Channel Page:09\n  Pan ID:3077\n  Addr:1234567890ABCDEF\n  LQI:73\n  PairID:01234567"),
                   ParseResult::Ok(WiSunEvent::PanDesc(PanDescBody {
                       channel: 0x20,
                       pan_id: 0x3077,
                       addr: [0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF],
                   })));
    }
}
