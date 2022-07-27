use std::net::Ipv6Addr;

#[derive(Debug, PartialEq)]
enum ParseResult {
    Ok(SerialMessage),
    Err(String),
    More,
}

#[derive(Debug, PartialEq)]
enum SerialMessage {
    Ok,
    Fail(String),
    Event(WiSunEvent),
    Unknown(String),
}

#[derive(Debug, PartialEq)]
enum WiSunEvent {
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
#[derive(Debug, PartialEq)]
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
