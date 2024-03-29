use std::convert::TryInto;
use std::fmt::Debug;
use std::mem;

use crate::echonet::{Error, Result};
use crate::echonet::enums::{EchonetObject, EchonetProperty, EchonetService};

const ECHONET_LITE_EHD1: u8 = 0x10;
const ECHONET_FORMAT_1: u8 = 0x81;

#[derive(PartialEq, Debug)]
pub struct EchonetPacket<P: EchonetProperty> {
    ehd1: u8,
    ehd2: u8,
    pub transaction_id: u16,
    pub data: Edata<P>,
}

#[repr(packed)]
struct EchonetPacketHeader {
    ehd1: u8,
    ehd2: u8,
    tid: u16,
}

#[derive(PartialEq, Debug)]
pub struct Edata<P: EchonetProperty> {
    pub source_object: EchonetObject,
    pub destination_object: EchonetObject,
    pub echonet_service: EchonetService,
    pub properties: Vec<Property<P>>,
}

#[repr(packed)]
struct EdataHeader {
    seoj: [u8; 3],
    deoj: [u8; 3],
    esv: u8,
    opc: u8,
}

#[derive(PartialEq, Debug)]
pub struct Property<P: EchonetProperty> {
    pub epc: P,
    pub data: Vec<u8>,
}

impl<P: EchonetProperty> EchonetPacket<P> {
    pub fn new(transaction_id: u16, data: Edata<P>) -> Self {
        Self {
            ehd1: ECHONET_LITE_EHD1,
            ehd2: ECHONET_FORMAT_1,
            transaction_id,
            data,
        }
    }

    pub fn parse(bin: &[u8]) -> Result<Self> {
        if bin.len() < 4 {
            return Err(Error::ParseError(String::from("data length too short")));
        }

        let header: [u8; 4] = bin[..4].try_into()?;
        let header: EchonetPacketHeader = unsafe { mem::transmute(header) };
        if header.ehd1 != ECHONET_LITE_EHD1 {
            return Err(Error::InvalidValueError(String::from("EHD1 MUST BE 0x10")));
        }
        if header.ehd2 != ECHONET_FORMAT_1 {
            return Err(Error::InvalidValueError(String::from("EHD2 MUST BE 0x10")));
        }

        let edata = Edata::parse(&bin[4..])?;
        Ok(EchonetPacket {
            ehd1: header.ehd1,
            ehd2: header.ehd2,
            transaction_id: header.tid,
            data: edata,
        })
    }

    pub fn dump(&self) -> Vec<u8> {
        let header = EchonetPacketHeader {
            ehd1: self.ehd1,
            ehd2: self.ehd2,
            tid: self.transaction_id,
        };

        let mut bin = Vec::new();
        let header: [u8; 4] = unsafe { mem::transmute(header) };
        bin.extend(header.iter());
        bin.extend(self.data.dump());
        bin
    }

    pub fn get_property(&self, prop: P) -> Option<&Property<P>> {
        self.data.properties.iter().find(|ep| ep.epc == prop)
    }
}

impl<P: EchonetProperty> Edata<P> {
    fn parse(bin: &[u8]) -> Result<Self> {
        if bin.len() < 8 {
            return Err(Error::ParseError(String::from("data length too short")));
        }

        let header: [u8; 8] = bin[..8].try_into()?;

        let header: EdataHeader = unsafe { mem::transmute(header) };
        let mut edata = Edata {
            source_object: header.seoj.try_into()?,
            destination_object: header.deoj.try_into()?,
            echonet_service: header.esv.try_into()?,
            properties: Vec::new(),
        };

        let mut pos = 8;
        for _ in 0..header.opc {
            if pos >= bin.len() {
                return Err(Error::ParseError(String::from("data length too short")));
            }
            let (num, prop) = Property::parse(&bin[pos..])?;
            pos += num;
            edata.properties.push(prop);
        }

        Ok(edata)
    }

