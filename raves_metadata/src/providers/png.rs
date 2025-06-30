//! Contains a metadata provider for the PNG format.

use crate::{
    MetadataProvider,
    iptc::{Iptc, error::IptcError},
    xmp::{Xmp, error::XmpError},
};
use winnow::{
    binary::be_u32,
    error::{ContextError, ErrMode, StrContext, StrContextValue},
    prelude::*,
    token::{literal, rest, take},
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
        let xml: &'file str =
            get_xmp_block(self.file).expect("TODO: add an error variant for this");

        let xmp: Xmp = crate::xmp::Xmp::new(xml).expect("TODO: add an error variant for this");

        Ok(xmp)
    }
}

/// A header for a PNG "chunk"
struct PngChunkHeader {
    pub chunk_length: u32,    // in bytes
    pub chunk_ident: [u8; 4], // four ascii letters
}

/// From the very start of the file, this function will find the XMP block and
/// return it as a normal (i.e., UTF-8) [`String`].
fn get_xmp_block(mut input: &[u8]) -> ModalResult<&str, ContextError> {
    // grab the PNG signature - should be the first eight bytes.
    //
    // this ensures we're working with a PNG. if we don't find the signature,
    // we'll immediately stop parsing
    log::trace!("Attempting to parse out the PNG signature...");
    take(8_usize)
        .and_then(parse_png_signature)
        .parse_next(&mut input)
        .inspect_err(|e| {
            log::warn!(
                "This \"PNG\" didn't contain its required PNG signature. \
                Is it actually a PNG..? \
                err: {e}"
            );
        })?;
    log::trace!("Found a PNG signature! Continuing with chunk parsing.");

    // recursively scan for a chunk w/ XMP
    loop {
        // grab the next header, if available
        log::trace!("Attempting to grab new chunk header...");
        let PngChunkHeader {
            chunk_length,
            chunk_ident,
        } = parse_chunk_header.parse_next(&mut input)?;
        log::trace!(
            "Found chunk with ident: {}",
            core::str::from_utf8(&chunk_ident).unwrap_or("not UTF-8")
        );

        // if it's the right chunk, parse it and its data
        if &chunk_ident == b"iTXt" {
            log::trace!("Chunk is iTXt. Checking if it contains XMP...");
            let chunk_data = take(chunk_length as usize).parse_next(&mut input)?;
            _ = be_u32.parse_next(&mut input)?; // the next chunk'll be the crc; skip it!

            if let Some(xmp) = try_to_parse_xmp_from_itxt(chunk_data)? {
                log::trace!("Chunk contained XMP data!");
                return Ok(xmp);
            }
        } else {
            // payload + CRC
            log::trace!("Chunk was not iTXt. Skipping...");
            take(chunk_length as usize + 4).parse_next(&mut input)?;
        }
    }
}

/// Parses out the file's PNG signature.
fn parse_png_signature(input: &mut &[u8]) -> ModalResult<(), ContextError> {
    const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    literal(PNG_SIGNATURE).void().parse_next(input)
}

/// Parses out a chunk's header.
///
/// These are the first eight bytes on each chunk.
fn parse_chunk_header(input: &mut &[u8]) -> ModalResult<PngChunkHeader, ContextError> {
    let chunk_length: u32 = be_u32
        .context(StrContext::Label("chunk length"))
        .parse_next(input)?;

    let chunk_ident: [u8; 4] = take(4_usize)
        .context(StrContext::Label("ASCII chunk identifier"))
        .parse_next(input)?
        .try_into()
        .unwrap_or_else(|e| unreachable!("winnow already said this must be 4 bytes. but err: {e}"));

    Ok(PngChunkHeader {
        chunk_length,
        chunk_ident,
    })
}

