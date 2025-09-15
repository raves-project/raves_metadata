//! AVIF, the "AV1 Video Format", is a high-efficiency image format.

use crate::{
    MetadataProvider,
    exif::{Exif, error::ExifFatalError},
    iptc::{Iptc, error::IptcError},
    providers::shared::bmff::heif::{HeifLike, HeifLikeConstructionError},
    xmp::{Xmp, error::XmpError},
};

/// Supported brands for AVIF files.
pub const SUPPORTED_AVIF_BRANDS: &[[u8; 4]] = &[*b"avif", *b"avis"];

/// An AVIF file.
pub struct Avif<'input> {
    heic_like: HeifLike<'input>,
}

impl<'input> Avif<'input> {
    /// Constructs a new AVIF file representation using the `input` blob.
    pub fn new(mut input: &'input [u8]) -> Result<Self, HeifLikeConstructionError> {
        HeifLike::new(&mut input, SUPPORTED_AVIF_BRANDS).map(|heic_like| Avif { heic_like })
    }
}

impl<'input> MetadataProvider for Avif<'input> {
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
