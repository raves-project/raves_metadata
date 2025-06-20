use raves_metadata_types::iptc::IptcKeyValue;

mod iptc4xmp;

/// Parsed IPTC.
pub struct Iptc {
    pub pairs: Vec<IptcKeyValue>,
}
