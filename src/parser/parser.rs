use crate::parser::messages::{ParseResult, SerialMessage};

struct WiSunModuleParser {
    pending_message: Option<String>,
}

impl WiSunModuleParser {
    pub fn new() -> Self {
        WiSunModuleParser {
            pending_message: None
        }
    }

    pub fn add_line() -> ParseResult<SerialMessage> {
        todo!();
    }
}

#[cfg(test)]
mod test{

}

