use std::net::Ipv6Addr;
use std::thread::sleep;

use std::time::{Duration, SystemTime};
use crate::echonet::{EchonetObject, EchonetPacket, EchonetProperty, EchonetService, EchonetSmartMeterProperty, EchonetSuperClassProperty, Edata, Property, PropertyMap};

use crate::parser::{Parser, ParseResult, SerialMessage, WiSunEvent, WiSunModuleParser};
use crate::parser::event::{EventKind, PanDescBody};
use crate::serial::{Connection, Error as SerialError};
use crate::wisun_module::errors::{Error, Result};

const ECHONET_PORT: u16 = 3610;

pub struct WiSunClient<T: Connection> {
    serial_connection: T,
    serial_parser: WiSunModuleParser,
    message_buffer: Vec<SerialMessage>,
    address: Option<Ipv6Addr>,
    property_map: Option<PropertyMap>,
}

impl<T: Connection> WiSunClient<T> {
    pub fn new(serial_connection: T) -> Result<Self> {
        let mut client = WiSunClient {
            serial_connection,
            serial_parser: WiSunModuleParser::new(),
            message_buffer: Vec::new(),
            address: None,
            property_map: None,
        };
        client.ensure_echoback_off()?;
        Ok(client)
    }

    fn get_message(&mut self) -> Result<bool> {
        loop {
            match self.serial_connection.read_line() {
                Ok(line) => {
                    match self.serial_parser.add_line(line.as_str()) {
                        ParseResult::Ok(m) => {
                            self.message_buffer.push(m);
                            return Ok(true);
                        }
                        ParseResult::Empty => {
                            return Ok(false);
                        }
                        ParseResult::More => {
                            continue;
                        }
                        ParseResult::Err(_) => {
                            // TODO: logging
                            continue;
                        }
                    }
                }
                Err(SerialError::IoError(ioe)) => {
                    return Err(Error::SerialError(SerialError::IoError(ioe)));
                }
                Err(e) => {
                    log::debug!("{:?}", e);

                    return Err(Error::SerialError(e));
                }
            }
        }
    }

    pub fn flush_messages(&mut self) {
        // TODO: read line
        log::debug!("flushing messages");
        self.message_buffer.clear();
    }

    fn search_on_buffer<F>(&mut self, pred: &F) -> Option<SerialMessage>
        where F: Fn(&SerialMessage) -> bool {
        // Search on message_buffer
        let mut delete_idx = usize::MAX;
        for i in 0..self.message_buffer.len() {
            if let Some(m) = self.message_buffer.get(i) {
                if pred(m) {
                    delete_idx = i;
                    break;
                }
            }
        }
        if delete_idx < usize::MAX {
            return Some(self.message_buffer.remove(delete_idx));
        }
        None
    }

    fn wait_fn<F, H>(&mut self, pred: F, err_if: H, timeout: Option<Duration>) -> Result<SerialMessage>
        where F: Fn(&SerialMessage) -> bool, H: Fn(&SerialMessage) -> Option<String> {

        // Search on message_buffer
        if let Some(m) = self.search_on_buffer(&pred) {
            return Ok(m);
        }

        let start = SystemTime::now();

        // get new message from console
        loop {
            match timeout {
                Some(t) => {
                    if SystemTime::now() > start + t {
                        return Err(Error::TimeoutError());
                    }
                }
                None => {}
            }
            match self.get_message() {
                Ok(true) => {
                    if let Some(m) = self.message_buffer.last() {
                        if pred(m) {
                            return Ok(self.message_buffer.remove(self.message_buffer.len() - 1));
                        }
                        if let Some(e) = err_if(m) {
                            return Err(Error::CommandError(e));
                        }
                    }
                }
                Err(Error::SerialError(SerialError::IoError(ioe))) => {
                    if ioe.kind() == std::io::ErrorKind::TimedOut {
                        continue;
                    }
                }
                _ => { continue; }
            }
            sleep(Duration::from_millis(1));
        }
    }

    fn wait_ok(&mut self) -> Result<()> {
        self.wait_fn(|m| *m == SerialMessage::Ok, err_when_fail, None)?;
        Ok(())
    }

