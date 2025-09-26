//! Contains a metadata provider for the PNG format.

use parking_lot::RwLock;
use std::sync::Arc;

use crate::{
    MetadataProvider, MetadataProviderRaw,
    util::{MaybeParsedExif, MaybeParsedXmp},
};
use winnow::{
    binary::be_u32,
    combinator::peek,
    error::{ContextError, EmptyError, ErrMode, StrContext, StrContextValue},
    prelude::*,
    token::{literal, rest, take},
};

/// A signature indicating that a file is a PNG.
pub const PNG_SIGNATURE: &[u8; 8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/// PNG, or the Portable Network Graphics format, is a common image format as
/// of writing.
///
/// It can store all three supported metadata standards directly in the file.
#[derive(Clone, Debug)]
pub struct Png {
    exif: Arc<RwLock<Option<MaybeParsedExif>>>,
    xmp: Arc<RwLock<Option<MaybeParsedXmp>>>,
}

impl MetadataProviderRaw for Png {
    fn exif_raw(&self) -> Arc<RwLock<Option<MaybeParsedExif>>> {
        Arc::clone(&self.exif)
    }

    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::clone(&self.xmp)
    }
}

impl MetadataProvider for Png {
    type ConstructionError = PngConstructionError;

    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        let mut input = input.as_ref();

        // grab the PNG signature - should be the first eight bytes.
        //
        // this ensures we're working with a PNG. if we don't find the signature,
        // we'll immediately stop parsing
        log::trace!("Attempting to parse out the PNG signature...");
        let signature: &[u8; 8] = take(8_usize)
            .parse_next(&mut input)
            .map(|s| TryInto::try_into(s).unwrap_or_else(|_| unreachable!()))
            .map_err(|e: ContextError| {
                log::warn!(
                    "This \"PNG\" didn't contain its required PNG signature. \
                    Is it actually a PNG..? \
                    err: {e}"
                );
                PngConstructionError::NoSignature
            })?;

        // ensure the signature is correct
        parse_png_signature.parse(signature).map_err(|e| {
            log::warn!(
                "Signature obtained from given file did not match a PNG! \
                err: {e}, \
                found: `{signature:?}`
                "
            );
            PngConstructionError::NotAPng { found: *signature }
        })?;

        log::trace!("Found a PNG signature! Continuing with chunk parsing.");

        // grab metadata by parsing chunks until we've found everything
        let GetMetadata { exif, xmp } = get_metadata(&mut input);

        // return any metadata we found inside this `self`...
        Ok(Self {
            exif: Arc::new(RwLock::new(exif.map(|p| MaybeParsedExif::Raw(p.into())))),
            xmp: Arc::new(RwLock::new(xmp.map(|r| MaybeParsedXmp::Raw(r.into())))),
        })
    }
}

/// A header for a PNG "chunk"
struct PngChunkHeader {
    pub chunk_length: u32,    // in bytes
    pub chunk_ident: [u8; 4], // four ascii letters
}

/// Parses out the file's PNG signature.
fn parse_png_signature(input: &mut &[u8]) -> ModalResult<(), ContextError> {
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

struct GetMetadata<'input> {
    exif: Option<&'input [u8]>,
    xmp: Option<&'input str>,
}

pub const EXIF_CHUNK_IDENT: [u8; 4] = *b"eXIf";

/// Parses through the PNG chunks to find metadata.
///
/// Continues until we run out of chunks, or all metadata has been located.
fn get_metadata<'input>(input: &mut &'input [u8]) -> GetMetadata<'input> {
    let mut metadata: GetMetadata = GetMetadata {
        exif: None,
        xmp: None,
    };

    // loop until we're out of input
    while !input.is_empty() {
        if metadata.exif.is_some() && metadata.xmp.is_some() {
            break;
        }

        // parse out chunk
        let Ok(PngChunkHeader {
            chunk_length,
            chunk_ident,
        }) = parse_chunk_header.parse_next(input)
        else {
            log::warn!("Failed to parse PNG chunk header!");
            break;
        };

        // log what we got
        log::trace!(
            "Found chunk with ident: `{}`",
            core::str::from_utf8(&chunk_ident).unwrap_or("not UTF-8")
        );

        // metadata: exif
        if chunk_ident == EXIF_CHUNK_IDENT {
            // try parsing out the actual exif data.
            match peek(take::<_, _, EmptyError>(chunk_length)).parse_next(input) {
                Ok(exif_blob) => {
                    _ = take::<_, _, EmptyError>(chunk_length)
                        .void()
                        .parse_next(input);
                    _ = take::<_, _, EmptyError>(4_usize).void().parse_next(input); // crc
                    log::trace!("Chunk had Exif data!");
                    metadata.exif = Some(exif_blob);
                    continue;
                }

                Err(_) => {
                    log::error!("Failed to parse out Exif blob from Exif chunk!");
                }
            }
        }

        // metadata: xmp
        if &chunk_ident == b"iTXt" {
            log::trace!("Chunk is iTXt. Checking if it contains XMP...");
            let Ok::<_, EmptyError>(ref mut chunk_data) =
                peek(take(chunk_length as usize)).parse_next(input)
            else {
                log::warn!(
                    "Couldn't find enough data inside `iTXt`! expected: `{chunk_length}`, got: `{}`",
                    input.len()
                );
                break;
            };

            let Ok(maybe_xmp) = try_to_parse_xmp_from_itxt(chunk_data) else {
                log::warn!("Failed to parse any XMP data from chunk!");
                break;
            };

            if let Some(xmp) = maybe_xmp {
                log::trace!("Chunk contained XMP data!");
                metadata.xmp = Some(xmp);
            }
        }

        // if we haven't `continue`d yet, we didn't get anything useful.
        //
        // skip the payload and crc...
        //
        // payload
        if take::<_, _, EmptyError>(chunk_length as usize)
            .void()
            .parse_next(input)
            .is_err()
        {
            break;
        };

        // crc
        if take::<_, _, EmptyError>(4_usize)
            .void()
            .parse_next(input)
            .is_err()
        {
            break;
        };
    }

    metadata
}

