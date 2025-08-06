use winnow::{
    Parser,
    error::{ContextError, EmptyError},
    token::take,
};

use super::shared::bmff::parse_header;
use crate::{
    MetadataProvider,
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
    fn iptc(&self) -> Option<Result<crate::iptc::Iptc, crate::iptc::error::IptcError>> {
        let todo_impl_iptc_if_possible = ();
        None
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

    fn exif(&self) -> Result<crate::exif::Exif, crate::exif::error::ExifFatalError> {
        let todo_make_this_take_none = todo!();
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
