#![forbid(unsafe_code)]

use std::collections::HashMap;

use error::IptcError;

pub mod error;
pub mod providers;
pub mod util;

pub type Pairs = HashMap<String, Vec<String>>;

/// Parsed IPTC.
pub struct Iptc {
    pub pairs: Pairs,
}

/// A media file with IPTC support.
///
/// Each file format is a "provider" - it'll yield its IPTC through parsing.
pub trait IptcProvider {
    /// Parses `self`, a media source, for its IPTC block(s) and returns them
    /// combined into one list of (key, value) pairs.
    fn iptc(&self) -> Result<Iptc, IptcError>;
}
