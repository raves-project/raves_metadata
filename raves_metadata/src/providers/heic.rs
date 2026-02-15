//! HEIC, the "High-Efficiency Image Codec", is a high-efficiency image format.
//!
//! It uses HEVC for encoding, which is efficient, but results in licensing
//! issues in real-world uses.
//!
//! You might prefer AVIF for better efficiency and freely permitted usage.

use crate::{
    MetadataProvider,
    providers::shared::bmff::heif::{HeifLike, HeifLikeConstructionError},
};

const SUPPORTED_HEIC_BRANDS: &[[u8; 4]] = &[*b"heic"];

/// A HEIC file.
#[derive(Clone, Debug)]
pub struct Heic {
    heic_like: HeifLike,
}

impl MetadataProvider for Heic {
    type ConstructionError = HeifLikeConstructionError;

    fn magic_number(input: &[u8]) -> bool {
        HeifLike::parse_magic_number(input, SUPPORTED_HEIC_BRANDS)
    }

    /// Constructs a HEIC representation from the given input blob.
    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        HeifLike::parse(&mut input.as_ref(), SUPPORTED_HEIC_BRANDS)
            .map(|heic_like| Heic { heic_like })
    }

    fn exif(&self) -> Option<Result<&crate::exif::Exif, &crate::exif::error::ExifFatalError>> {
        self.heic_like.exif.as_ref().map(|r| r.as_ref())
    }

    fn xmp(&self) -> Option<Result<&crate::xmp::Xmp, &crate::xmp::error::XmpError>> {
        self.heic_like.xmp.as_ref().map(|r| r.as_ref())
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

        // grab its exif ifd
        let exif_ifd = exif
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
