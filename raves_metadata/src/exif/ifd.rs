use winnow::{
    Parser,
    binary::{u16, u32},
    error::EmptyError,
};

use super::{
    NextIfdPointer, Stream,
    error::{ExifFatalError, ExifFieldError},
    value::parse_value,
};
use raves_metadata_types::exif::{Field, ifd::IfdGroup};

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

/// A sub-IFD of the primary IFD.
///
/// Each one of these has a group, representing which fields it may have, and a
/// pointer in the Exif data slice.
pub struct SubIfd {
    group: IfdGroup,
    pointer: u32,
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
            log::trace!("Another IFD was detected! index: `{raw_location}`");
            Some(raw_location)
        }
    };

    Ok((ifd, next_ifd_location))
}
