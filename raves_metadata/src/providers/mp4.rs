//! MP4-related stuff.

use std::sync::Arc;

use parking_lot::RwLock;

use winnow::{
    Parser,
    error::{ContextError, EmptyError},
    token::take,
};

use crate::{
    MaybeParsedXmp, MetadataProvider, MetadataProviderRaw,
    providers::shared::bmff::{BoxHeader, BoxType, XMP_UUID, ftyp::FtypBox},
};

/// An MPEG-4 (MP4) file.
#[derive(Clone, Debug)]
pub struct Mp4 {
    xmp: Arc<RwLock<Option<MaybeParsedXmp>>>,
}

impl MetadataProviderRaw for Mp4 {
    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::clone(&self.xmp)
    }
}

impl MetadataProvider for Mp4 {
    type ConstructionError = Mp4ConstructionError;

    fn magic_number(input: &[u8]) -> bool {
        parse_ftyp(input).is_ok()
    }

    /// Reads the given data as an MP4 file.
    ///
    /// This operation extracts its metadata.
    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        parse(input.as_ref())
    }
}

/// Parses out the initial filetype information box (`ftyp`).
fn parse_ftyp(input: &[u8]) -> Result<(), Mp4ConstructionError> {
    let mut input = input;

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

    Ok(())
}

/// Parses out metadata from an MP4 file.
fn parse(mut input: &[u8]) -> Result<Mp4, Mp4ConstructionError> {
    // ensure we're working with an MP4 file...
    parse_ftyp(input)?;

    // check all the other boxes until we find what we want!
    let raw_xmp_bytes = parse_boxes_until_xmp(&mut input);

    Ok(Mp4 {
        xmp: Arc::new(RwLock::new(
            raw_xmp_bytes.map(|raw| MaybeParsedXmp::Raw(raw.to_vec())),
        )),
    })
}

fn parse_boxes_until_xmp<'input>(input: &mut &'input [u8]) -> Option<&'input [u8]> {
    while !input.is_empty() {
        // parse box
        let box_header: BoxHeader = match BoxHeader::new(input) {
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

/// An error that occurred when parsing an MP4.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum Mp4ConstructionError {
    /// The filetype box is required to continue parsing, but there wasn't one!
    NoFtypBox,

    /// The given file isn't actually an MP4.
    ///
    /// Its filetype info denoted that it's something else:
    NotAnMp4([u8; 4]),
}

impl core::error::Error for Mp4ConstructionError {}

impl core::fmt::Display for Mp4ConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mp4ConstructionError::NoFtypBox => f.write_str(
                "No `ftyp`/filetype box was found in the MP4 file, \
                but one is required to continue parsing.",
            ),
            Mp4ConstructionError::NotAnMp4(ftyp) => {
                let maybe_ftyp_str = str::from_utf8(ftyp);

                if let Ok(ftyp_str) = maybe_ftyp_str {
                    write!(
                        f,
                        "The `ftyp`/filetype box indicated that this \
                        file was not an MP4. \
                        Instead, it's a: `{ftyp:?}` (ASCII: `{ftyp_str}`)",
                    )
                } else {
                    write!(
                        f,
                        "The `ftyp`/filetype box indicated that this \
                        file was not an MP4. \
                        Instead, it's: `{ftyp:?}` (ASCII conv. failed)",
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::xmp::{XmpElement, XmpPrimitive, XmpValue};

    use crate::{MetadataProvider, providers::mp4::Mp4, util::logger};

    #[test]
    fn parse_real_mp4() {
        logger();

        let bytes = include_bytes!("../../assets/01_simple_with_aves_tags.mp4");

        let mp4: Mp4 = Mp4::new(&bytes).expect("parsing mp4 should work");

        let xmp = mp4
            .xmp()
            .expect("this file has XMP embedded")
            .expect("should find the XMP data");
        let locked_xmp = xmp.read();

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
                            c.value = XmpValue::Simple(XmpPrimitive::Text(format!("tag {v}")));
                            c
                        })
                        .collect(),
                ),
            },
        ]);

        let mut got = locked_xmp.document().values_ref().to_vec();
        got.sort_by_key(|a| a.name.clone());

        assert_eq!(got, expected);
    }
}
