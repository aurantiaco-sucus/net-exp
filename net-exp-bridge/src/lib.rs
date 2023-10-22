use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Address {
    pub data: [u8; 4]
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let a1 = self.data[0];
        let a2 = self.data[1];
        let a3 = self.data[2];
        let a4 = self.data[3];
        write!(f, "{a1:02x}:{a2:02x}:{a3:02x}:{a4:02x}")
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Segment {
    pub data: [u8; 2]
}

impl Display for Segment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let a1 = self.data[0];
        let a2 = self.data[1];
        write!(f, "{a1:02x}:{a2:02x}")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Frame {
    src: Address,
    dst: Address,
    data: [u8; 16]
}