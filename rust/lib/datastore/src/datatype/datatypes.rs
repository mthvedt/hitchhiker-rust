/// 32-bit markers for datatypes, one for each.

// TODO: maybe separate general-purpose lenses from TD lenses?
#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum DatatypeId {
    // Basic types : 0..

    // Complex types: 001..

    // Composite types: 002..

    // Object-like types: 003..
    /// Javascript family starts with 0030.
    Javascript =    0x00300000,
    Json =          0x00310000,

    // DB types: 01..
    TdJson =        0x01000000,

    // System types: 8..
    Thunderhead =   0x80000000,
}
