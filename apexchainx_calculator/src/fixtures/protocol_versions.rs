//! Protocol versions supported by the fixture audit.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolVersion {
    V1,
    V2,
}

impl ProtocolVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolVersion::V1 => "v1",
            ProtocolVersion::V2 => "v2",
        }
    }
}