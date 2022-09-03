use std::mem;
use crate::echonet::{Error, Result};
use std::convert::TryInto;

struct EchonetPacket {
    ehd1: u8,
    ehd2: u8,
    tid: u16,
    edata: Edata,
}

#[derive(PartialEq, Debug)]
struct Edata {
    seoj: [u8; 3],
    deoj: [u8; 3],
    esv: u8,
    opc: u8,
    data: Vec<Property>,
}

#[repr(packed)]
struct EdataHeader {
    seoj: [u8; 3],
    deoj: [u8; 3],
    esv: u8,
    opc: u8,
}

#[derive(PartialEq, Debug)]
struct Property {
    epc: u8,
    data: Vec<u8>,
}

impl Edata {
    fn parse(bin: &[u8]) -> Result<Self> {
        if bin.len() < 8 {
            return Err(Error::ParseError(String::from("data length too short")));
        }

        let header: [u8; 8] = bin[..8].try_into()?;

        let header: EdataHeader = unsafe { mem::transmute(header) };
        let mut edata = Edata {
            seoj: header.seoj,
            deoj: header.deoj,
            esv: header.esv,
            opc: header.opc,
            data: Vec::new(),
        };

        let mut pos = 8;
        for _ in 0..header.opc {
            if pos >= bin.len() {
                return Err(Error::ParseError(String::from("data length too short")));
            }
            let (num, prop) = Property::parse(&bin[pos..])?;
            pos += num;
            edata.data.push(prop);
        }

        Ok(edata)
    }
}

impl Property {
    fn parse(bin: &[u8]) -> Result<(usize, Self)> {
        if bin.len() < 2 {
            return Err(Error::ParseError(String::from("empty data")));
        }

        let epc = bin[0];
        let pdc = bin[1] as usize;
        if bin.len() < 2 + pdc {
            return Err(Error::ParseError(String::from("less data length")));
        }
        let data = bin[2..pdc + 2].to_vec();
        let ret = Property { epc, data };
        Ok((2 + pdc, ret))
    }
}

#[cfg(test)]
mod test {
    mod edata_test {
        use crate::echonet::packet::{Edata, Property};

        #[test]
        fn parse_test() {
            let bin = hex::decode("02880105FF017202E7040000020EE7040000020F").unwrap();
            let expected = Edata {
                seoj: [0x02, 0x88, 0x01],
                deoj: [0x05, 0xFF, 0x01],
                esv: 0x72,
                opc: 0x02,
                data: vec![Property { epc: 0xE7, data: hex::decode("0000020E").unwrap() },
                           Property { epc: 0xE7, data: hex::decode("0000020F").unwrap() }],
            };
            assert_eq!(Edata::parse(bin.as_slice()).unwrap(), expected);
        }

        #[test]
        fn parse_test_less_property() {
            let bin = hex::decode("02880105FF017202E7040000020E").unwrap();
            assert_eq!(Edata::parse(bin.as_slice()).is_err(), true);
        }
    }

    mod property_test {
        use crate::echonet::packet::Property;

        #[test]
        fn parse_test_1() {
            let bin = hex::decode("E7040000020E").unwrap();
            let expected = Property { epc: 0xE7, data: hex::decode("0000020E").unwrap() };
            let (actual, data) = Property::parse(bin.as_slice()).unwrap();
            assert_eq!(data, expected);
            assert_eq!(actual, 6);
        }

        #[test]
        fn parse_test_2() {
            let bin = hex::decode("E7040000020EE704000FF20E").unwrap();
            let expected = Property { epc: 0xE7, data: hex::decode("0000020E").unwrap() };
            let (actual, data) = Property::parse(bin.as_slice()).unwrap();
            assert_eq!(data, expected);
            assert_eq!(actual, 6);
        }

        #[test]
        fn parse_error_on_empty() {
            let bin = hex::decode("").unwrap();
            assert_eq!(Property::parse(bin.as_slice()).is_err(), true);
        }

        #[test]
        fn parse_error_on_insufficient_length() {
            let bin = hex::decode("E704000002").unwrap();
            assert_eq!(Property::parse(bin.as_slice()).is_err(), true);
        }
    }
}
