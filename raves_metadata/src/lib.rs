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

/// A media file with support for various metadata formats.
///
/// Each file format is a "provider" - it'll yield its metdata through parsing.
pub trait MetadataProvider {
    /// Parses `self`, a media source, for its IPTC block(s) and returns them
    /// combined into one list of (key, value) pairs.
    fn iptc(&self) -> Result<Iptc, IptcError>;

    // fn exif(&self) -> Result<Exif, ExifError>;

    // fn xmp(&self) -> Result<Xmp, XmpError>;
}

pub enum IptcKey {}