    fn dump(&self) -> Vec<u8> {
        let header = EdataHeader {
            seoj: self.source_object.into(),
            deoj: self.destination_object.into(),
            esv: self.echonet_service as u8,
            opc: self.properties.len() as u8,
        };

        let header: [u8; 8] = unsafe { mem::transmute(header) };
        let mut bin = Vec::new();
        bin.extend(header.iter());
        for d in &self.properties {
            bin.extend(d.dump().iter());
        }

        bin
    }
}

impl<P: EchonetProperty> Property<P> {
    fn parse(bin: &[u8]) -> Result<(usize, Self)> {
        if bin.len() < 2 {
            return Err(Error::ParseError(String::from("empty data")));
        }

        let b = bin[0];

        let epc: P = P::try_from_primitive(b)?;
        let pdc = bin[1] as usize;
        if bin.len() < 2 + pdc {
            return Err(Error::ParseError(String::from("less data length")));
        }
        let data = bin[2..pdc + 2].to_vec();
        let ret = Property { epc, data };
        Ok((2 + pdc, ret))
    }

    fn dump(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.data.len() + 2);
        data.push(self.epc.into());
        data.push(self.data.len() as u8);
        data.extend_from_slice(self.data.as_slice());

        data
    }

    pub fn get_i32(&self) -> Option<i32> {
        let bin: [u8; 4] = match self.data.clone().try_into() {
            Ok(b) => b,
            Err(_) => { return None; }
        };
        Some(i32::from_be_bytes(bin))
    }

    pub fn get_u32(&self) -> Option<u32> {
        let bin: [u8; 4] = match self.data.clone().try_into() {
            Ok(b) => b,
            Err(_) => { return None; }
        };
        Some(u32::from_be_bytes(bin))
    }
}

#[cfg(test)]
mod test {
    use crate::echonet::{EchonetPacket, Edata, Property};
    use crate::echonet::enums::EchonetSmartMeterProperty;

    type SmartMeterPacket = EchonetPacket<EchonetSmartMeterProperty>;
    type SmartMeterProperty = Property<EchonetSmartMeterProperty>;
    type SmartMeterEdata = Edata<EchonetSmartMeterProperty>;

    mod packet_test {
        use crate::echonet::enums::{EchonetObject, EchonetService, EchonetSmartMeterProperty};
        use crate::echonet::packet::{EchonetPacket, Edata, Property};
        use crate::echonet::packet::test::SmartMeterPacket;

        #[test]
        fn parse_test() {
            #[cfg(target_endian = "big")]
                let tid = 0x0001;

            #[cfg(target_endian = "little")]
                let tid = 0x0100;

            let bin = hex::decode("1081000102880105FF017202E7040000020EE7040000020F").unwrap();
            let expected = EchonetPacket {
                ehd1: 0x10,
                ehd2: 0x81,
                transaction_id: tid,
                data: Edata {
                    source_object: EchonetObject::SmartMeter,
                    destination_object: EchonetObject::HemsController,
                    echonet_service: EchonetService::ReadPropertyResponse,
                    properties: vec![Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020E").unwrap() },
                                     Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020F").unwrap() }],
                },
            };
            assert_eq!(EchonetPacket::parse(bin.as_slice()).unwrap(), expected);
        }

        #[test]
        fn parse_invalid_ehd1() {
            let bin = hex::decode("1181000102880105FF017202E7040000020EE7040000020F").unwrap();
            assert_eq!(SmartMeterPacket::parse(bin.as_slice()).is_err(), true);
        }

        #[test]
        fn parse_invalid_ehd2() {
            let bin = hex::decode("1082000102880105FF017202E7040000020EE7040000020F").unwrap();
            assert_eq!(SmartMeterPacket::parse(bin.as_slice()).is_err(), true);
        }

        #[test]
        fn parse_error_too_short_1() {
            let bin = hex::decode("10810001").unwrap();
            assert_eq!(SmartMeterPacket::parse(bin.as_slice()).is_err(), true);
        }

        #[test]
        fn dump_test() {
            #[cfg(target_endian = "big")]
                let tid = 0x0001;

            #[cfg(target_endian = "little")]
                let tid = 0x0100;

            let bin = hex::decode("1081000102880105FF017202E7040000020EE7040000020F").unwrap();
            let packet: EchonetPacket<EchonetSmartMeterProperty> = EchonetPacket {
                ehd1: 0x10,
                ehd2: 0x81,
                transaction_id: tid,
                data: Edata {
                    source_object: EchonetObject::SmartMeter,
                    destination_object: EchonetObject::HemsController,
                    echonet_service: EchonetService::ReadPropertyResponse,
                    properties: vec![Property {
                        epc: EchonetSmartMeterProperty::InstantaneousElectricPower,
                        data: hex::decode(
                            "0000020E").unwrap(),
                    },
                                     Property {
                                         epc: EchonetSmartMeterProperty::InstantaneousElectricPower,
                                         data: hex::decode(
                                             "0000020F").unwrap(),
                                     }],
                },
            };
            assert_eq!(bin, packet.dump());
        }
    }

