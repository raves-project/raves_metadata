use winnow::{
    Parser,
    error::{ContextError, EmptyError},
    token::take,
};

use super::shared::bmff::parse_header;
use crate::{
    MetadataProvider,
    exif::{Exif, error::ExifFatalError},
    providers::shared::bmff::{BoxHeader, BoxType, XMP_UUID, ftyp::FtypBox},
    xmp::{Xmp, error::XmpError},
};

#[derive(Debug)]
pub struct Mp4<'input> {
    xmp: Option<&'input [u8]>,
}

impl Mp4<'_> {
    /// Reads the given data as an MP4 file.
    ///
    /// This operation extracts its metadata.
    pub fn new(input: &[u8]) -> Result<Mp4, Mp4ConstructionError> {
        parse(input)
    }
}

impl MetadataProvider for Mp4<'_> {
    fn exif(&self) -> Option<Result<Exif, ExifFatalError>> {
        None // MP4 doesn't support Exif
    }

    fn iptc(&self) -> Option<Result<crate::iptc::Iptc, crate::iptc::error::IptcError>> {
        None // container has no IPTC support
    }

    fn xmp(&self) -> Option<Result<Xmp, XmpError>> {
        let Some(raw_xmp_data) = self.xmp else {
            log::trace!("No XMP data on this MP4.");
            return None;
        };

        let xmp_str = match core::str::from_utf8(raw_xmp_data) {
            Ok(s) => s,
            Err(e) => {
                log::error!("The provided MP4's XMP data was not valid UTF-8. err: {e}");
                return Some(Err(XmpError::NotUtf8));
            }
        };

        Some(crate::Xmp::new(xmp_str))
    }
}

/// Parses out metadata from an MP4 file.
fn parse(mut input: &[u8]) -> Result<Mp4, Mp4ConstructionError> {
    // grab the ftyp box!
    //
    // it's almost always the first box in the file...
    let first_box: FtypBox = FtypBox::new(&mut input).ok_or_else(|| {
        log::error!("Didn't find first box in MP4 file!");
        Mp4ConstructionError::NoFtypBox
    })?;

    // ensure the format is MP4
    const MP4_FORMATS: &[&[u8; 4]] = &[b"iso2", b"isom", b"mp41", b"mp42"];
    let major_is_mp4 = MP4_FORMATS.contains(&&first_box.major_brand);
    let compat_with_mp4 = MP4_FORMATS
        .iter()
        .any(|fourcc| first_box.compatible_brands.contains(fourcc));

    if !(major_is_mp4 || compat_with_mp4) {
        log::warn!(
            "The provided file is not an MP4/MP4-like file. \
            major_brand: `{}`, \
            compatible_brands: `{:?}`",
            core::str::from_utf8(&first_box.major_brand).unwrap_or_default(),
            first_box
                .compatible_brands
                .iter()
                .map(|fourcc: &[u8; 4]| core::str::from_utf8(fourcc))
        );
        return Err(Mp4ConstructionError::NotAnMp4(first_box.major_brand));
    }

    // check all the other boxes until we find what we want!
    let raw_xmp_bytes = parse_boxes_until_xmp(&mut input);

    Ok(Mp4 { xmp: raw_xmp_bytes })
}

fn parse_boxes_until_xmp<'input>(input: &mut &'input [u8]) -> Option<&'input [u8]> {
    while !input.is_empty() {
        // parse box
        let box_header: BoxHeader = match parse_header(input) {
            Ok(b) => b,
            Err(e) => {
                log::error!("Failed to parse box header! err: {e}");
                break;
            }
        };

        // check if it's the right type
        let BoxType::Uuid(uuid) = box_header.box_type else {
            // if this is the last box, stop looping!
            let Some(payload_len) = box_header.payload_len() else {
                break;
            };

            // otherwise, skip the num. of bytes from the payload
            _ = take::<_, _, EmptyError>(payload_len)
                .void()
                .parse_next(input)
                .inspect_err(|_| log::error!("can't take payload len tokens! payload len: `{payload_len}`, slice len: `{}`", input.len()));
            continue;
        };

        if uuid == XMP_UUID {
            log::trace!("XMP UUID found!");

            // if this is (somehow) the last thing in the file, it won't have a
            // known end point.
            //
            // in that case, we just continue reading the slice until the end!
            // but, that actually means we can just return the mutated slice...
            return if let Some(payload_len) = box_header.payload_len() {
                match take::<_, _, ContextError>(payload_len).parse_next(input) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        log::error!("Failed to take `{payload_len}` (payload len) bytes! err: {e}");
                        continue;
                    }
                }
            } else {
                Some(*input)
            };
        }
    }

    // by default, return nothing :(
    None
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mp4ConstructionError {
    /// The filetype box is required to continue parsing, but there wasn't one!
    NoFtypBox,

    /// The given file isn't actually an MP4.
    ///
    /// Its filetype info denoted that it's something else:
    NotAnMp4([u8; 4]),
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::xmp::{XmpElement, XmpPrimitive, XmpValue};

    use crate::{
        MetadataProvider,
        providers::mp4::Mp4,
        xmp::{Xmp, XmpDocument},
    };

    #[test]
    fn parse_real_mp4() {
        logger();

        let bytes = include_bytes!("../../assets/01_simple_with_aves_tags.mp4");

        let mp4: Mp4 = Mp4::new(bytes).expect("parsing mp4 should work");

        let xmp: Xmp = mp4
            .xmp()
            .expect("this file has XMP embedded")
            .expect("should find the XMP data");

        let xmp_document: XmpDocument = xmp.parse().expect("parse XMP");

        let common_array_element: XmpElement = XmpElement {
            namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
            prefix: "rdf".into(),
            name: "li".into(),
            value: XmpValue::Simple(XmpPrimitive::Text("".into())),
        };

        let expected = Vec::from([
            XmpElement {
                namespace: "http://ns.adobe.com/xap/1.0/".into(),
                prefix: "xmp".into(),
                name: "MetadataDate".into(),
                value: XmpValue::Simple(XmpPrimitive::Date("2025-08-05T22:08:44-05:00".into())),
            },
            XmpElement {
                namespace: "http://ns.adobe.com/xap/1.0/".into(),
                prefix: "xmp".into(),
                name: "ModifyDate".into(),
                value: XmpValue::Simple(XmpPrimitive::Date("2025-08-05T22:08:44-05:00".into())),
            },
            XmpElement {
                namespace: "http://purl.org/dc/elements/1.1/".into(),
                prefix: "dc".into(),
                name: "subject".into(),
                value: XmpValue::UnorderedArray(
                    [1, 2, 3]
                        .into_iter()
                        .map(|v| {
                            let mut c = common_array_element.clone();
                            c.value =
                                XmpValue::Simple(XmpPrimitive::Text(format!("tag {v}").into()));
                            c
                        })
                        .collect(),
                ),
            },
        ]);

        let mut got = xmp_document.values_ref().to_vec();
        got.sort_by_key(|a| a.name.clone());

        assert_eq!(got, expected);
    }

    /// helper: init the logger
    fn logger() {
        env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .init();
    }
}
