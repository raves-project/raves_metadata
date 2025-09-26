//! HEIC, the "High-Efficiency Image Codec", is a high-efficiency image format.
//!
//! It uses HEVC for encoding, which is efficient, but results in licensing
//! issues in real-world uses.
//!
//! You might prefer AVIF for better efficiency and freely permitted usage.

use crate::{MetadataProvider, MetadataProviderRaw, providers::shared::bmff::heif::HeifLike};

const SUPPORTED_HEIC_BRANDS: &[[u8; 4]] = &[*b"heic"];

/// A HEIC file.
#[derive(Clone, Debug)]
pub struct Heic {
    heic_like: HeifLike,
}

impl MetadataProviderRaw for Heic {
    fn exif_raw(&self) -> std::sync::Arc<parking_lot::RwLock<Option<crate::MaybeParsedExif>>> {
        self.heic_like.exif_raw()
    }

    fn xmp_raw(&self) -> std::sync::Arc<parking_lot::RwLock<Option<crate::MaybeParsedXmp>>> {
        self.heic_like.xmp_raw()
    }
}

impl MetadataProvider for Heic {
    type ConstructionError = <HeifLike as MetadataProvider>::ConstructionError;

    /// Constructs a HEIC representation from the given input blob.
    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        HeifLike::parse(&mut input.as_ref(), SUPPORTED_HEIC_BRANDS)
            .map(|heic_like| Heic { heic_like })
    }
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::exif::{
        Field, FieldData, FieldTag,
        primitives::{Primitive, PrimitiveTy},
        tags::{ExifIfdTag, KnownTag},
    };

    use crate::{MetadataProvider, providers::heic::Heic, util::logger};

    #[test]
    fn nokia_conformance_file_c034_heic_parses_despite_malformed_exif_tiff_header_offset() {
        logger();

        let blob: &[u8] = include_bytes!("../../assets/providers/heic/C034.heic");

        // parse it into heic
        let file: Heic = Heic::new(&blob).expect("parse as heic");

        // it should only have exif
        assert!(file.iptc().is_none(), "iptc unsupported");
        assert!(file.xmp().is_none(), "xmp not present in file");

        // grab exif
        let exif = file
            .exif()
            .expect("file has exif")
            .expect("exif should be well-formed");
        let exif_locked = exif.read();

        // grab its exif ifd
        let exif_ifd = exif_locked
            .ifds
            .first()
            .expect("should have an ifd")
            .sub_ifds
            .first()
            .expect("should have one sub-ifd, which is ExifIFD");

        // ensure its "exif version" is 0230
        assert_eq!(
            *exif_ifd
                .fields
                .iter()
                .flatten()
                .find(|f| f.tag == FieldTag::Known(KnownTag::ExifIfdTag(ExifIfdTag::ExifVersion)))
                .expect("find ExifVersion field"),
            Field {
                tag: FieldTag::Known(KnownTag::ExifIfdTag(ExifIfdTag::ExifVersion)),
                data: FieldData::List {
                    list: [0x30, 0x32, 0x33, 0x30].map(Primitive::Undefined).into(),
                    ty: PrimitiveTy::Undefined
                }
            }
        );
    }
}
