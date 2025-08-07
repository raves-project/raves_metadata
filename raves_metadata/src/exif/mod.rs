//! Exif is a media metadata format primarily used by cameras.
//!
//! Unlike XMP, it's a structured binary format, so, while it's not as
//! "extensible," Exif does allow for proprietary extensions that are just
//! blobs of bytes.
//!
//! However, it's somewhat self-describing! Each field on an IFD
//! (Image File Directory) contains a tag ID, primitive data type, and count
//! saying how many primitives are stored. That means we can easily provide
//! proprietary extensions - all without knowing how they're structured.
//!
//! Note that proprietary extensions usually use the opaque (`Undefined`) data
//! type, so you usually won't get much useful info from them. Nonetheless,
//! they're provided for folks who need them.

pub use raves_metadata_types::exif::{Endianness, Field, FieldData, primitives::*};

use winnow::{
    Parser as _, Stateful,
    binary::{Endianness as WinnowEndianness, u16, u32},
    error::EmptyError,
    token::take,
};

use self::{
    error::{ExifFatalError, ExifFatalResult},
    ifd::Ifd,
    ifd::parse_ifd,
};
use raves_metadata_types::exif::ifd::IfdGroup;

pub mod error;
mod ifd;
mod value;

/// Extracted information from an Exif metadata block.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Exif {
    /// The endianness of the Exif block.
    pub endianness: Endianness,

    /// The IFDs found in the Exif metadata.
    pub ifds: Vec<Ifd>,
}

impl Exif {
    /// Parses the given Exif blob into our `Exif` structure.
    pub fn new(input: &mut &[u8]) -> ExifFatalResult<Self> {
        #[expect(
            suspicious_double_ref_op,
            reason = "we want to save the original slice (\"blob\") for absolute offsets"
        )]
        let blob: &[u8] = input.clone(); // this is the original input

        // parse the endianness
        let endianness: Endianness = parse_blob_endianness.parse_next(input)?;

        let winnow_endianness: WinnowEndianness = match endianness {
            Endianness::Little => WinnowEndianness::Little,
            Endianness::Big => WinnowEndianness::Big,
        };

        // alright. from here on out, we've got to account for the endianness
        // of everything.
        //
        // to do so, our input is wrapped in `Stateful`
        let stateful_input = &mut Stream {
            input,
            state: State {
                endianness: &winnow_endianness,
                blob,
                current_ifd: IfdGroup::_0, // we always start with IFD 0
            },
        };

        // ensure we've got a TIFF marker (magic number)
        parse_tiff_magic_number.parse_next(stateful_input)?;

        // grab the offset from the TIFF marker where we'll start
        let offset: u32 = parse_tiff_header_offset(stateful_input)?;

        // perform the offset
        take(offset)
            .parse_next(&mut stateful_input.input)
            .inspect_err(|_| log::error!("Failed to skip to IFDs"))
            .map_err(|_: EmptyError| ExifFatalError::NotEnoughDataForHeaderOffset)?;

        // if there are no IFDs, do an early return
        let mut ifds: Vec<Ifd> = Vec::new();
        if stateful_input.is_empty() {
            log::trace!("There's no more input. Assuming there are zero IFDs.");
            return Ok(Self { endianness, ifds });
        }

        // parse out the first IFD (it tells us where the rest are)
        let (first_ifd, mut maybe_next_ifd_ptr): (Ifd, Option<u32>) =
            parse_ifd.parse_next(stateful_input).inspect_err(|e| {
                log::error!("Failed to parse Exif! The first IFD failed to parse! err: {e}")
            })?;
        log::trace!("Completed first IFD! ptr: {maybe_next_ifd_ptr:?}");
        ifds.push(first_ifd);

        // now, parse out each IFD
        while let Some(next_ifd_ptr) = maybe_next_ifd_ptr {
            // swap out the saved input for the absolute offset provided by the
            // previous IFD
            log::trace!("At next IFD! index: `{next_ifd_ptr}`");
            stateful_input.input = &blob[(next_ifd_ptr as usize)..];

            // keep parsing
            let (ifd, ptr) = parse_ifd.parse_next(stateful_input)?;
            ifds.push(ifd);
            maybe_next_ifd_ptr = ptr;
        }

        Ok(Self { endianness, ifds })
    }
}

