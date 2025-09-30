//! JPEG is an older image format designed with old-school compression in mind.
//!
//! It uses an unfortunate internal structure that's difficult to parse and
//! edit, so this crate treads lightly.

use crate::{MaybeParsedExif, MaybeParsedXmp, MetadataProvider, MetadataProviderRaw};
use parking_lot::RwLock;
use std::sync::Arc;

mod error;
mod parse;

pub use error::JpegConstructionError;

/// A JPEG file.
#[derive(Clone, Debug)]
pub struct Jpeg {
    exif: Arc<RwLock<Option<MaybeParsedExif>>>,
    xmp: Arc<RwLock<Option<MaybeParsedXmp>>>,
}

impl MetadataProviderRaw for Jpeg {
    fn exif_raw(&self) -> Arc<RwLock<Option<MaybeParsedExif>>> {
        Arc::clone(&self.exif)
    }

    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::clone(&self.xmp)
    }
}

impl MetadataProvider for Jpeg {
    type ConstructionError = JpegConstructionError;

    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        parse::parse(input.as_ref())
    }
}
