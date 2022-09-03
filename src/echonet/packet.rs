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

struct Property {
    epc: u8,
    data: Vec<u8>,
}
