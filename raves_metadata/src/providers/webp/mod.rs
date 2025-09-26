use std::sync::Arc;

use parking_lot::RwLock;
use winnow::{Parser as _, binary::u8, error::EmptyError, token::take};

use crate::{MaybeParsedExif, MaybeParsedXmp, MetadataProvider, MetadataProviderRaw};

use self::{chunk::RiffChunk, error::WebpConstructionError, header::WebpFileHeader};

mod chunk;
mod error;
mod extended;
mod header;

#[derive(Clone, Debug)]
pub struct Webp {
    _header: WebpFileHeader,
    // relevant_chunks: Vec<(RiffChunk, &'file [u8])>,
    exif: Arc<RwLock<Option<MaybeParsedExif>>>,
    xmp: Arc<RwLock<Option<MaybeParsedXmp>>>,
}

impl MetadataProviderRaw for Webp {
    fn exif_raw(&self) -> std::sync::Arc<parking_lot::RwLock<Option<MaybeParsedExif>>> {
        Arc::clone(&self.exif)
    }

    fn xmp_raw(&self) -> std::sync::Arc<parking_lot::RwLock<Option<MaybeParsedXmp>>> {
        Arc::clone(&self.xmp)
    }
}

impl MetadataProvider for Webp {
    type ConstructionError = WebpConstructionError;

    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        // this does a little parsing, then disposes of the file...
        let mut input = input.as_ref();

        // first, look for the header.
        let header =
            header::webp_file_header(&mut input).map_err(|_| WebpConstructionError::NoHeader)?;

        // all WebPs should have at least one chunk
        let first_chunk = chunk::chunk(&mut input).map_err(|_| WebpConstructionError::NoChunks)?;

        // create an empty type for the file based on those two
        let mut s = Self {
            _header: header,
            exif: Arc::new(const { RwLock::new(None) }),
            xmp: Arc::new(const { RwLock::new(None) }),
        };

        let mut relevant_chunks = const { Vec::new() };

        // if it's an "extended" WebP, then it'll use the extended file format.
        //
        // that means it has a file feature info chunk, `VP8X`! if the file
        // doesn't have that chunk, then it has no metadata, and isn't useful
        // to us whatsoever.
        if &first_chunk.fourcc != b"VP8X" {
            log::debug!(
                "Not in 'extended' format: no metadata is present. No relevant chunks exist."
            );
            return Ok(s);
        }

        // then, get info about the file.
        //
        // this is arranged in a manner explained in the WebP docs. see:
        // https://developers.google.com/speed/webp/docs/riff_container
        let file_info_flags: u8 = u8
            .parse_next(&mut input)
            .map_err(|_: EmptyError| WebpConstructionError::MalformedExtendedHeader)?;

        // check the `E` (Exif) and `X` (XMP) presence bits
        let (has_exif, has_xmp) = (
            file_info_flags & 0b_0000_1000 != 0,
            file_info_flags & 0b_0001_0000 != 0,
        );

        // map the bools into a list of chunks we care about
        let required_chunks: &[[u8; 4]] = match (has_exif, has_xmp) {
            (false, false) => {
                log::debug!("The provided WebP file has no metadata.");
                return Ok(s);
            }
            (false, true) => &[*b"XMP "],
            (true, false) => &[*b"EXIF"],
            (true, true) => &[*b"EXIF", *b"XMP "],
        };

        // consume the remaining 3 bytes of header + 6 bytes of img size
        take(9_usize)
            .parse_next(&mut input)
            .map_err(|_: EmptyError| {
                log::error!(
                    "Couldn't consume remaining 'extended' bytes! \
                    This is a bug! Please report it."
                );
                WebpConstructionError::MalformedExtendedHeader
            })?;

        // account for any padding in the first chunk
        if first_chunk.len & 1 != 0 {
            _ = take::<_, _, EmptyError>(1_usize)
                .void()
                .parse_next(&mut input);
        }

