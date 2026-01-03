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

    fn magic_number(input: &[u8]) -> bool {
        parse::magic_number(input)
    }

    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        parse::parse(input.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::{
        exif::{
            Field, FieldData, FieldTag,
            primitives::{Primitive, PrimitiveTy},
            tags::{Ifd0Tag, KnownTag},
        },
        xmp::{XmpElement, XmpPrimitive, XmpValue},
    };

    use crate::{MetadataProvider, providers::jpeg::Jpeg, util::logger};

    #[test]
    fn real_jpeg_no_meta() {
        logger();

        let file = include_bytes!("../../../assets/providers/jpeg/Cat-in-da-hat.jpg");
        let jpeg = Jpeg::new(file).unwrap();

        assert!(jpeg.exif.read().is_none());
        assert!(jpeg.xmp.read().is_none());
    }

    #[test]
    fn real_jpeg_written_meta_with_exiftool() {
        logger();

        let file = include_bytes!("../../../assets/providers/jpeg/Calico_Cat_Asleep.jpg");
        let jpeg = Jpeg::new(file).unwrap();

        let exif = jpeg.exif().unwrap().unwrap();
        let xmp = jpeg.xmp().unwrap().unwrap();

        let locked_exif = exif.read();
        let locked_xmp = xmp.read();

        assert_eq!(
            locked_exif
                .ifds
                .first()
                .unwrap()
                .fields
                .iter()
                .flatten()
                .find(|f| f.tag == FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::Artist)))
                .unwrap(),
            &Field {
                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::Artist)),
                data: FieldData::List {
                    list: b"Am9489\0".iter().map(|b| Primitive::Ascii(*b)).collect(),
                    ty: PrimitiveTy::Ascii
                }
            }
        );

        assert_eq!(
            locked_xmp
                .document()
                .values_ref()
                .iter()
                .find(|f| f.prefix == "dc" && f.name == "subject")
                .unwrap(),
            &XmpElement {
                namespace: "http://purl.org/dc/elements/1.1/".into(),
                prefix: "dc".into(),
                name: "subject".into(),
                value: XmpValue::UnorderedArray(vec![
                    XmpElement {
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        name: "li".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("cat".into()))
                    },
                    XmpElement {
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        name: "li".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("cute".into()))
                    },
                ])
            }
        );
    }

    #[test]
    fn real_jpeg_with_meta_from_camera() {
        logger();

        let file = include_bytes!(
            "../../../assets/providers/jpeg/General_Rafael_Urdaneta_Bridge_view_from_the_lake_to_Cabimas_side.jpg"
        );
        let jpeg = Jpeg::new(file).unwrap();

        let exif = jpeg.exif().unwrap().unwrap();
        let exif_locked = exif.read();

        assert_eq!(
            exif_locked
                .ifds
                .first()
                .unwrap()
                .fields
                .iter()
                .flatten()
                .find(|f| f.tag == FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::Copyright)))
                .unwrap(),
            &Field {
                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::Copyright)),
                data: FieldData::List {
                    list: b"Creative Commons CC0 1.0 Universal Public Domain\0"
                        .iter()
                        .map(|b| Primitive::Ascii(*b))
                        .collect(),
                    ty: PrimitiveTy::Ascii
                }
            }
        );

        assert!(jpeg.xmp().is_none());
    }

    #[test]
    fn sample_jpeg_with_kinda_corrupted_fields() {
        logger();

        let file = include_bytes!(
            "../../../assets/providers/jpeg/Metadata test file - includes data in IIM, XMP, and Exif.jpg.jpg"
        );

        let jpeg = Jpeg::new(file).unwrap();

        // this file contains extended xmp, but no actual extendedxmp blocks.
        //
        // let's grab the concatenated version we made
        let xmp = jpeg.xmp().unwrap().unwrap();
        let locked_xmp = xmp.read();

        // should still contain the original tags.
        //
        // here's one of those:
        assert_eq!(
            locked_xmp
                .document()
                .values_ref()
                .iter()
                .find(|f| f.prefix == "aux" && f.name == "Lens")
                .unwrap(),
            &XmpElement {
                namespace: "http://ns.adobe.com/exif/1.0/aux/".into(),
                prefix: "aux".into(),
                name: "Lens".into(),
                value: XmpValue::Simple(XmpPrimitive::Text("Samsung Galaxy S7 Rear Camera".into()))
            }
        );
    }

    #[test]
    fn real_jpeg_with_hdr_and_extended_xmp() {
        logger();

        let file = include_bytes!("../../../assets/providers/jpeg/exiv2-bug922.jpg");

        let jpeg = Jpeg::new(file).unwrap();

        // this file contains extended xmp, but no actual extendedxmp blocks.
        //
        // let's grab the concatenated version we made
        let xmp = jpeg.xmp().unwrap().unwrap();
        let locked_xmp = xmp.read();

        assert!(
            locked_xmp
                .document()
                .values_ref()
                .iter()
                .any(|v| v.name == "Data" && v.prefix == "GImage")
        );
    }
}
