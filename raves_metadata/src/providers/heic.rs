//! HEIC, the "High-Efficiency Image Codec", is a high-efficiency image format.
//!
//! It uses HEVC for encoding, which is efficient, but results in licensing
//! issues in real-world uses.
//!
//! You might prefer AVIF for better efficiency and freely permitted usage.

use crate::{
    MetadataProvider,
    exif::{Exif, error::ExifFatalError},
    iptc::{Iptc, error::IptcError},
    providers::shared::bmff::heif::{HeifLike, HeifLikeConstructionError},
    xmp::{Xmp, error::XmpError},
};

const SUPPORTED_HEIC_BRANDS: &[[u8; 4]] = &[*b"heic"];

/// A HEIC file.
pub struct Heic<'input> {
    heic_like: HeifLike<'input>,
}

impl<'input> Heic<'input> {
    /// Constructs a HEIC representation from the given input blob.
    pub fn new(mut input: &'input [u8]) -> Result<Self, HeifLikeConstructionError> {
        HeifLike::new(&mut input, SUPPORTED_HEIC_BRANDS).map(|heic_like| Heic { heic_like })
    }
}

impl<'input> MetadataProvider for Heic<'input> {
    fn exif(&self) -> Option<Result<Exif, ExifFatalError>> {
        self.heic_like.exif()
    }

    fn iptc(&self) -> Option<Result<Iptc, IptcError>> {
        self.heic_like.iptc()
    }

    fn xmp(&self) -> Option<Result<Xmp, XmpError>> {
        self.heic_like.xmp()
    }
}
