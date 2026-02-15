//! AVIF, the "AV1 Video Format", is a high-efficiency image format.

use crate::{
    MetadataProvider,
    providers::shared::bmff::heif::{HeifLike, HeifLikeConstructionError},
};

/// Supported brands for AVIF files.
pub const SUPPORTED_AVIF_BRANDS: &[[u8; 4]] = &[*b"avif", *b"avis"];

/// An AVIF file.
#[derive(Clone, Debug)]
pub struct Avif {
    heic_like: HeifLike,
}

impl MetadataProvider for Avif {
    type ConstructionError = HeifLikeConstructionError;

    fn magic_number(input: &[u8]) -> bool {
        HeifLike::parse_magic_number(input, SUPPORTED_AVIF_BRANDS)
    }

    /// Constructs a new AVIF file representation using the `input` blob.
    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        HeifLike::parse(&mut input.as_ref(), SUPPORTED_AVIF_BRANDS)
            .map(|heic_like| Avif { heic_like })
    }

    fn exif(&self) -> &Option<Result<crate::exif::Exif, crate::exif::error::ExifFatalError>> {
        &self.heic_like.exif
    }

    fn xmp(&self) -> &Option<Result<crate::xmp::Xmp, crate::xmp::error::XmpError>> {
        &self.heic_like.xmp
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

    use crate::{MetadataProvider as _, exif::Ifd, providers::avif::Avif, util::logger};

    #[test]
    fn sample_img_meta_after_img_blob_should_parse() {
        logger();

        let bytes = include_bytes!("../../assets/providers/avif/exif_xmp_after_image_blob.avif");
        let file: Avif = Avif::new(bytes).unwrap();

        // construct the xmp
        let xmp = file
            .xmp()
            .clone()
            .expect("XMP is supported + provided in file")
            .expect("XMP should be present");

        let mut xmp_values = xmp.document().values_ref().to_vec();
        xmp_values.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap());

        assert_eq!(
            *xmp_values.first().unwrap(),
            XmpElement {
                namespace: "http://www.gimp.org/xmp/".into(),
                prefix: "GIMP".into(),
                name: "TimeStamp".into(),
                value: XmpValue::Simple(XmpPrimitive::Text("1613247941462908".into()))
            },
        );

        // parse exif
        let mut exif = file
            .exif()
            .clone()
            .expect("exif should be supported")
            .expect("exif should be found");

        // ensure only one ifd
        assert_eq!(exif.ifds.len(), 1, "should only be one ifd");
        let ifd: Ifd = exif.ifds.remove(0);

        // grab same gimp timestamp above
        let gimp_timestamp: Vec<Field> = ifd
            .fields
            .into_iter()
            .flatten()
            .filter(|field| field.tag == FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::DateTime)))
            .collect();

        assert_eq!(
            *gimp_timestamp,
            vec![Field {
                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::DateTime)),
                data: FieldData::List {
                    list: {
                        b"2021:02:13 21:25:32\0" // yea... that escape confused me for like 20 min D:
                            .iter()
                            .map(|cha| Primitive::Ascii(*cha))
                            .collect()
                    },
                    ty: PrimitiveTy::Ascii
                }
            }]
        )
    }

    #[test]
    fn sample_img_meta_before_img_blob_should_parse() {
        logger();

        let bytes = include_bytes!("../../assets/providers/avif/exif_xmp_before_image_blob.avif");
        let file: Avif = Avif::new(bytes).unwrap();

        let xmp = file
            .xmp()
            .clone()
            .expect("XMP is supported + provided in file")
            .expect("XMP should be present");

        let mut xmp_values = xmp.document().values_ref().to_vec();
        xmp_values.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap());

        assert_eq!(
            *xmp_values
                .iter()
                .find(|v| v.name == "AuthorsPosition")
                .unwrap(),
            XmpElement {
                namespace: "http://ns.adobe.com/photoshop/1.0/".into(),
                prefix: "photoshop".into(),
                name: "AuthorsPosition".into(),
                value: XmpValue::Simple(XmpPrimitive::Text("Computer Scientist".into()))
            },
        );

        let mut exif = file
            .exif()
            .clone()
            .expect("exif should be supported")
            .expect("exif should be found");

        assert_eq!(exif.ifds.len(), 1, "should only be one ifd");
        let ifd: Ifd = exif.ifds.remove(0);

        let gimp_timestamp: Vec<Field> = ifd
            .fields
            .into_iter()
            .flatten()
            .filter(|field| field.tag == FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::DateTime)))
            .collect();

        assert_eq!(
            *gimp_timestamp,
            vec![Field {
                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::DateTime)),
                data: FieldData::List {
                    list: {
                        b"2021:02:13 21:19:50\0"
                            .iter()
                            .map(|cha| Primitive::Ascii(*cha))
                            .collect()
                    },
                    ty: PrimitiveTy::Ascii
                }
            }]
        )
    }

    /// from Big Buck Bunny
    #[test]
    fn sample_img_tiny_exif_should_parse() {
        logger();

        let bytes = include_bytes!("../../assets/providers/avif/bbb_4k.avif");
        let file: Avif = Avif::new(bytes).unwrap();

        // ensure that iptc doesn't work (not supported)
        assert!(
            file.iptc().is_none(),
            "to my knowledge, iptc isn't supported in HEIC formats"
        );

        // ensure that there's no xmp
        assert!(file.xmp().is_none(), "file only has exif - no xmp.");

        // parse exif
        let mut exif = file
            .exif()
            .clone()
            .expect("exif should be supported + found")
            .expect("exif should be well-formed");

        // ensure only one ifd
        assert_eq!(exif.ifds.len(), 1, "should only be one ifd");
        let ifd: Ifd = exif.ifds.remove(0);

        // grab same gimp timestamp above
        let gimp_timestamp: Vec<Field> = ifd
            .fields
            .into_iter()
            .flatten()
            .filter(|field| field.tag == FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::Copyright)))
            .collect();

        assert_eq!(
            *gimp_timestamp,
            vec![Field {
                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::Copyright)),
                data: FieldData::List {
                    list: {
                        b"Blender Foundation 2008, Janus Bager Kristensen 2013 - Creative Commons Attribution 3.0 - http://bbb3d.renderfarming.net\0"
                            .iter()
                            .map(|cha| Primitive::Ascii(*cha))
                            .collect()
                    },
                    ty: PrimitiveTy::Ascii
                }
            }]
        )

        // parse exif
    }
}
