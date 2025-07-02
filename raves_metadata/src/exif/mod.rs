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
use raves_metadata_types::exif::{FieldTag, parse_table::KnownField};

use winnow::{
    Parser as _, Stateful,
    binary::{Endianness as WinnowEndianness, i32, u8, u16, u32},
    error::EmptyError,
    token::take,
};

use crate::exif::error::{ExifFatalError, ExifFatalResult, ExifFieldError, ExifFieldResult};

pub mod error;

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
        let blob = input.clone(); // this is the original input

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
        let (ifd, mut maybe_next_ifd_ptr): (Ifd, Option<u32>) =
            parse_ifd.parse_next(stateful_input)?;
        log::trace!("Completed first IFD! ptr: {maybe_next_ifd_ptr:?}");
        ifds.push(ifd);

        // now, parse out each IFD
        while let Some(next_ifd_ptr) = maybe_next_ifd_ptr {
            // swap out the saved input for the absolute offset provided by the
            // previous IFD
            log::trace!("At next IFD! ptr: {next_ifd_ptr:X}");
            stateful_input.input = &blob[(next_ifd_ptr as usize)..];

            // keep parsing
            let (ifd, ptr) = parse_ifd.parse_next(stateful_input)?;
            ifds.push(ifd);
            maybe_next_ifd_ptr = ptr;
        }

        Ok(Self { endianness, ifds })
    }
}

/// An image file directory found within Exif metadata.
///
/// These contain a number of fields - at least one - and directions to the
/// next IFD.
#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, PartialOrd)]
pub struct Ifd {
    /// A list of fields on this IFD.
    pub fields: Vec<Result<Field, ExifFieldError>>,
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

/// Parses out an entire IFD.
fn parse_ifd(input: &mut Stream) -> Result<(Ifd, NextIfdPointer), ExifFatalError> {
    let endianness = *input.state.endianness;

    let entry_count: u16 = u16(endianness).parse_next(input).map_err(|_: EmptyError| {
        log::error!("Couldn't find count on IFD - ran out of data!");
        ExifFatalError::IfdNoEntryCount
    })?;

    if entry_count == 0 {
        log::error!("IFD reported itself as having zero fields! This is fatal to parsing.");
        return Err(ExifFatalError::IfdHadZeroFields);
    }

    log::trace!("Parsing `{entry_count}` fields...");
    let ifd = Ifd {
        fields: (0..entry_count).map(|_| parse_value(input)).collect(),
    };
    log::trace!("Completed field parsing!");

    let next_ifd_location = {
        let raw_location: u32 = u32(endianness).parse_next(input).map_err(|_: EmptyError| {
            log::error!("IFD didn't contain a pointer to the next IFD!");
            ExifFatalError::IfdNoPointer
        })?;

        if raw_location == 0_u32 {
            log::trace!("There won't be a next IFD.");
            None
        } else {
            log::trace!("Another IFD was detected! ptr: `{raw_location:X}`");
            Some(raw_location)
        }
    };

    Ok((ifd, next_ifd_location))
}

/// Parses out one value from an IFD.
fn parse_value(input: &mut Stream) -> ExifFieldResult {
    let endianness = input.state.endianness;

    // grab tag (2 bytes)
    let tag: FieldTag = {
        let raw_tag: u16 = u16(*endianness)
            .parse_next(&mut input.input)
            .map_err(|_: EmptyError| ExifFieldError::FieldNoTag)?;

        KnownField::try_from(raw_tag)
            .map(FieldTag::Known)
            .unwrap_or(FieldTag::Unknown(raw_tag))
    };

    // type (2 bytes)
    let ty: PrimitiveTy = {
        // grab the raw value
        let raw_ty: u16 = u16(*endianness)
            .parse_next(&mut input.input)
            .map_err(|_: EmptyError| ExifFieldError::FieldNoTy)?;

        // make it into a type repr enum
        PrimitiveTy::try_from(raw_ty).map_err(|_| {
            log::error!("Encountered unknown field type: `{raw_ty}`");
            ExifFieldError::FieldUnknownType { got: raw_ty }
        })?
    };

    // count (4 bytes)
    let count: u32 = u32(*endianness)
        .parse_next(&mut input.input)
        .map_err(|_: EmptyError| ExifFieldError::FieldNoCount)?;

    // grab the value or offset (4 bytes. we'll handle deciding in a sec)
    let value_or_offset: u32 = u32(*endianness)
        .parse_next(&mut input.input)
        .map_err(|_: EmptyError| ExifFieldError::FieldNoOffsetOrValue)?;

    log::trace!(
        "(field info... tag: {tag}, ty: {ty:?}, count: {count}, offset: {value_or_offset})"
    );

    // warn if the real type isn't an expected type
    if let FieldTag::Known(known_tag) = tag
        && !known_tag.types().contains(&ty)
    {
        log::warn!(
            "Field `{known_tag:?}` had a type mismatch! \
            Continuing parsing with wrong type anyway... \
            got: `{ty:?}`, \
            expected: {:?}",
            known_tag.types()
        );
    }

    // TODO: check for Count::SpecialHandling

    // check how large the stored data is
    let total_size: u32 = ty.size_bytes() as u32 * count;

    // figure out what `value_or_offset` really is
    let is_offset: bool = total_size > 4_u32;
    let value: [u8; 4] = match endianness {
        WinnowEndianness::Big => value_or_offset.to_be_bytes(),
        WinnowEndianness::Little => value_or_offset.to_le_bytes(),
        WinnowEndianness::Native => unreachable!("we never use this variant"),
    };

    // if the value is an offset, apply the offset and use the shifted blob as
    // the buffer. (offsets are relative to the beginning of the blob)
    //
    // if it's not, just use our value and leave :)
    let data: &[u8] = match is_offset {
        true => {
            log::trace!("Using reference to blob for value's absolute offset.");
            let blob_max_index: u32 = input.state.blob.len().saturating_sub(1) as u32;

            if value_or_offset > blob_max_index {
                log::error!(
                    "Field said its data is stored outside the blob! \
                    That's not possible. Can't continue parsing this field. \
                    offset: `{value_or_offset}`, blob's maximum index: `{blob_max_index}`"
                );
                return Err(ExifFieldError::OffsetTooFar {
                    offset: value_or_offset,
                });
            }

            // use the fr input as our value input
            input
                .state
                .blob
                .get(value_or_offset as usize..)
                .ok_or_else(|| {
                    log::error!("Attempted to offset too far!");
                    ExifFieldError::OffsetTooFar {
                        offset: value_or_offset,
                    }
                })?
        }

        false => {
            log::trace!("No value offset detected.");
            let mut sli = value.as_slice(); // it's just a value; send it over as a slice

            // account for big-endian values smaller than 4 bytes.
            //
            // in essence, we need to scoot the bits we care about over to the
            // other side. otherwise, we're reading them in the right order,
            // but with padding at the beginning :(
            if *endianness == WinnowEndianness::Big && total_size < 4 {
                sli = &sli[4 - total_size as usize..];
            }

            sli
        }
    };

    // construct the stateful stream containing the field's data
    let prim_stream = &mut PrimitiveStream {
        input: data,
        state: PrimitiveState {
            tag: &tag,
            endianness,
            count,
            ty: &ty,
        },
    };

    // parse the data for use in the field
    let field_data = match count {
        // if the count is zero, we won't perform any work at all
        0_u32 => {
            log::trace!("There are no stored primitives in this IFD. Returning early!");
            FieldData::None(ty)
        }

        // when we just have one, parse it alone and return immediately
        1_u32 => {
            log::trace!("Asked to only parse one primitive.");
            FieldData::Primitive(parse_primitive(prim_stream)?)
        }

        // other counts are higher; we'll make a list
        _ => {
            log::trace!("Asked to parse list of primitives. value ct: `{count}`");
            FieldData::List {
                list: parse_primitive_list(prim_stream)?,
                ty,
            }
        }
    };

    // return it wrapped in a field
    Ok(Field {
        tag,
        data: field_data,
    })
}

#[derive(Debug)]
struct PrimitiveState<'s> {
    tag: &'s FieldTag,
    endianness: &'s WinnowEndianness,
    count: u32,
    ty: &'s PrimitiveTy,
}
type PrimitiveStream<'s> = Stateful<&'s [u8], PrimitiveState<'s>>;