    fn ensure_echoback_off(&mut self) -> Result<()> {
        self.flush_messages();
        self.serial_connection.write_line("SKSREG SFE 0")?;
        self.wait_ok()
    }

    pub fn get_version(&mut self) -> Result<String> {
        self.flush_messages();
        self.serial_connection.write_line("SKVER")?;
        self.wait_ok()?;
        let msg = self.wait_fn(|m| -> bool{
            match m {
                SerialMessage::Event(WiSunEvent::Version(_)) => true,
                _ => false,
            }
        }, err_when_fail, None)?;
        if let SerialMessage::Event(WiSunEvent::Version(ver)) = msg {
            return Ok(ver);
        }
        Err(Error::CommandError("Unexpected msg".to_string()))
    }

    pub fn connect(&mut self, bid: &str, password: &str) -> Result<()> {
        self.set_password(password)?;
        self.set_bid(bid)?;
        let pan = self.scan()?;
        let channel = format!("{:X}", pan.channel);
        let pan_id = format!("{:X}", pan.pan_id);
        self.set_register("S2", channel.as_str())?;
        self.set_register("S3", pan_id.as_str())?;
        let ip = self.get_ip(&pan.addr);
        self.join(&ip)?;
        self.address = Some(ip);
        self.get_property_map()?;
        Ok(())
    }

    fn set_password(&mut self, password: &str) -> Result<()> {
        self.flush_messages();
        let line = format!("SKSETPWD {:X} {}", password.len(), password);
        self.serial_connection.write_line(line.as_str())?;
        self.wait_ok()
    }

    fn set_bid(&mut self, bid: &str) -> Result<()> {
        self.flush_messages();
        let line = format!("SKSETRBID {}", bid);
        self.serial_connection.write_line(line.as_str())?;
        self.wait_ok()
    }

    fn scan(&mut self) -> Result<PanDescBody> {
        for i in 4..10 {
            // Start scanning -> Wait for scan finish -> Look for EPANDESC
            self.flush_messages();
            let line = format!("SKSCAN 2 FFFFFFFF {}", i);
            self.serial_connection.write_line(line.as_str())?;
            self.wait_ok()?;
            self.wait_fn(|m| -> bool{
                match m {
                    SerialMessage::Event(WiSunEvent::Event(e)) => {
                        e.kind == EventKind::FinishedActiveScan
                    }
                    _ => false,
                }
            }, err_when_fail, None)?;
            let desc = self.search_on_buffer(&|m| -> bool{
                match m {
                    SerialMessage::Event(WiSunEvent::PanDesc(_)) => true,
                    _ => false,
                }
            });
            if let Some(SerialMessage::Event(WiSunEvent::PanDesc(body))) = desc {
                return Ok(body);
            }
        }
        Err(Error::ScanError("pan not found".to_string()))
    }

    fn join(&mut self, addr: &Ipv6Addr) -> Result<()> {
        let line = format!("SKJOIN {}", ipv6_addr_full_string(addr));
        self.serial_connection.write_line(line.as_str())?;
        self.wait_ok()?;
        self.wait_fn(|m| -> bool{
            match m {
                SerialMessage::Event(WiSunEvent::Event(e)) => {
                    e.kind == EventKind::EstablishedPanaConnection
                }
                _ => false,
            }
        },
                     |m| -> Option<String>{
                         match m {
                             SerialMessage::Fail(s) => Some(s.clone()),
                             SerialMessage::Event(WiSunEvent::Event(e)) => {
                                 if e.kind == EventKind::ErrorOnPanaConnection {
                                     return Some("failed to connect to pana".to_string());
                                 }
                                 None
                             }
                             _ => None
                         }
                     }, None)?;
        Ok(())
    }

    fn set_register(&mut self, reg: &str, value: &str) -> Result<()> {
        self.flush_messages();
        let line = format!("SKSREG {} {}", reg, value);
        self.serial_connection.write_line(line.as_str())?;
        self.wait_ok()
    }