    mod edata_test {
        use crate::echonet::enums::{EchonetObject, EchonetService, EchonetSmartMeterProperty};
        use crate::echonet::packet::{Edata, Property};
        use crate::echonet::packet::test::SmartMeterEdata;

        #[test]
        fn parse_test() {
            let bin = hex::decode("02880105FF017202E7040000020EE7040000020F").unwrap();
            let expected = Edata {
                source_object: EchonetObject::SmartMeter,
                destination_object: EchonetObject::HemsController,
                echonet_service: EchonetService::ReadPropertyResponse,
                properties: vec![Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020E").unwrap() },
                                 Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020F").unwrap() }],
            };
            assert_eq!(Edata::parse(bin.as_slice()).unwrap(), expected);
        }

        #[test]
        fn parse_test_less_property() {
            let bin = hex::decode("02880105FF017202E7040000020E").unwrap();
            assert_eq!(SmartMeterEdata::parse(bin.as_slice()).is_err(), true);
        }

        #[test]
        fn dump_test() {
            let data = Edata {
                source_object: EchonetObject::SmartMeter,
                destination_object: EchonetObject::HemsController,
                echonet_service: EchonetService::ReadPropertyResponse,
                properties: vec![Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020E").unwrap() },
                                 Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020F").unwrap() }],
            };
            let bin = hex::decode("02880105FF017202E7040000020EE7040000020F").unwrap();
            assert_eq!(data.dump(), bin);
        }
    }

    mod property_test {
        use crate::echonet::enums::EchonetSmartMeterProperty;
        use crate::echonet::packet::Property;
        use crate::echonet::packet::test::SmartMeterProperty;

        #[test]
        fn parse_test_1() {
            let bin = hex::decode("E7040000020E").unwrap();
            let expected = Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020E").unwrap() };
            let (actual, data) = Property::parse(bin.as_slice()).unwrap();
            assert_eq!(data, expected);
            assert_eq!(actual, 6);
        }

        #[test]
        fn parse_test_2() {
            let bin = hex::decode("E7040000020EE704000FF20E").unwrap();
            let expected = Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020E").unwrap() };
            let (actual, data) = Property::parse(bin.as_slice()).unwrap();
            assert_eq!(data, expected);
            assert_eq!(actual, 6);
        }

        #[test]
        fn parse_error_on_empty() {
            let bin = hex::decode("").unwrap();
            assert_eq!(SmartMeterProperty::parse(bin.as_slice()).is_err(), true);
        }

        #[test]
        fn parse_error_on_insufficient_length() {
            let bin = hex::decode("E704000002").unwrap();
            assert_eq!(SmartMeterProperty::parse(bin.as_slice()).is_err(), true);
        }

        #[test]
        fn dump_test() {
            let bin = hex::decode("E7040000020E").unwrap();
            let property = Property { epc: EchonetSmartMeterProperty::InstantaneousElectricPower, data: hex::decode("0000020E").unwrap() };
            assert_eq!(bin, property.dump());
        }
    }
}
