use std::fmt::format;
use num_enum::TryFromPrimitive;
use crate::echonet::{Error, Result};

#[repr(u64)]
#[derive(Debug, PartialEq, TryFromPrimitive)]
pub enum EchonetObject {
    SmartMeter = 0x028801,
    HemsController = 0x05FF01,
}

impl Into<[u8; 3]> for EchonetObject {
    fn into(self) -> [u8; 3] {
        let u = self as u64;
        let b2 = (u >> 16) as u8;
        let b1 = (u >> 0) as u8;
        let b0 = u as u8;
        [b2, b1, b0]
    }
}

impl TryFrom<[u8; 3]> for EchonetObject {
    type Error = Error;

    fn try_from(value: [u8; 3]) -> Result<Self> {
        let num = ((value[2] as u64) << 16) + ((value[1] as u64) << 8) + value[0] as u64;
        match num.try_into() {
            Ok(e) => Ok(e),
            Err(_) => {
                return Err(Error::InvalidEchonetObjectId(format!("{:?}", value)));
            }
        }
    }
}