        // loop the rest of the file, collecting only chunks we care about.
        while !input.is_empty() {
            log::info!("loopin");

            // grab the chunk header
            let chunk: RiffChunk = match chunk::chunk(&mut input) {
                Ok(c) => c,
                Err(e) => {
                    log::error!(
                        "Invalid RIFF chunk in WebP file! Returning \
                        results before erroneous chunk. err: {e}"
                    );
                    break;
                }
            };

            // something something borrow checker something
            let chunk_len: u32 = chunk.len;

            // if it's something we care about, add it and its data to the
            // relevent chunks list.
            //
            // otherwise, take its data and move on!
            if required_chunks.contains(&chunk.fourcc) {
                // grab the chunk data
                let Ok::<_, EmptyError>(chunk_data) = take(chunk.len).parse_next(&mut input) else {
                    log::warn!(
                        "Failed to take chunk's length of data. expected len of `{}`, but was only `{}`.",
                        chunk.len,
                        input.len()
                    );
                    continue;
                };

                // add it to the vec
                relevant_chunks.push((chunk, chunk_data));
            } else {
                _ = take::<_, _, EmptyError>(chunk.len)
                    .void()
                    .parse_next(&mut input);
            }

            // if the chunk has an odd length, we'll use its padding byte
            if !chunk_len.is_multiple_of(2) {
                _ = take::<_, _, EmptyError>(1_usize)
                    .void()
                    .parse_next(&mut input);
            }
        }

        const EXIF_CHUNK_HEADER: [u8; 4] = *b"EXIF";
        const XMP_CHUNK_HEADER: [u8; 4] = *b"XMP ";

        s.exif = Arc::new(RwLock::new(
            find_chunk(EXIF_CHUNK_HEADER, &relevant_chunks)
                .map(|r| MaybeParsedExif::Raw(Vec::from(r))),
        ));
        s.xmp = Arc::new(RwLock::new(
            find_chunk(XMP_CHUNK_HEADER, &relevant_chunks)
                .map(|r| MaybeParsedXmp::Raw(Vec::from(r))),
        ));

        Ok(s)
    }
}

