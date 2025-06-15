#![forbid(unsafe_code)]

use error::IptcError;
use raves_iptc_types::IptcKeyValue;

pub mod error;
pub mod providers;
pub mod util;


/// Parsed IPTC.
pub struct Iptc {
    pub pairs: Vec<IptcKeyValue>,
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
