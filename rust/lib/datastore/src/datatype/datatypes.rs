/// 32-bit markers for datatypes, one for each.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum DatatypeId {
    // Basic types : 0..

    // Complex types: 001..

    // Composite types: 002..

    // Object-like types: 003..
    Json = 0x00300000,

    // System types: 8..
    Thunderhead = 0x80000000,
}
