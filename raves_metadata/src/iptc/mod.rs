use raves_metadata_types::iptc::IptcKeyValue;

pub mod error;
mod iptc4xmp;

/// Parsed IPTC.
#[derive(Clone, Debug, PartialEq)]
pub struct Iptc {
    pub pairs: Vec<IptcKeyValue>,
}

impl Iptc {
    /// Parses IPTC out of a byte slice/similar, assuming that byte slice
    /// contains XMP.
    pub fn new_xmp<B: AsRef<[u8]>>(raw: B) -> Result<Self, error::IptcError> {
        iptc4xmp::parse_xmp_for_iptc(raw.as_ref()).map_err(error::IptcError::Iptc4Xmp)
    }
}
