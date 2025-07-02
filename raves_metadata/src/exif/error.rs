use raves_metadata_types::exif::{Field, primitives::PrimitiveTy};

/// This type describes the parsing result.
///
/// In summary, if it's the `Err` variant, the parsing failed completely, and
/// you aren't getting any Exif data at all.
///
/// `Ok` means that parsing largely succeeded, but each field in the list is
/// still wrapped in a `Result`. For more info, see [`ExifFieldResult`].
pub type ExifFatalResult<T> = Result<T, ExifFatalError>;

/// Parsing a field may fail due to standard-derived invariants, an incorrect
/// save by another metadata parser/modifier, or other problems.
///
/// In that case, we'll report that inside the list.
pub type ExifFieldResult = Result<Field, ExifFieldError>;

#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum ExifFatalError {
    /// The input was too short to provide a byte order marker.
    NoByteOrderMarker { len: u8 },

    /// The byte order marker was weird - it's not one of the two expected
    /// values (in ASCII, should be either `II` or `MM`).
    WeirdByteOrderMarker { found: [u8; 2] },

    /// Didn't find the TIFF magic number.
    NoTiffMagicNumber,

    /// The magic number indexes had a weird value. It's not TIFF's.
    MagicNumberWasntTiff { found: u16 },

    /// No TIFF header offset was found.
    NoTiffHeaderOffset,

    /// The header offset would place us before the header! That doesn't make
    /// any sense, so we can't keep parsing.
    HeaderOffsetBeforeHeader,

    /// Failed to skip to the offset - ran outta data.
    NotEnoughDataForHeaderOffset,

    /// The IFD didn't say how many entries it has.
    IfdNoEntryCount,

    /// The IFD was completely blank.
    IfdHadZeroFields,

    /// The IFD didn't give a pointer to the next entry.
    IfdNoPointer,
}

impl winnow::error::ParserError<&[u8]> for ExifFatalError {
    type Inner = Self;

    fn from_input(_input: &&[u8]) -> Self {
        unreachable!("we let winnow make an error without mapping. please report this!") // TODO: take another look at this when time allows
    }

    fn into_inner(self) -> winnow::Result<Self::Inner, Self> {
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum ExifFieldError {
    //
    // field stuff
    /// The field did not provide a tag.
    FieldNoTag,

    /// The field didn't provide a primitive type.
    FieldNoTy,

    /// Encountered an unknown type while parsing n field.
    FieldUnknownType { got: u16 },

    /// The field didn't specify how many primitives it contains.
    FieldNoCount,

    /// The field didn't provide an offset or value.
    FieldNoOffsetOrValue,

    //
    // value parsing stuff
    /// Couldn't parse to offset. It was likely too far (malformed).
    OffsetTooFar { offset: u32 },

    /// Couldn't parse primitive - no more data.
    OuttaData { ty: PrimitiveTy },
}

impl core::fmt::Display for ExifFatalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoByteOrderMarker { len } => {
                write!(f, "No byte order marker was found. len: `{len}`")
            }
            Self::WeirdByteOrderMarker { found } => match core::str::from_utf8(found.as_slice()) {
                Ok(found_utf8_bom) => {
                    write!(f, "Got a weird byte-order marker: `{found_utf8_bom}`")
                }
                Err(e) => {
                    log::error!("Unknown byte-order marker was not ASCII! conversion err: {e}");
                    write!(f, "Got a weird byte-order marker - wasn't ASCII: {found:?}")
                }
            },

            Self::NoTiffMagicNumber => {
                f.write_str("No TIFF magic number found - the slice was likely cut short.")
            }
            Self::MagicNumberWasntTiff { found } => {
                write!(f, "Magic number was not TIFF! got: `{found}`")
            }
            Self::NoTiffHeaderOffset => f.write_str("No TIFF header offset was found."),
            Self::HeaderOffsetBeforeHeader => f.write_str(
                "TIFF header offset asked us to move before the header. Likely a \
                broken file - cannot continue parsing.",
            ),
            Self::NotEnoughDataForHeaderOffset => {
                f.write_str("Not enough data to skip to TIFF header offset.")
            }
            Self::IfdNoEntryCount => f.write_str("The IFD didn't say how many entries it has."),
            Self::IfdHadZeroFields => {
                f.write_str("The IFD told us it had zero fields, which is invalid.")
            }
            Self::IfdNoPointer => f.write_str("The IFD didn't give a pointer to the next entry."),
        }
    }
}

impl core::fmt::Display for ExifFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExifFieldError::FieldNoTag => f.write_str("The field did not provide a tag."),
            ExifFieldError::FieldNoTy => f.write_str("The field didn't provide a primitive type."),
            ExifFieldError::FieldNoCount => {
                f.write_str("The field didn't specify how many primitives it contains.")
            }
            ExifFieldError::FieldNoOffsetOrValue => {
                f.write_str("The field didn't provide an offset or value.")
            }
            ExifFieldError::FieldUnknownType { got } => write!(
                f,
                "Encountered an unknown type while parsing n field! got: {got}"
            ),

            ExifFieldError::OuttaData { ty } => write!(
                f,
                "Couldn't parse primitive - no more data in blob. ty: `{ty:?}`"
            ),
            ExifFieldError::OffsetTooFar { offset } => write!(
                f,
                "Couldn't skip to offset - no more data in blob. offset: `{offset}`"
            ),
        }
    }
}

impl core::error::Error for ExifFatalError {}
impl core::error::Error for ExifFieldError {}
