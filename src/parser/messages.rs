use crate::parser::event::WiSunEvent;

#[derive(Debug, PartialEq)]
pub enum ParseResult<T> {
    Ok(T),
    Err(String),
    Empty,
    More,
}

#[derive(Debug, PartialEq)]
pub enum SerialMessage {
    Ok,
    Fail(String),
    Event(WiSunEvent),
    Unknown(String),
}

#[cfg(test)]
mod test{

}