    fn get_ip(&self, addr: &[u8; 8]) -> Ipv6Addr {
        let mut ip: [u8; 16] = [0xFE, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        for i in 0..8 {
            ip[8 + i] = addr[i];
        }
        ip[8] ^= 0b00000010;
        ip.into()
    }

    fn get_properties<P: EchonetProperty>(&mut self, props: &[P]) -> Result<EchonetPacket<P>> {
        self.check_property_exists(props)?;
        let transaction_id = rand::random();
        let packet = EchonetPacket::new(transaction_id, Edata {
            source_object: EchonetObject::HemsController,
            destination_object: EchonetObject::SmartMeter,
            echonet_service: EchonetService::ReadPropertyRequest,
            properties: props.iter()
                .map(|p| Property { epc: *p, data: Vec::new() })
                .collect(),
        });
        self.send_udp(&packet.dump())?;
        let packet = self.wait_echonet_packet(|p: &EchonetPacket<P>| -> bool{
            if p.transaction_id != transaction_id {
                return false;
            }
            let edata = &p.data;
            if edata.destination_object != EchonetObject::HemsController || edata.source_object != EchonetObject::SmartMeter {
                return false;
            }
            true
        }, Duration::from_secs(20))?;

        Ok(packet)
    }

    fn check_property_exists<P: EchonetProperty>(&self, props: &[P]) -> Result<()> {
        if props.len() == 1 && props[0].into() == EchonetSuperClassProperty::GetPropertyMap.into() {
            return Ok(());
        }

        let map = match &self.property_map {
            Some(m) => m,
            None => {
                return Err(Error::CommandError(String::from("property map is not initialized.")));
            }
        };
        for p in props {
            if !map.has_property(*p) {
                return Err(Error::CommandError(format!("Property {:?} is not implemented.", p)));
            }
        }
        Ok(())
    }

    pub fn get_power_consumption(&mut self) -> Result<i32> {
        let packet = self.get_properties(&[EchonetSmartMeterProperty::InstantaneousElectricPower])?;

        let property = match packet.get_property(EchonetSmartMeterProperty::InstantaneousElectricPower).map(|p| p.get_i32()) {
            Some(Some(p)) => p,
            Some(None) => {
                return Err(Error::CommandError("malformed property".to_string()));
            }
            None => {
                return Err(Error::CommandError("unknown error".to_string()));
            }
        };

        Ok(property)
    }

    pub fn get_property_map(&mut self) -> Result<()> {
        let prop = self.get_properties(&[EchonetSuperClassProperty::GetPropertyMap])?
            .get_property(EchonetSuperClassProperty::GetPropertyMap)
            .map(|p| PropertyMap::parse(&p.data));

        match prop {
            Some(Ok(m)) => {
                log::debug!("property id list: {:X?}", m.get_property_ids());
                self.property_map = Some(m);
                Ok(())
            }
            Some(Err(e)) => Err(e.into()),
            None => Err(Error::CommandError("property not found".to_string()))
        }
    }

    pub fn get_cumulative_electric_energy(&mut self) -> Result<f64> {
        let props = self.get_properties(
            &[EchonetSmartMeterProperty::NormalDirectionCumulativeElectricEnergy,
                EchonetSmartMeterProperty::UnitForCumulativeElectricEnergy,
                EchonetSmartMeterProperty::Coefficient])?;

        let base = match props.get_property(EchonetSmartMeterProperty::NormalDirectionCumulativeElectricEnergy).map(|p| p.get_u32()) {
            Some(Some(p)) => p,
            Some(None) => {
                return Err(Error::CommandError("malformed property".to_string()));
            }
            None => {
                return Err(Error::CommandError("unknown error".to_string()));
            }
        };
        let unit = match props.get_property(EchonetSmartMeterProperty::UnitForCumulativeElectricEnergy).map(|p| p.data[0]) {
            Some(0x00) => 1.0,
            Some(0x01) => 0.1,
            Some(0x02) => 0.01,
            Some(0x03) => 0.001,
            Some(0x04) => 0.0001,
            Some(0x0A) => 10.0,
            Some(0x0B) => 100.0,
            Some(0x0C) => 1000.0,
            Some(0x0D) => 10000.0,
            None => {
                return Err(Error::CommandError("unknown error".to_string()));
            }
            Some(b) => {
                return Err(Error::CommandError(format!("unexpected unit {:X}", b)));
            }
        };

        let coefficient = match props.get_property(EchonetSmartMeterProperty::Coefficient).map(|p| p.get_u32()) {
            Some(Some(p)) => p,
            Some(None) => {
                return Err(Error::CommandError("malformed property".to_string()));
            }
            None => {
                return Err(Error::CommandError("unknown error".to_string()));
            }
        };
        log::debug!("base: {}, unit: {}, coefficient: {}",base,unit,coefficient);

        Ok((base as f64) * unit * (coefficient as f64))
    }

    fn send_udp(&mut self, data: &[u8]) -> Result<()> {
        let addr = match self.address {
            Some(a) => a,
            None => {
                return Err(Error::CommandError("address is not set".to_string()));
            }
        };
        self.flush_messages();
        let security_bit = 1u8;
        let data_base = create_send_udp_base(&addr, security_bit, data.len());
        let mut bin: Vec<u8> = Vec::new();
        bin.extend_from_slice(data_base.as_bytes());
        bin.extend_from_slice(data);
        bin.extend_from_slice("\r\n".as_bytes());

        self.serial_connection.write_byte(bin.as_slice())?;
        self.wait_ok()
    }

    fn wait_echonet_packet<F, P: EchonetProperty>(&mut self, pred: F, timeout: Duration) -> Result<EchonetPacket<P>>
        where F: Fn(&EchonetPacket<P>) -> bool {
        let msg = self.wait_fn(|m| -> bool{
            match m {
                SerialMessage::Event(WiSunEvent::RxUdp(p)) => {
                    match EchonetPacket::parse(p.data.as_slice()) {
                        Ok(e) => pred(&e),
                        Err(e) => {
                            log::warn!("failed to parse packet: {:?} packet: {}",e, hex::encode(p.data.as_slice()));
                            false
                        }
                    }
                }
                _ => false,
            }
        }, err_when_fail, Some(timeout))?;
        if let SerialMessage::Event(WiSunEvent::RxUdp(p)) = msg {
            return Ok(EchonetPacket::parse(p.data.as_slice())?);
        }
        return Err(Error::CommandError("Unknown error".to_string()));
    }
}

fn create_send_udp_base(addr: &Ipv6Addr, security_bit: u8, data_length: usize) -> String {
    format!("SKSENDTO 1 {} {:04X} {} {:04X} ", ipv6_addr_full_string(addr), ECHONET_PORT, security_bit, data_length)
}

fn err_when_fail(m: &SerialMessage) -> Option<String> {
    match m {
        SerialMessage::Fail(s) => Some(s.clone()),
        _ => None
    }
}

fn ipv6_addr_full_string(ip: &Ipv6Addr) -> String {
    let seg = &ip.segments();
    format!("{:04X}:{:04X}:{:04X}:{:04X}:{:04X}:{:04X}:{:04X}:{:04X}",
            seg[0], seg[1], seg[2], seg[3], seg[4], seg[5], seg[6], seg[7])
}

#[cfg(test)]
mod test {
    use std::net::Ipv6Addr;
    use std::str::FromStr;
    use crate::parser::WiSunModuleParser;