/// We'll try to grab XMP from this iTXt.
///
/// If it's the right keyword, we'll return its data in `Some(data)`.
fn try_to_parse_xmp_from_itxt<'input>(
    input: &mut &'input [u8],
) -> ModalResult<Option<&'input str>, ContextError> {
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
    literal(b"XML:com.adobe.xmp").void().parse_next(input)?;
    log::trace!("Ate XMP keyword. Continuing to grab from input...");

    // ok, we have that keyword.
    //
    // let's skip the NUL byte we know about now
    literal(0_u8).void().parse_next(input)?;

    // the next thing will be the "compression flag", which, according to the
    // XMP specification, must always be `0` for an XMP block
    literal(0_u8)
        .context(StrContext::Expected(StrContextValue::Description(
            "to be marked as uncompressed text (0x0)",
        )))
        .void()
        .parse_next(input)?;

    // after that is the "compression method" - it's also `0` for XMP
    literal(0_u8)
        .context(StrContext::Expected(StrContextValue::Description(
            "no specified compression method (0x0)",
        )))
        .void()
        .parse_next(input)?;

    // there's another two NUL bytes after those
    literal(0_u8).void().parse_next(input)?;
    literal(0_u8).void().parse_next(input)?;

    // the rest of the input is XMP
    let the_rest: &[u8] = rest.parse_next(input)?;

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

/// An error that occurs when constructing a [`Png`] for its metadata.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum PngConstructionError {
    /// The file ran out of bytes before we could check for a signature.
    ///
    /// It might be empty.
    NoSignature,

    /// No PNG signature was detected.
    NotAPng { found: [u8; 8] },
}

impl core::fmt::Display for PngConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const NOT_A_PNG_MSG: &str = "The given file's signature indicated it was not a PNG";

        match self {
            PngConstructionError::NoSignature => {
                f.write_str("File didn't have enough bytes for a signature.")
            }

            PngConstructionError::NotAPng { found } => match core::str::from_utf8(found) {
                Ok(utf8_found) => write!(
                    f,
                    "{NOT_A_PNG_MSG}. Signature was: `{found:?}`. (UTF-8: `{utf8_found}`)"
                ),
                Err(_) => write!(
                    f,
                    "{NOT_A_PNG_MSG}. Signature was: `{found:?}`. (Not valid UTF-8.)`"
                ),
            },
        }
    }
}

impl core::error::Error for PngConstructionError {}

#[cfg(test)]
mod tests {

    use raves_metadata_types::{
        exif::{
            Field, FieldData, FieldTag,
            primitives::{Primitive, Rational},
            tags::{Ifd0Tag, KnownTag},
        },
        xmp::{XmpElement, XmpValue},
    };

    use crate::{MetadataProvider as _, providers::png::Png, util::logger};

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
        let png: Png = Png::new(&technically_a_png).expect("is a png");

        let xmp = png
            .xmp()
            .expect("this PNG has XMP")
            .expect("get XMP from PNG");
        let locked_xmp = xmp.read();

        let parsed_xmp = (*locked_xmp).parse().expect("parse XMP data");

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

    /// Tests parsing out Exif from a 64x64 PNG file taken on my macbook.
    #[test]
    fn blank_sample_with_exif() {
        logger();
        const BLOB: &[u8] = include_bytes!("../../assets/providers/png/exif.png");

        let png: Png = Png::new(&BLOB).expect("parse PNG");

        let exif = png
            .exif()
            .expect("PNG contains Exif")
            .expect("Exif is well-formed");
        let exif_locked = exif.read();

        let a = exif_locked.ifds.first().unwrap();

        let expected_field_tag = FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::XResolution));
        assert_eq!(
            *a.fields
                .iter()
                .flatten()
                .find(|f| f.tag == expected_field_tag)
                .expect("find xres field"),
            Field {
                tag: expected_field_tag,
                data: FieldData::Primitive(Primitive::Rational(Rational {
                    numerator: 144,
                    denominator: 1
                }))
            }
        )
    }
}
