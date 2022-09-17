use std::collections::HashSet;
use crate::echonet::{EchonetProperty, Error};
use super::errors::Result;

struct PropertyMap {
    properties: HashSet<u8>,
}

impl PropertyMap {
    pub fn parse(bin: &[u8]) -> Result<PropertyMap> {
        if bin.len() == 0 {
            return Err(Error::ParseError(String::from("empty data")));
        }

        if bin[0] < 16 {
            return Ok(PropertyMap {
                properties: HashSet::from_iter(bin[1..].iter().map(|i| *i)),
            });
        }

        if bin.len() != 17 {
            return Err(Error::ParseError(String::from("data length MUST be equal to 17")));
        }

        let map = &bin[1..];
        let mut props = HashSet::with_capacity(bin[0] as usize);
        for i in 0..16 {
            if map[i] == 0 {
                continue;
            }

            for j in 0..8 {
                if (map[i] & (0x01u8 << j)) != 0 {
                    props.insert((i as u8) + ((8 + j) << 4));
                }
            }
        }

        if props.len() != bin[0] as usize {
            return Err(Error::ParseError(String::from("property count is wrong")));
        }

        Ok(PropertyMap { properties: props })
    }

    pub fn has_property(&self, prop: impl EchonetProperty) -> bool {
        self.properties.contains(&prop.into())
    }
}

#[cfg(test)]
mod test {
    mod parse_test {
        use std::collections::HashSet;
        use crate::echonet::property_map::PropertyMap;

        #[test]
        fn parse_short() {
            let map = PropertyMap::parse(&vec![0x0A, 0x80, 0x81, 0x82, 0x83, 0x88, 0x8A, 0x9D, 0x9E, 0x9F, 0xE0]).unwrap();
            assert_eq!(HashSet::from_iter(vec![0x80, 0x81, 0x82, 0x83, 0x88, 0x8A, 0x9D, 0x9E, 0x9F, 0xE0].iter().map(|i| *i)), map.properties);
        }

        #[test]
        fn parse_long() {
            let map = PropertyMap::parse(&vec![0x16, 0x0B, 0x01, 0x01, 0x09, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03]).unwrap();
            assert_eq!(HashSet::from_iter(vec![0x80, 0x81, 0x82, 0x83, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x8E, 0x8F, 0x90, 0x9A, 0x9B, 0x9C, 0x9D, 0x9E, 0x9F, 0xB0, 0xB3].iter().map(|i| *i)), map.properties);
        }
    }
}