/// We'll try to grab XMP from this iTXt.
///
/// If it's the right keyword, we'll return its data in `Some(data)`.
fn try_to_parse_xmp_from_itxt(mut input: &[u8]) -> ModalResult<Option<&str>, ContextError> {
    // we can increase performance with an early-return
    if !input.starts_with(b"XML:com.adobe.xmp") {
        log::trace!("Input doesn't contain the desired XMP keyword (marker). Moving on...");
        return Ok(None);
    }

    // let's grab the keyword!
    //
    // note that this is in ISO/IEC 8859-1, which means no character will be
    // `0x00`. in other words, we'll need to take letters until we find the
    // NUL byte
    log::trace!("Found expected keyword for XMP!");
    literal(b"XML:com.adobe.xmp")
        .void()
        .parse_next(&mut input)?;
    log::trace!("Ate XMP keyword. Continuing to grab from input...");

    // ok, we have that keyword.
    //
    // let's skip the NUL byte we know about now
    literal(0_u8).void().parse_next(&mut input)?;

    // the next thing will be the "compression flag", which, according to the
    // XMP specification, must always be `0` for an XMP block
    literal(0_u8)
        .context(StrContext::Expected(StrContextValue::Description(
            "to be marked as uncompressed text (0x0)",
        )))
        .void()
        .parse_next(&mut input)?;

    // after that is the "compression method" - it's also `0` for XMP
    literal(0_u8)
        .context(StrContext::Expected(StrContextValue::Description(
            "no specified compression method (0x0)",
        )))
        .void()
        .parse_next(&mut input)?;

    // there's another two NUL bytes after those
    literal(0_u8).void().parse_next(&mut input)?;
    literal(0_u8).void().parse_next(&mut input)?;

    // the rest of the input is XMP
    let the_rest: &[u8] = rest.parse_next(&mut input)?;

    // map it into a string
    core::str::from_utf8(the_rest)
        .map(|s: &str| Some(s))
        .map_err(|_e| {
            let mut ce = ContextError::new();
            ce.push(StrContext::Expected(
                winnow::error::StrContextValue::StringLiteral("XMP wasn't UTF-8!"),
            ));
            ErrMode::Cut(ce)
        })
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::xmp::{XmpElement, XmpValue};

    use crate::{MetadataProvider as _, providers::png::Png, xmp::Xmp};

    /// Checks that we can parse out a PNG signature.
    #[test]
    fn png_signature_parsing() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        assert_eq!(
            Ok(()),
            super::parse_png_signature(
                &mut [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].as_slice()
            ),
            "we should successfully parse a PNG signature",
        )
    }

    /// Ensures that we can parse out some XMP from a PNG.
    #[test]
    fn png_containing_xmp_parses_correctly() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        #[rustfmt::skip]
        let technically_a_png: Vec<u8> = [
            // png signature
           [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].as_slice(),

            // required: `IHDR` (image header)
            [

                0x0, 0x0, 0x0, 0xD,    // data length
                b'I', b'H', b'D', b'R', // type
                0x0, 0x0, 0xA, 0x0,     // res: width (2560_u32)
                0x0, 0x0, 0x5, 0xA0,    // res: height (1440_u32)
                8,                      // bit depth (8)
                6,                      // color type (truecolor w/ alpha)
                0, 0, 0,                // compression, filter, interlace (all off)
                0xB3, 0xE4, 0x34, 0x52, // CRC32
            ].as_slice(),

            // a junk `iTXt` that we don't care about
            [
                0x0, 0x0, 0x0, 0x1D, // data length
                b'i', b'T', b'X', b't', // type
                b'S', b'o', b'f', b't', b'w', b'a', b'r', b'e', b'\0', // keyword
                0x00, 0x00, // compression (off)
                b'e', b'n', b'-', b'U', b'S', b'\0', // language tag
                b'S', b'o', b'f', b't', b'w', b'a', b'r', b'e', b'\0', // translated keyword
                b'H', b'i', b'!', // text
                0x69, 0x5C, 0x21, 0xB2, // CRC
            ].as_slice(),

            // a good `iTXt` with useful data
            [
                // header + data up to XML
                [
                    0x0, 0x0, 0x1, 0x67, // data length (363_u32)
                    b'i', b'T', b'X', b't', // type
                    b'X', b'M', b'L', b':', b'c', b'o', b'm', b'.', b'a', b'd', b'o', b'b', b'e', b'.', b'x', b'm', b'p', b'\0', // keyword
                    b'\0', // language tag (none)
                    b'\0', // translated keyword (none)
                    0x00, 0x00, // compression (off)
                ].as_slice(),

                // XMP UTF-8 data as a byte slice
                r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                    <rdf:Description rdf:about="" xmlns:my_ns="https://barretts.club">
                    <my_ns:MyStruct>
                        <rdf:Description />
                    </my_ns:MyStruct>
                    </rdf:Description>
                </rdf:RDF>"#.as_bytes(),

                [0xDC, 0xFD, 0x6E, 0x88].as_slice(), // CRC32
            ]
            .into_iter()
            .flat_map(|sli| sli.iter().copied())
            .collect::<Vec<_>>()
            .as_slice(),

            // required: `IEND`
            [0x49, 0x45, 0x4E, 0x44].as_slice(),
        ]
        .into_iter()
        .flat_map(|sli| sli.iter().copied())
        .collect();

        // with that all over, we can actually run the test ;D
        let png: Png = Png::new(&technically_a_png);
        let xmp: Xmp = png.xmp().expect("get XMP from PNG");

        let parsed_xmp = xmp.parse().expect("parse XMP data");

        assert_eq!(
            parsed_xmp.values_ref().len(),
            1_usize,
            "should only parse that one struct"
        );
        assert_eq!(
            parsed_xmp.values_ref().first().expect("must have an item"),
            &XmpElement {
                namespace: "https://barretts.club".into(),
                prefix: "my_ns".into(),
                name: "MyStruct".into(),
                value: XmpValue::Struct(Vec::new()),
            },
            "found struct should match the expected (right) side"
        )
    }
}
