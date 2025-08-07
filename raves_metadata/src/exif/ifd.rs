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

/// A limit on recursion.
///
/// This means that one IFD may have `RECURSION_LIMIT` layers of sub-IFDs, but
/// passing the limit will stop parsing.
pub const RECURSION_LIMIT: u8 = 32;

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

    // add the IFD's pointer to the call stack
    {
        let ifd_ptr: u32 = (input.state.blob.len() - input.len()) as u32;

        // first, check if the IFD was already ckd (i.e., self recursion)
        for maybe_ptr in &input.state.recursion_stack[..input.state.recursion_ct as usize] {
            let Some(ptr) = maybe_ptr else {
                unreachable!(
                    "there should be `RECURSION` elements in the array. this is a bug - please report it!"
                );
            };

            if ifd_ptr == *ptr {
                return Err(ExifFatalError::SelfRecursion {
                    ifd_group: input.state.current_ifd,
                    call_stack: Box::new(input.state.recursion_stack),
                });
            }
        }

        // then, update the recursion stack
        update_recursion_stack_or_error(input, ifd_ptr)?;
    }

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
                    recursion_ct: input.state.recursion_ct.saturating_add(1_u8),
                    recursion_stack: input.state.recursion_stack,
                },
            };

            // update its recursion stack
            update_recursion_stack_or_error(state, ptr).ok()?;

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

/// Attempts to update the given `input` stream's recursion stack.
///
/// This function assumes you have already incremented the `recursion_ct`, so
/// please ensure you have done so before calling.
///
/// # Errors
///
/// If the new IFD pointer won't fit, this returns an error.
fn update_recursion_stack_or_error(input: &mut Stream, ifd_ptr: u32) -> Result<(), ExifFatalError> {
    // if this hits the recursion limit, ret an error
    if input.state.recursion_ct >= RECURSION_LIMIT {
        // uh-oh!
        //
        // there are no more `None` spots, meaning the parser has hit the recursion
        // limit.
        //
        // this is a fatal error, so let's warn in the terminal and return an error
        log::error!("Hit IFD recursion limit! input: {input:#?}");
        return Err(ExifFatalError::HitRecursionLimit {
            ifd_group: input.state.current_ifd,
            call_stack: Box::new(input.state.recursion_stack.map(|c| {
                // note: this unwrap is fine, as we've already confirmed above that
                // all element of the array are `None`
                c.unwrap()
            })),
        });
    }

    // otherwise, set the next element that's `None` and return happily :)
    debug_assert!(
        input.state.recursion_stack[input.state.recursion_ct as usize].is_none(),
        "array indexing should be correct, but `stack[{}]` is not None!",
        input.state.recursion_ct
    );
    input.state.recursion_stack[input.state.recursion_ct as usize] = Some(ifd_ptr);
    Ok(())
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::exif::ifd::IfdGroup;

    use crate::exif::{Stream, error::ExifFatalError, ifd::RECURSION_LIMIT};

    /// If we hit the recursion limit, the `update_recursion` func should
    /// return an error indicating that.
    #[test]
    fn hitting_recursion_limit_should_return_err() {
        logger();

        let bytes = [0_u8; 200];
        let mut state = crate::exif::State {
            blob: &bytes,
            current_ifd: IfdGroup::_0,
            endianness: &winnow::binary::Endianness::Big,
            recursion_ct: RECURSION_LIMIT,
            recursion_stack: (0..RECURSION_LIMIT as u32)
                .map(Some)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        };

        // update the recursion count, as required by the function
        state.recursion_ct += 1;

        let res = super::update_recursion_stack_or_error(
            &mut Stream {
                input: &bytes,
                state,
            },
            RECURSION_LIMIT as u32,
        );

        assert!(
            matches!(res.unwrap_err(), ExifFatalError::HitRecursionLimit { .. }),
            "should hit recursion limit"
        );
    }

    /// helper: init logger
    fn logger() {
        _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();
    }
}
