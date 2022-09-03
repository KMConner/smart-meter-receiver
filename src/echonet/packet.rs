use crate::echonet::{Error, Result};

struct EchonetPacket {
    ehd1: u8,
    ehd2: u8,
    tid: u16,
    edaata: Edata,
}

struct Edata {
    seoj: [u8; 3],
    deoj: [u8; 3],
    esv: u8,
    opc: u8,
    data: Vec<Property>,
}

#[derive(PartialEq, Debug)]
struct Property {
    epc: u8,
    data: Vec<u8>,
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
