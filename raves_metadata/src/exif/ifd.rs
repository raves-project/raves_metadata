use winnow::{
    Parser,
    binary::{u16, u32},
    error::EmptyError,
};

use crate::exif::State;

use super::{
    NextIfdPointer, Stream,
    error::{ExifFatalError, ExifFieldError},
    value::parse_value,
};
use raves_metadata_types::exif::{
    Field, FieldData, FieldTag,
    ifd::IfdGroup,
    primitives::Primitive,
    tags::{Ifd0Tag, KnownTag, SUB_IFD_POINTER_TAGS},
};

/// An image file directory found within Exif metadata.
///
/// These contain a number of fields - at least one - and directions to the
/// next IFD.
#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, PartialOrd)]
pub struct Ifd {
    /// A list of fields on this IFD.
    pub fields: Vec<Result<Field, ExifFieldError>>,

    /// The sub-IFDs present on this IFD.
    pub sub_ifds: Vec<Ifd>,
}

/// Parses out an entire IFD.
pub fn parse_ifd(input: &mut Stream) -> Result<(Ifd, NextIfdPointer), ExifFatalError> {
    let endianness = *input.state.endianness;

    let entry_count: u16 = u16(endianness).parse_next(input).map_err(|_: EmptyError| {
        log::error!("Couldn't find count on IFD - ran out of data!");
        ExifFatalError::IfdNoEntryCount
    })?;

    if entry_count == 0 {
        log::error!("IFD reported itself as having zero fields! This is fatal to parsing.");
        return Err(ExifFatalError::IfdHadZeroFields);
    }

    // parse all fields on this IFD
    log::trace!("Parsing `{entry_count}` fields...");
    let mut ifd = Ifd {
        fields: (0..entry_count).map(|_| parse_value(input)).collect(),
        sub_ifds: Vec::new(),
    };
    log::trace!("Completed field parsing!");

    // check for any sub-ifds
    log::trace!("Checking for sub-IFDs...");
    let sub_ifds: Vec<Ifd> = ifd
        .fields
        .iter()
        .flatten()
        .filter(|field| SUB_IFD_POINTER_TAGS.contains(&field.tag))
        .flat_map(|sub_ifd_field| {
            let ptr: u32 = match sub_ifd_field.data {
                FieldData::Primitive(Primitive::Long(long)) => long,
                _ => {
                    log::error!(
                        "Found a sub-IFD, but its field data wasn't a long! got: {sub_ifd_field:#?}"
                    );
                    return None;
                }
            };

            let ifd_group = match sub_ifd_field.tag {
                FieldTag::Known(KnownTag::Ifd0Tag(tag)) => match tag {
                    Ifd0Tag::ExifIfdPointer => IfdGroup::Exif,
                    Ifd0Tag::GpsInfoIfdPointer => IfdGroup::Gps,
                    Ifd0Tag::InteroperabilityIfdPointer => IfdGroup::Interop,
                    _ => todo!(),
                },

                _ => todo!(),
            };

            Some((ifd_group, ptr))
        })
        .flat_map(|(ifd_group, ptr)| {
            // skip to the IFD in the original blob
            let new_ifd_input = &input.state.blob[ptr as usize..];

            // construct the next state
            let state = &mut Stream {
                input: new_ifd_input,
                state: State {
                    endianness: &endianness,
                    blob: input.state.blob,
                    current_ifd: ifd_group,
                },
            };

            let (sub_ifd, uhh_offset_of_next_todo_maybe_use) = parse_ifd.parse_next(state).ok()?;
            Some(sub_ifd)
        })
        .collect();
    log::trace!("Found {} sub-IFD(s)! Returning...", sub_ifds.len());

    // set the sub-IFDs on the parent IFD
    ifd.sub_ifds = sub_ifds;

    Ok((ifd, next_ifd_location(input)))
}

fn next_ifd_location(input: &mut Stream) -> Option<u32> {
    let endianness = *input.state.endianness;

    let raw_location: u32 = u32(endianness)
        .parse_next(input)
        .inspect_err(|_: &EmptyError| {
            log::debug!("IFD didn't contain a pointer to the next IFD!");
        })
        .ok()?;

    if raw_location == 0_u32 {
        log::trace!("There won't be a next IFD.");
        None
    } else {
        log::trace!("Another IFD was detected! index: `{raw_location}`");
        Some(raw_location)
    }
}