/// Parses a list of primitives.
fn parse_primitive_list(input: &mut PrimitiveStream) -> Result<Vec<Primitive>, ExifFieldError> {
    let mut v: Vec<Primitive> = Vec::with_capacity(input.state.count as usize);

    for i in 0..input.state.count {
        v.push(parse_primitive.parse_next(input).inspect_err(|e| {
            log::error!(
                "Failed to create primitive #{i} on {}. err: {e}",
                input.state.tag
            )
        })?);
    }

    Ok(v)
}

/// Parses a single primitive.
fn parse_primitive(input: &mut PrimitiveStream) -> Result<Primitive, ExifFieldError> {
    let endianness = input.state.endianness;
    let ty = *input.state.ty;

    match ty {
        PrimitiveTy::Byte => Ok(Primitive::Byte(
            u8.parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),

        PrimitiveTy::Ascii => Ok(Primitive::Ascii(
            u8.parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),

        PrimitiveTy::Short => Ok(Primitive::Short(
            u16(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),

        PrimitiveTy::Long => Ok(Primitive::Long(
            u32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),

        PrimitiveTy::Rational => Ok(Primitive::Rational(Rational {
            numerator: u32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
            denominator: u32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        })),

        PrimitiveTy::Undefined => Ok(Primitive::Undefined(
            u8.parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),

        PrimitiveTy::SLong => Ok(Primitive::SLong(
            i32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),

        PrimitiveTy::SRational => Ok(Primitive::Rational(Rational {
            numerator: u32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
            denominator: u32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        })),

        PrimitiveTy::Utf8 => Ok(Primitive::Utf8(
            u8.parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),
    }
}
