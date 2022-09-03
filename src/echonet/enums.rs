use num_enum::TryFromPrimitive;
use crate::echonet::{Error, Result};

#[repr(u64)]
#[derive(Debug, PartialEq, TryFromPrimitive, Copy, Clone)]
pub enum EchonetObject {
    SmartMeter = 0x028801,
    HemsController = 0x05FF01,
}

impl Into<[u8; 3]> for EchonetObject {
    fn into(self) -> [u8; 3] {
        let u = self as u64;
        let b0 = (u >> 16) as u8;
        let b1 = (u >> 8) as u8;
        let b2 = u as u8;
        [b0, b1, b2]
    }
}

impl TryFrom<[u8; 3]> for EchonetObject {
    type Error = Error;

    fn try_from(value: [u8; 3]) -> Result<Self> {
        let num = ((value[0] as u64) << 16) + ((value[1] as u64) << 8) + value[2] as u64;
        match num.try_into() {
            Ok(e) => Ok(e),
            Err(_) => {
                return Err(Error::InvalidEchonetObjectId(format!("{:?}", value)));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::echonet::enums::EchonetObject;

    #[test]
    fn into_slice_test() {
        let expected = hex::decode("028801").unwrap();
        let actual: [u8; 3] = EchonetObject::SmartMeter.into();
        let actual = actual.to_vec();
        assert_eq!(expected, actual);
    }

    #[test]
    fn from_slice_test() {
        let expected = EchonetObject::SmartMeter;
        let actual = [0x02, 0x88, 0x01].try_into().unwrap();
        assert_eq!(expected, actual);
    }
}