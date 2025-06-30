//! Contains a metadata provider for the PNG format.

use crate::{
    MetadataProvider,
    iptc::{Iptc, error::IptcError},
    xmp::{Xmp, error::XmpError},
};

/// PNG, or the Portable Network Graphics format, is a common image format as
/// of writing.
///
/// It can store all three supported metadata standards directly in the file.
pub struct Png<'file> {
    file: &'file [u8],
}

impl<'file> Png<'file> {
    pub fn new(file: &'file [u8]) -> Self {
        Self { file }
    }
}

impl<'file> MetadataProvider for Png<'file> {
    fn iptc(&self) -> Result<Iptc, IptcError> {
        todo!()
    }

    fn xmp(&self) -> Result<Xmp, XmpError> {
        todo!()
    }
}