    use crate::wisun_module::client::{create_send_udp_base, ipv6_addr_full_string};
    use crate::wisun_module::mock::MockSerial;

    use super::WiSunClient;

    fn new_client<F>(mut prepare_mock: F) -> WiSunClient<MockSerial>
        where
            F: FnMut(&mut MockSerial),
    {
        let mut mock_serial = MockSerial::new();
        prepare_mock(&mut mock_serial);
        WiSunClient {
            serial_connection: mock_serial,
            serial_parser: WiSunModuleParser::new(),
            message_buffer: Vec::new(),
            address: None,
            property_map: None,
        }
    }

    mod wait_ok_test {
        use std::io::{Error as IoError, ErrorKind as IoErrorKind};

        use mockall::Sequence;

        use crate::serial::Error as SerialError;

        use super::*;

        #[test]
        fn ok_when_read_ok() {
            let mut cli = new_client(|s| -> () {
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn read_again_when_not_ok() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s| -> () {
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));

                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("OK")));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn read_again_when_timeout() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s| -> () {
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| {
                        Err(SerialError::IoError(IoError::new(
                            IoErrorKind::TimedOut,
                            "timeout",
                        )))
                    });
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("OK")));
            });
            cli.wait_ok().unwrap();
        }

        #[test]
        fn error_when_fail() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s| -> () {
                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("SKVER")));

                s.expect_read_line()
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|| Ok(String::from("FAIL ER04")));
            });
            assert_eq!(cli.wait_ok().is_err(), true);
        }
    }

    mod get_version_test {
        use mockall::{predicate, Sequence};

        use super::*;

        #[test]
        fn ok_before_ever() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s| -> () {
                s.expect_write_line()
                    .with(predicate::eq("SKVER"))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|_| Ok(()));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("EVER 1.2.3")));
            });
            let ver = cli.get_version().unwrap();
            assert_eq!(ver, "1.2.3".to_string());
        }

        #[test]
        fn ever_before_ok() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s| -> () {
                s.expect_write_line()
                    .with(predicate::eq("SKVER"))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|_| Ok(()));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("EVER 2.3.4")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
            });
            let ver = cli.get_version().unwrap();
            assert_eq!(ver, "2.3.4".to_string());
        }
    }

    mod connect_test {
        use std::net::Ipv6Addr;

        use mockall::{predicate, Sequence};

        use crate::parser::event::PanDescBody;
        use crate::wisun_module::client::test::new_client;

        #[test]
        fn get_ip() {
            let cli = new_client(|_| {});
            let mac: [u8; 8] = [0x00, 0x1D, 0x12, 0x90, 0x12, 0x34, 0x56, 0x78];
            let expected = Ipv6Addr::from([
                0xFE, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x02, 0x1D, 0x12, 0x90, 0x12, 0x34, 0x56, 0x78
            ]);
            assert_eq!(expected, cli.get_ip(&mac));
        }

        #[test]
        fn scan() {
            let mut seq = Sequence::new();
            let mut cli = new_client(|s| {
                s.expect_write_line()
                    .with(predicate::eq("SKSCAN 2 FFFFFFFF 4"))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|_| Ok(()));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok("EVENT 22 FE80:0000:0000:0000:1234:5678:90AB:CDEF".to_string()));

                s.expect_write_line()
                    .with(predicate::eq("SKSCAN 2 FFFFFFFF 5"))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(|_| Ok(()));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("OK")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("EPANDESC")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("  Channel:2F")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("  Channel Page:09")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("  Pan ID:3077")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("  Addr:1234567890ABCDEF")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("  LQI:73")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok(String::from("  PairID:01234567")));
                s.expect_read_line()
                    .times(1)
                    .returning(|| Ok("EVENT 22 FE80:0000:0000:0000:1234:5678:90AB:CDEF".to_string()));
            });
            assert_eq!(PanDescBody {
                channel: 0x2F,
                pan_id: 0x3077,
                addr: [0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF],
            }, cli.scan().unwrap());
        }
    }

    #[test]
    fn ipv6_addr_full_string_test() {
        let ip = Ipv6Addr::from_str("FE80:0000:0000:0000:1234:5678:90AB:CDEF").unwrap();
        assert_eq!(ipv6_addr_full_string(&ip), "FE80:0000:0000:0000:1234:5678:90AB:CDEF".to_string());
    }

    #[test]
    fn create_send_udp_base_test() {
        let addr = Ipv6Addr::from_str("FE80:0000:0000:0000:1234:5678:90AB:CDEF").unwrap();

        assert_eq!(create_send_udp_base(&addr, 1, 30),
                   "SKSENDTO 1 FE80:0000:0000:0000:1234:5678:90AB:CDEF 0E1A 1 001E ");
    }
}