/// Attempts to find the needle in the list of chunks.
///
/// - `needle` is the wanted chunk's header.
/// - `chunks` comes from the `Webp`'s `relevant_chunks` field.
fn find_chunk<'vec_ref, 'file: 'vec_ref>(
    needle: [u8; 4],
    chunks: &'vec_ref [(RiffChunk, &'file [u8])],
) -> Option<&'file [u8]> {
    for (RiffChunk { fourcc, .. }, blob) in chunks {
        if *fourcc == needle {
            return Some(blob);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::{
        exif::{
            Endianness, Field, FieldData, FieldTag,
            primitives::{Primitive, PrimitiveTy, Rational},
            tags::{Ifd0Tag, KnownTag},
        },
        xmp::{XmpElement, XmpPrimitive, XmpValue},
    };

    use crate::{
        MetadataProvider,
        exif::{Exif, Ifd},
        providers::webp::{chunk::RiffChunk, error::WebpConstructionError, find_chunk},
        util::logger,
    };

    use super::Webp;

    /// There are no "empty" WebP files - the standard requires at least one
    /// chunk on all kinds.
    ///
    /// The
    #[test]
    fn empty_webp_should_fail() {
        logger();

        let minimal_webp: &[u8] = &make_webp_sample(Vec::new());

        // assertion: empty webp should parse alright
        assert!(
            matches!(
                Webp::new(&minimal_webp),
                Err(WebpConstructionError::NoChunks)
            ),
            "shouldn't parse webp files w/ 0 chunks"
        );
    }

    /// The parser shouldn't reject "simple" WebP files.
    ///
    /// While they don't have metadata, parsing them should result in no work
    /// done.
    #[test]
    fn should_construct_simple_webp() {
        logger();

        let simple_webp: &[u8] = &make_webp_sample(vec![
            // note: the `VP8 ` chunk stores image data;
            //
            // it's the only chunk in a "simple" WebP.
            (b"VP8 ", [0_u8].as_slice()),
        ]);

        assert!(Webp::new(&simple_webp).is_ok());
    }

    /// Extended WebP files should construct fine.
    #[test]
    fn extended_webp_should_construct() {
        logger();

        let vp8x_chunk_data = vp8x(false, false);
        let bytes = &make_webp_sample(vec![
            (b"VP8X", vp8x_chunk_data.as_slice()),
            (b"FAKE", [33_u8; 29].as_slice()),
            (b"TEST", [1_u8; 2].as_slice()),
            (b"ONLY", [0_u8; 4].as_slice()),
        ]);

        assert!(Webp::new(bytes).is_ok());
    }

    /// Odd chunks shouldn't result in any weird corruption or nonsense.
    #[test]
    fn odd_num_of_chunk_bytes_should_construct() {
        logger();

        // simple
        {
            let bytes = &make_webp_sample(vec![(b"FAKE", [0_u8; 11].as_slice())]);
            assert!(Webp::new(bytes).is_ok());
        }

        // extended
        {
            let vp8x_chunk_data = vp8x(false, false);
            let bytes = &make_webp_sample(vec![
                (b"VP8X", vp8x_chunk_data.as_slice()),
                (b"FAKE", [33_u8; 29].as_slice()),
            ]);

            assert!(Webp::new(bytes).is_ok());
        }
    }

    /// Attempting to grab IPTC for a file should return `None`.
    ///
    /// It shouldn't error or anything, though!
    #[test]
    fn ensure_iptc_is_unsupported() {
        logger();

        let simple_webp: &[u8] = &make_webp_sample(vec![(b"VP8 ", [0_u8; 100].as_slice())]);
        let webp: Webp = Webp::new(&simple_webp).unwrap();

        assert!(
            webp.iptc().is_none(),
            "iptc is unsupported and should return None"
        );
    }

    /// XMP parsing should work fine.
    #[test]
    fn check_xmp() {
        logger();

        const XMP_DATA: &str = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description rdf:about="" xmlns:my_ns="https://barretts.club">
                <my_ns:MyStruct>
                    <rdf:Description />
                </my_ns:MyStruct>
            </rdf:Description>
        </rdf:RDF>"#;

        // setup the sample
        let vp8x_chunk_data = vp8x(false, true);
        let bytes = &make_webp_sample(vec![
            (b"VP8X", &vp8x_chunk_data),
            (b"XMP ", XMP_DATA.as_bytes()),
            (b"VP8 ", &[0x00]),
        ]);

        // construct webp representation
        let webp: Webp = Webp::new(bytes).unwrap();

        // construct the xmp
        let xmp = webp
            .xmp()
            .expect("XMP is supported _and_ provided in the file")
            .expect("the XMP should construct correctly");
        let locked_xmp = xmp.read();

        // parse xmp
        let xmp_doc = locked_xmp.parse().expect("xmp is valid");

        assert_eq!(
            xmp_doc.values_ref().first().unwrap(),
            &XmpElement {
                namespace: "https://barretts.club".into(),
                prefix: "my_ns".into(),
                name: "MyStruct".into(),
                value: XmpValue::Struct(Vec::new()),
            }
        );
    }

    #[test]
    fn real_sample_image_should_construct() {
        logger();

        let bytes = include_bytes!("../../../assets/1.webp");

        // construct webp representation
        let webp: Webp = Webp::new(bytes).unwrap();

        // there isn't any metadata
        assert!(webp.exif.read().is_none());
        assert!(webp.xmp.read().is_none());
    }

    #[test]
    fn real_sample_image_should_parse() {
        let bytes = include_bytes!("../../../assets/photopea.webp");

        // construct webp representation
        let webp: Webp = Webp::new(bytes).unwrap();

        // construct the xmp
        let xmp = webp
            .xmp()
            .expect("XMP is supported _and_ provided in the file")
            .expect("the XMP should construct correctly");
        let locked_xmp = xmp.read();

        // parse xmp
        let xmp_doc = locked_xmp.parse().expect("xmp is valid");

        // note: this is the same check as one in the `xmp` module
        assert_eq!(
            xmp_doc.values_ref().to_vec(),
            vec![XmpElement {
                namespace: "http://purl.org/dc/elements/1.1/".into(),
                prefix: "dc".into(),
                name: "subject".into(),
                value: XmpValue::UnorderedArray(vec![
                    XmpElement {
                        name: "li".into(),
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("farts".into()))
                    },
                    XmpElement {
                        name: "li".into(),
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("not farts".into()))
                    },
                    XmpElement {
                        name: "li".into(),
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("etc.".into()))
                    },
                ])
            }]
        );
    }

    #[test]
    fn should_find_exif_in_real_sample_image() {
        logger();

        let file = include_bytes!("../../../assets/providers/webp/RIFF.webp");
        let webp: Webp = Webp::new(file).unwrap();

        let exif = webp.exif().expect("file has exif").expect("exif is valid");
        let locked_exif = exif.read().clone();

        assert_eq!(
            locked_exif,
            Exif {
                endianness: Endianness::Big,
                ifds: vec![Ifd {
                    fields: vec![
                        Ok(Field {
                            tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::XResolution)),
                            data: FieldData::Primitive(Primitive::Rational(Rational {
                                numerator: 72,
                                denominator: 1
                            })),
                        }),
                        Ok(Field {
                            tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::YResolution)),
                            data: FieldData::Primitive(Primitive::Rational(Rational {
                                numerator: 72,
                                denominator: 1
                            })),
                        }),
                        Ok(Field {
                            tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::ResolutionUnit)),
                            data: FieldData::Primitive(Primitive::Short(2)),
                        }),
                        Ok(Field {
                            tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::Artist)),
                            data: FieldData::List {
                                list: b"me\0".map(Primitive::Ascii).into(),
                                ty: PrimitiveTy::Ascii,
                            },
                        }),
                        Ok(Field {
                            tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::YCbCrPositioning)),
                            data: FieldData::Primitive(Primitive::Short(1)),
                        }),
                    ],
                    sub_ifds: Vec::new(),
                }]
            }
        );
    }

    /// The `find_chunk` function should be able to find all the needles.
    #[test]
    fn find_chunk_finds_needles() {
        logger();

        let fourcc_list = [b"1234", b"AAAA", b"\0\0\0\0", b"Eggs"];

        let chunks = fourcc_list.map(|needle| {
            (
                RiffChunk {
                    fourcc: *needle,
                    len: 0_u32,
                },
                needle.as_slice(),
            )
        });

        for needle in fourcc_list {
            let maybe_blob = find_chunk(*needle, chunks.as_slice());
            assert_eq!(maybe_blob, Some(needle.as_slice()));
        }
    }

    /// helper: create the `VP8X` chunk (required for "extended" WebP)
    fn vp8x(has_exif: bool, has_xmp: bool) -> Vec<u8> {
        let exif_bit: u8 = match has_exif {
            true => 0b0000_1000,
            false => 0b0000_0000,
        };

        let xmp_bit: u8 = match has_xmp {
            true => 0b0001_0000,
            false => 0b0000_0000,
        };

        #[rustfmt::skip]
        let bytes = [
            exif_bit | xmp_bit,
            0_u8, 0_u8, 0_u8,
            0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8,
        ].to_vec();

        bytes
    }

    /// helper: build a file to make these tests readable
    ///
    /// (trust me; they weren't before)
    fn make_webp_sample(chunks: Vec<(&[u8; 4], &[u8])>) -> Vec<u8> {
        let mut bytes = Vec::new();

        // add the file header
        bytes.extend_from_slice(b"RIFF");
        bytes.extend([0; 4]); // we'll fill this in just a sec
        bytes.extend_from_slice(b"WEBP");

        // make each chunk
        for (chunk_fourcc, chunk_data) in chunks.iter() {
            // add fourcc directly
            bytes.extend_from_slice(chunk_fourcc.as_slice());

            // handle chunk data
            bytes.extend((chunk_data.len() as u32).to_le_bytes()); // len
            bytes.extend_from_slice(chunk_data); // fr data

            // add an extra padding byte if the size is odd
            if chunk_data.len() % 2 != 0 {
                bytes.push(0_u8);
            }
        }

        // with all chunks done, we set the file size
        let total_size_of_chunks: u32 = (bytes.len() as u32) - 8_u32;
        bytes[4..8].copy_from_slice(&total_size_of_chunks.to_le_bytes());

        bytes
    }
}