/// Finds the endianness of the Exif blob.
fn parse_blob_endianness(input: &mut &[u8]) -> ExifFatalResult<Endianness> {
    let input_len = input.len();

    // ensure we've got two good bytes
    log::trace!("Looking for the BOM bytes...");
    let two_ascii_bytes: [u8; 2] = take(2_usize)
        .parse_next(input)
        .map_err(|_: EmptyError| {
            log::error!("Couldn't find endianness marker!");
            ExifFatalError::NoByteOrderMarker {
                len: input_len as u8,
            }
        })?
        .try_into()
        .unwrap_or_else(|e| unreachable!("winnow verified the size. but err: {e}"));
    log::trace!("Found two BOM bytes!");

    // parse the bytes we found
    log::trace!("Grabbing BOM...");
    match two_ascii_bytes {
        [b'I', b'I'] => Ok(Endianness::Little).inspect(|f| log::trace!("It's LE: {f:?}")),
        [b'M', b'M'] => Ok(Endianness::Big).inspect(|f| log::trace!("It's BE: {f:?}")),

        // found a weird bom!
        found => {
            let e = ExifFatalError::WeirdByteOrderMarker { found };
            log::error!("Couldn't parse out Exif! err: {e}");
            Err(e)
        }
    }
}

/*
*
*
*
  NOTE:

  all parsers from here on out generally require knowing the endianness. so,
  it's stored inside a custom state struct + a stream wrapper.

  this allows us to easily pass our state between pieces of the parser, all
  without globals or other nasty stuff
*
*
*
*
*/

#[derive(Debug)]
struct State<'a> {
    current_ifd: IfdGroup,
    endianness: &'a WinnowEndianness,
    blob: &'a [u8],
}

/// A stream of the blob wrapped with our endianness.
type Stream<'s> = Stateful<&'s [u8], State<'s>>;

/// Ensures we're working with the correct kind of file.
fn parse_tiff_magic_number(input: &mut Stream) -> ExifFatalResult<()> {
    // we account for endianness from here on out
    let endianness = input.state.endianness;

    // grab the magic number bytes as a u16
    log::trace!("Getting magic number...");
    let magic_number: u16 = u16(*endianness)
        .parse_next(input)
        .map_err(|_: EmptyError| {
            log::error!("Couldn't find TIFF magic number!");
            ExifFatalError::NoTiffMagicNumber
        })?;

    // check the magic number
    log::trace!("Checking magic number...");
    if magic_number != 42 {
        log::error!("Magic number wasn't for TIFF. got: `{magic_number}`");
        return Err(ExifFatalError::MagicNumberWasntTiff {
            found: magic_number,
        });
    }

    log::trace!("Magic number was good!");
    Ok(())
}

/// Grabs the TIFF header offset.
///
/// This offset tells us how much of the file isn't useful to us - that is, how
/// much the parser will `.take(n).void()`.
fn parse_tiff_header_offset(input: &mut Stream) -> ExifFatalResult<u32> {
    let endianness = input.state.endianness;

    u32(*endianness)
        .parse_next(&mut input.input)
        .map_err(|_: EmptyError| {
            log::error!("Didn't find a TIFF header offset!");
            ExifFatalError::NoTiffHeaderOffset
        })
        .inspect(|offset| log::trace!("found offset: `{offset}`"))?
        .checked_sub(8_u32)
        .ok_or_else(|| {
            log::error!("Exif blob placed offset out of bounds! Can't continue parsing.");
            ExifFatalError::HeaderOffsetBeforeHeader
        })
}

/// A pointer in the blob specifying the next IFD, if any.
type NextIfdPointer = Option<u32>;

#[cfg(test)]
mod tests {
    use raves_metadata_types::exif::{
        Endianness, Field, FieldData, FieldTag,
        ifd::IfdGroup,
        primitives::{Primitive, PrimitiveCount, PrimitiveTy, Rational},
        tags::{Ifd0Tag, KnownTag},
    };
    use winnow::binary::Endianness as WinnowEndianness;

    use crate::exif::{
        Exif, Ifd, error::ExifFatalError, parse_blob_endianness, parse_tiff_header_offset,
        parse_tiff_magic_number,
    };

