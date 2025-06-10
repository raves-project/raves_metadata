use std::collections::HashMap;

pub mod providers;

/// Parsed IPTC.
pub struct Iptc {
    pub pairs: HashMap<String, Vec<String>>,
}

/// A media file with IPTC support.
///
/// Each file format is a "provider" - it'll yield its IPTC through parsing.
pub trait IptcProvider {
    /// Parses `self`, a media source, for its IPTC block(s) and returns them
    /// combined into one list of (key, value) pairs.
    fn iptc(&self) -> Iptc;
}
