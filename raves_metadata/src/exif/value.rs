use winnow::{
    Parser as _, Stateful,
    binary::{Endianness as WinnowEndianness, i32, u8, u16, u32},
    error::EmptyError,
};

use super::{
    Stream,
    error::{ExifFieldError, ExifFieldResult},
};
use raves_metadata_types::exif::{Field, FieldData, FieldTag, primitives::*, tags::KnownTag};

/// Parses out one value from an IFD.
pub fn parse_value(input: &mut Stream) -> ExifFieldResult {
    let endianness = input.state.endianness;

    // grab tag (2 bytes)
    let tag: FieldTag = {
        let raw_tag: u16 = u16(*endianness)
            .parse_next(&mut input.input)
            .map_err(|_: EmptyError| ExifFieldError::FieldNoTag)?;

        KnownTag::try_from((input.state.current_ifd, raw_tag))
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
        "(field info...
    tag: {tag},
    ty: {ty:?},
    count: {count},
    value or offset: {value_or_offset}
)"
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
    log::trace!("total size for field: `{total_size}`");

    // figure out what `value_or_offset` really is
    let is_offset_instead_of_inline_value: bool = total_size > 4_u32;
    log::trace!("field has offset instead of inline data..? `{is_offset_instead_of_inline_value}`");
    let value: [u8; 4] = match endianness {
        WinnowEndianness::Big => value_or_offset.to_be_bytes(),
        WinnowEndianness::Little => value_or_offset.to_le_bytes(),
        WinnowEndianness::Native => unreachable!("we never use this variant"),
    };

    // if the value is an offset, apply the offset and use the shifted blob as
    // the buffer. (offsets are relative to the beginning of the blob)
    //
    // if it's not, just use our value and leave :)
    let data: &[u8] = match is_offset_instead_of_inline_value {
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

            // it's just a value; send it over as a slice
            value.as_slice()
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
            log::trace!("There are no stored primitives in this field. Returning early!");
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

    // endianness should never be native!
    debug_assert!(
        *endianness != WinnowEndianness::Native,
        "endianness should never be native. this is a bug - please report it!"
    );

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

        PrimitiveTy::SRational => Ok(Primitive::SRational(SRational {
            numerator: i32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
            denominator: i32(*endianness)
                .parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        })),

        PrimitiveTy::Utf8 => Ok(Primitive::Utf8(
            u8.parse_next(input)
                .map_err(|_: EmptyError| ExifFieldError::OuttaData { ty })?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::exif::{
        Field, FieldData, FieldTag,
        ifd::IfdGroup,
        primitives::{Primitive, PrimitiveTy},
    };
    use winnow::binary::Endianness as WinnowEndianness;

    use crate::{
        exif::{
            State, Stream,
            error::ExifFieldError,
            value::{PrimitiveState, PrimitiveStream},
        },
        util::logger,
    };

    /// Unknown types should be rejected.
    #[test]
    fn unknown_type() {
        logger();

        let mut backing_bytes = Vec::new();
        backing_bytes.extend_from_slice(0_u16.to_le_bytes().as_slice()); // field tag id
        backing_bytes.extend_from_slice(0_u16.to_le_bytes().as_slice()); // field type
        backing_bytes.extend_from_slice(1_u32.to_le_bytes().as_slice()); // field count
        backing_bytes.extend_from_slice(0_u32.to_le_bytes().as_slice()); // data

        assert_eq!(
            crate::exif::value::parse_value(&mut Stream {
                input: &backing_bytes,
                state: State {
                    current_ifd: IfdGroup::_0,
                    endianness: &WinnowEndianness::Little,
                    blob: &backing_bytes,
                    recursion_ct: 0,
                    recursion_stack: Default::default(),
                }
            }),
            Err(ExifFieldError::FieldUnknownType { got: 0_u16 })
        );
    }

    /// We should accept a long, unknown field.
    #[test]
    fn long_field() {
        logger();

        let mut backing_bytes = Vec::new();
        backing_bytes.extend_from_slice(666_u16.to_le_bytes().as_slice()); // field tag id
        backing_bytes.extend_from_slice(1_u16.to_le_bytes().as_slice()); // field type
        backing_bytes.extend_from_slice(300_u32.to_le_bytes().as_slice()); // field count
        backing_bytes.extend_from_slice(
            (backing_bytes.len() as u32 + 20_u32)
                .to_le_bytes()
                .as_slice(),
        ); // "the data is in 20 more bytes, including me"
        backing_bytes.extend_from_slice([0_u8; 16].as_slice()); // 16 bytes of padding
        backing_bytes.extend_from_slice([61_u8; 300].as_slice()); // field data

        assert_eq!(
            super::parse_value(&mut Stream {
                input: &backing_bytes,
                state: State {
                    endianness: &WinnowEndianness::Little,
                    blob: &backing_bytes,
                    current_ifd: IfdGroup::_0,
                    recursion_ct: 0,
                    recursion_stack: Default::default(),
                }
            }),
            Ok(Field {
                tag: FieldTag::Unknown(666_u16),
                data: FieldData::List {
                    list: [Primitive::Byte(61_u8); 300].into(),
                    ty: PrimitiveTy::Byte
                }
            })
        );
    }

    #[test]
    fn all_nonrational_exif_primitives_should_parse_under_le_and_be() {
        logger();

        let end_u16 = |v: u16, e: WinnowEndianness| match e {
            WinnowEndianness::Big => v.to_be_bytes(),
            WinnowEndianness::Little => v.to_le_bytes(),
            _ => unreachable!(),
        };

        let _end_i16 = |v: i16, e: WinnowEndianness| match e {
            WinnowEndianness::Big => v.to_be_bytes(),
            WinnowEndianness::Little => v.to_le_bytes(),
            _ => unreachable!(),
        };

        let end_u32 = |v: u32, e: WinnowEndianness| match e {
            WinnowEndianness::Big => v.to_be_bytes(),
            WinnowEndianness::Little => v.to_le_bytes(),
            _ => unreachable!(),
        };

        let end_i32 = |v: i32, e: WinnowEndianness| match e {
            WinnowEndianness::Big => v.to_be_bytes(),
            WinnowEndianness::Little => v.to_le_bytes(),
            _ => unreachable!(),
        };

        for endianness in [WinnowEndianness::Big, WinnowEndianness::Little] {
            log::info!("endianness: {endianness:?}");

            for (ty, value, expected_result) in [
                (
                    PrimitiveTy::Byte,
                    mk_value([4_u8].as_slice()),
                    Primitive::Byte(4_u8),
                ),
                (
                    PrimitiveTy::Ascii,
                    mk_value(b"c".as_slice()),
                    Primitive::Ascii(b'c'),
                ),
                (
                    PrimitiveTy::Short,
                    mk_value(end_u16(u16::MAX, endianness).as_slice()),
                    Primitive::Short(u16::MAX),
                ),
                (
                    PrimitiveTy::Long,
                    mk_value(end_u32(45_u32, endianness).as_slice()),
                    Primitive::Long(45_u32),
                ),
                (
                    PrimitiveTy::Undefined,
                    mk_value(&[10_u8]),
                    Primitive::Undefined(10_u8),
                ),
                (
                    PrimitiveTy::SLong,
                    mk_value(end_i32(-2025_i32, endianness).as_slice()),
                    Primitive::SLong(-2025_i32),
                ),
                (PrimitiveTy::Utf8, mk_value(&[0_u8]), Primitive::Utf8(0_u8)),
            ] {
                log::info!("completing value: ({ty:?}, `{value:x?}`)");

                let mut prim_stream = PrimitiveStream {
                    input: value.as_slice(),
                    state: PrimitiveState {
                        tag: &FieldTag::Unknown(0),
                        endianness: &endianness,
                        count: 1,
                        ty: &ty,
                    },
                };

                let parsed_primitive = super::parse_primitive(&mut prim_stream).unwrap();

                assert_eq!(parsed_primitive.ty(), ty, "types should match");
                assert_eq!(
                    parsed_primitive, expected_result,
                    "reality should match expectation"
                );
            }
        }
    }

    /// helper: create primitive values padded correctly
    fn mk_value(slice: &[u8]) -> [u8; 4] {
        log::debug!("mk_value... input: {slice:?}");
        let mut bytes = [0_u8; 4];
        bytes[0..slice.len()].copy_from_slice(&slice[0..slice.len()]);
        log::debug!("mk_value... output: {bytes:?}");
        bytes
    }

    #[test]
    #[should_panic]
    fn passing_stream_w_native_endianness_should_panic() {
        let mut prim_stream = PrimitiveStream {
            input: &[4_u8; 4],
            state: PrimitiveState {
                tag: &FieldTag::Unknown(0_u16),
                endianness: &WinnowEndianness::Native, // it should panic bc of this
                count: 1,
                ty: &PrimitiveTy::Long,
            },
        };

        // ignore the result; this should cause a panic!
        _ = super::parse_primitive(&mut prim_stream);
    }
}