    /// Checks that we're able to parse endianness properly.
    #[test]
    fn endianness() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        assert_eq!(
            parse_blob_endianness(&mut b"II".as_slice()),
            Ok(Endianness::Little)
        );
        assert_eq!(
            parse_blob_endianness(&mut b"MM".as_slice()),
            Ok(Endianness::Big)
        );
        assert!(
            parse_blob_endianness(&mut b"other".as_slice()).is_err(),
            "other strings aren't indicative of endianness"
        );
    }

    /// Checks if we can parse the TIFF header correctly.
    #[test]
    fn tiff_header() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let backing_bytes = {
            let mut v = Vec::new();
            v.append(&mut b"II".to_vec());
            v.push(0x2a);
            v.push(0x00);
            v
        };
        let bytes = &mut backing_bytes.as_slice();

        // first, parse out the endianness
        let endianness = parse_blob_endianness(bytes);
        assert_eq!(endianness, Ok(Endianness::Little));
        log::info!("backing bytes now: {backing_bytes:#?}");

        // then, check for the header
        assert_eq!(
            parse_tiff_magic_number(&mut super::Stream {
                state: super::State {
                    current_ifd: IfdGroup::_0,
                    endianness: &WinnowEndianness::Little,
                    blob: backing_bytes.as_slice()
                },
                input: bytes
            }),
            Ok(()),
            "should find header"
        );
    }

    /// Checks if we can parse the TIFF header offset correctly.
    #[test]
    fn tiff_header_offset() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let backing_bytes = {
            let mut v = Vec::new();
            v.extend_from_slice(b"II".as_slice());
            v.push(0x2a);
            v.push(0x00);
            v.extend_from_slice(8_u32.to_le_bytes().as_slice());
            v
        };
        let bytes = &mut backing_bytes.as_slice();

        // parse out the endianness
        let endianness = parse_blob_endianness(bytes);
        assert_eq!(endianness, Ok(Endianness::Little));
        log::info!("backing bytes now: {backing_bytes:#?}");

        let stream = &mut super::Stream {
            state: super::State {
                current_ifd: IfdGroup::_0,
                endianness: &WinnowEndianness::Little,
                blob: backing_bytes.as_slice(),
            },
            input: bytes,
        };

        // check for the header
        assert_eq!(parse_tiff_magic_number(stream), Ok(()));

        // ensure the offset is zero
        assert_eq!(parse_tiff_header_offset(stream), Ok(0_u32));

        // also, ensure that headers with weird values (i.e. < 8) fail to parse
        assert_eq!(
            parse_tiff_header_offset(&mut super::Stream {
                state: super::State {
                    current_ifd: IfdGroup::_0,
                    endianness: &WinnowEndianness::Little,
                    blob: backing_bytes.as_slice()
                },
                input: 7_u32.to_le_bytes().as_slice(),
            }),
            Err(ExifFatalError::HeaderOffsetBeforeHeader),
        );
        assert_eq!(
            parse_tiff_header_offset(&mut super::Stream {
                state: super::State {
                    current_ifd: IfdGroup::_0,
                    endianness: &WinnowEndianness::Little,
                    blob: backing_bytes.as_slice()
                },
                input: 0_u32.to_le_bytes().as_slice(),
            }),
            Err(ExifFatalError::HeaderOffsetBeforeHeader),
        );
    }

    #[test]
    fn parses_minimal_exif() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let mut backing_bytes = Vec::new();
        backing_bytes.extend_from_slice(b"II");
        backing_bytes.extend_from_slice(42_u16.to_le_bytes().as_slice());
        backing_bytes.extend_from_slice(9_u32.to_le_bytes().as_slice()); // 9 bytes to skip - 8 are the header
        backing_bytes.push(u8::MAX); // push a junk byte! should be ignored.

        // there's only one field in this IFD
        backing_bytes.extend_from_slice(1_u16.to_le_bytes().as_slice());

        // make an IFD entry
        backing_bytes.extend_from_slice(
            KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth)
                .tag_id()
                .to_le_bytes()
                .as_slice(),
        );
        backing_bytes.extend_from_slice(3_u16.to_le_bytes().as_slice());
        backing_bytes.extend_from_slice(1_u32.to_le_bytes().as_slice());
        backing_bytes.extend_from_slice(1920_u16.to_le_bytes().as_slice());
        backing_bytes.extend_from_slice(0_u16.to_le_bytes().as_slice());

        // no other IFDs are after this one
        backing_bytes.extend_from_slice(0_u32.to_le_bytes().as_slice());

        let mut bytes = backing_bytes.as_slice();

        let exif = Exif::new(&mut bytes).unwrap();
        assert_eq!(exif.ifds.len(), 1, "only one IFD");

        // grab the only IFD and its only field
        let ifd0 = &exif.ifds[0];
        let Ok(ref field) = ifd0.fields[0] else {
            panic!("should have a field");
        };

        // check that it's right
        assert_eq!(
            field.tag,
            FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth)),
            "field tag"
        );
        assert_eq!(
            field.data,
            FieldData::Primitive(Primitive::Short(1920)),
            "field val"
        );
    }

    /// IDFs without fields are disallowed - they should fail parsing.
    #[test]
    fn ifd_with_no_fields_should_fail() {
        let mut backing_bytes = Vec::new();

        // header
        backing_bytes.extend_from_slice(b"MM");
        backing_bytes.extend_from_slice(42_u16.to_be_bytes().as_slice());
        backing_bytes.extend_from_slice(8_u32.to_be_bytes().as_slice());

        // invalid 'blank' IFD
        backing_bytes.extend_from_slice(0_u16.to_be_bytes().as_slice());
        backing_bytes.extend_from_slice(0_u32.to_le_bytes().as_slice());

        let parsed = Exif::new(&mut backing_bytes.as_slice());
        assert_eq!(parsed, Err(ExifFatalError::IfdHadZeroFields))
    }

    /// We should succeed at parsing when no IFDs are present.
    #[test]
    fn no_ifds() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let mut backing_bytes = Vec::new();

        // header
        backing_bytes.extend_from_slice(b"II");
        backing_bytes.extend_from_slice(42_u16.to_le_bytes().as_slice());
        backing_bytes.extend_from_slice(8_u32.to_le_bytes().as_slice());

        let parsed = Exif::new(&mut backing_bytes.as_slice());
        assert_eq!(
            parsed,
            Ok(Exif {
                endianness: Endianness::Little,
                ifds: vec![]
            }),
            "we shouldn't find any IFDs"
        );
    }

    /// Ensures we can parse a blob with multiple IFDs.
    #[test]
    fn multiple_ifds() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let mut backing_bytes = Vec::new();
        backing_bytes.extend_from_slice(b"MM");
        backing_bytes.extend_from_slice(42_u16.to_be_bytes().as_slice());
        backing_bytes.extend_from_slice(8_u32.to_be_bytes().as_slice());

        // the first IFD will have width + height
        backing_bytes.extend_from_slice(2_u16.to_be_bytes().as_slice()); // two fields
        backing_bytes.extend_from_slice(
            KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth)
                .tag_id()
                .to_be_bytes()
                .as_slice(),
        ); // f1 id
        backing_bytes.extend_from_slice(3_u16.to_be_bytes().as_slice()); // f1 ty
        backing_bytes.extend_from_slice(1_u32.to_be_bytes().as_slice()); // f1 ct
        backing_bytes.extend_from_slice(1920_u16.to_be_bytes().as_slice()); // f1 val
        backing_bytes.extend_from_slice(0_u16.to_be_bytes().as_slice());
        backing_bytes.extend_from_slice(
            KnownTag::Ifd0Tag(Ifd0Tag::ImageLength)
                .tag_id()
                .to_be_bytes()
                .as_slice(),
        ); // f2 id
        backing_bytes.extend_from_slice(3_u16.to_be_bytes().as_slice()); // f2 ty
        backing_bytes.extend_from_slice(1_u32.to_be_bytes().as_slice()); // f2 ct
        backing_bytes.extend_from_slice(1080_u16.to_be_bytes().as_slice()); // f2 val
        backing_bytes.extend_from_slice(0_u16.to_be_bytes().as_slice());

        // create an offset + some padding for the next IFD
        let next_ifd_offset = backing_bytes.len() as u32 + 4 + 88;
        backing_bytes.extend_from_slice(next_ifd_offset.to_be_bytes().as_slice());
        backing_bytes.extend_from_slice([0_u8; 88].as_slice());

        // IFD #2 gets one veeeery long field
        backing_bytes.extend_from_slice(1_u16.to_be_bytes().as_slice()); // 1 field
        backing_bytes.extend_from_slice(
            KnownTag::Ifd0Tag(Ifd0Tag::TransferFunction)
                .tag_id()
                .to_be_bytes()
                .as_slice(),
        ); // f1 tag
        backing_bytes.extend_from_slice(
            (KnownTag::Ifd0Tag(Ifd0Tag::TransferFunction).types()[0] as u16)
                .to_be_bytes()
                .as_slice(),
        ); // f1 ty
        backing_bytes.extend_from_slice(
            {
                let PrimitiveCount::Known(c) = KnownTag::Ifd0Tag(Ifd0Tag::TransferFunction).count()
                else {
                    panic!("wrong count");
                };
                c
            }
            .to_be_bytes()
            .as_slice(),
        ); // f1 count
        // the f1 data will be after all the IFDs, at blob[2000..]
        let ifd2_f1_data = [99_u8; (3 * 256_usize) * 2].as_slice();
        backing_bytes.extend_from_slice(2000_u32.to_be_bytes().as_slice());

        // create offset + padding for last IFD
        // let next_ifd_offset = backing_bytes.len() as u32 + 4 + 823_u32;
        // let next_ifd_offset: u32 = 0_u32;
        // backing_bytes.extend_from_slice(next_ifd_offset.to_be_bytes().as_slice());
        // backing_bytes.extend_from_slice([0_u8; 823].as_slice());
        let next_ifd_offset = backing_bytes.len() as u32 + 4 + 88;
        backing_bytes.extend_from_slice(next_ifd_offset.to_be_bytes().as_slice());
        backing_bytes.extend_from_slice([0_u8; 88].as_slice());

        // IFD #3
        backing_bytes.extend_from_slice(2_u16.to_be_bytes().as_slice()); // two fields
        backing_bytes.extend_from_slice(
            KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth)
                .tag_id()
                .to_be_bytes()
                .as_slice(),
        ); // f1 tag id
        backing_bytes.extend_from_slice(3_u16.to_be_bytes().as_slice()); // f1 ty
        backing_bytes.extend_from_slice(1_u32.to_be_bytes().as_slice()); // f1 count
        backing_bytes.extend_from_slice(1920_u16.to_be_bytes().as_slice()); // f1 data
        backing_bytes.extend_from_slice(0_u16.to_be_bytes().as_slice());
        backing_bytes.extend_from_slice(
            KnownTag::Ifd0Tag(Ifd0Tag::ImageLength)
                .tag_id()
                .to_be_bytes()
                .as_slice(),
        ); // f2 tag id
        backing_bytes.extend_from_slice(3_u16.to_be_bytes().as_slice()); // f2 ty
        backing_bytes.extend_from_slice(1_u32.to_be_bytes().as_slice()); // f2 count
        backing_bytes.extend_from_slice(1080_u16.to_be_bytes().as_slice()); // f2 data
        backing_bytes.extend_from_slice(0_u16.to_be_bytes().as_slice());

        // no more IFDs...
        backing_bytes.extend_from_slice(0_u32.to_be_bytes().as_slice()); // 'null' offset

        // place the giant [IFD 2, Field 1] data at index 2000.
        //
        // but first, add some padding
        backing_bytes.extend(
            (0..(2000 - backing_bytes.len()))
                .map(|_| 0_u8)
                .collect::<Vec<u8>>(),
        );
        backing_bytes.extend_from_slice(ifd2_f1_data);

        let parsed = Exif::new(&mut backing_bytes.as_slice()).expect("parsing should work");

        assert_eq!(
            parsed,
            Exif {
                endianness: Endianness::Big,
                ifds: vec![
                    Ifd {
                        fields: vec![
                            Ok(Field {
                                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth)),
                                data: FieldData::Primitive(Primitive::Short(1920)),
                            }),
                            Ok(Field {
                                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::ImageLength)),
                                data: FieldData::Primitive(Primitive::Short(1080)),
                            })
                        ],
                        sub_ifds: Vec::new(),
                    },
                    Ifd {
                        fields: vec![Ok(Field {
                            tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::TransferFunction)),
                            data: FieldData::List {
                                list: [Primitive::Short((99_u16 << 8) | 99_u16); (3 * 256_usize)]
                                    .into(),
                                ty: PrimitiveTy::Short
                            },
                        })],
                        sub_ifds: Vec::new(),
                    },
                    Ifd {
                        fields: vec![
                            Ok(Field {
                                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth)),
                                data: FieldData::Primitive(Primitive::Short(1920)),
                            }),
                            Ok(Field {
                                tag: FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::ImageLength)),
                                data: FieldData::Primitive(Primitive::Short(1080)),
                            })
                        ],
                        sub_ifds: Vec::new(),
                    },
                ]
            }
        )
    }
    /// helper: init logging
    fn logger() {
        _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();
    }
}
