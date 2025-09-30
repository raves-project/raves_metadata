#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum JpegConstructionError {
    /// The first marker in a JPEG file should be a `SOI`.
    ///
    /// However, this file had another marker first.
    FirstMarkerWasNotSoi {
        /// The first marker's marker code.
        marker_code: u8,
    },

    /// The first byte of a marker should be `0xFF`.
    ///
    /// It wasn't!
    FirstMarkerByteWasWrong(u8),

    /// Failed to get a marker code.
    ///
    /// Might be out of data before finding one.
    NoMarkerCode,

    /// A marker code was `0` or `255`, but those values are disallowed.
    MarkerCodeDisallowed(u8),

    /// This marker code has a known length, but its length wasn't found.
    NoLength {
        /// The marker code for which the length was not found.
        marker_code: u8,
    },

    /// A marker had a negative length (after removing 2 len bytes).
    NegativeLength {
        /// The afflicted marker's marker code.
        marker_code: u8,

        /// Its original length, including the marker length bytes.
        original_len: u16,
    },

    /// Not enough data for marker payload.
    NoDataForPayload {
        /// The afflicted marker's marker code.
        marker_code: u8,

        /// Its original length, including the marker length bytes.
        original_len: u16,

        /// The remaining length in the input, as of parsing.
        ///
        /// This should be more than `original_len`, but it wasn't!
        remaining_input_len: u64,
    },

    /// Ran outta data when parsing APP1.
    OuttaDataForApp1,

    /// Ran outta data when parsing SOS.
    OuttaDataForSos,

    /// No more data for grabbing the ExtendedXMP's offset.
    NoOffsetForExtendedXmp,

    /// We found multiple StandardXMP blobs.
    ///
    /// We can't use them since we don't know what order they're in.
    MultipleStandardXmpBlobs,

    /// Attempted to concatenate `ExtendedXMP` blobs, but there wasn't a
    /// `StandardXMP` blob to start with!
    CantConcatExtendedXmpWithoutStandardXmp,

    /// Couldn't find a chunk of `XMP` metadata required for `ExtendedXMP`.
    ExtendedXmpMissingChunk {
        /// The offset at which the missing chunk was expected to be.
        offset: u32,
    },

    /// We're required to parse the XMP metadata to remove a marker tag before
    /// providing things to the user, per the XMP standard.
    ///
    /// However, the XMP failed to parse, meaning this is an invalid JPEG.
    XmpDidntParse(crate::XmpError),

    /// No GUID was found for the ExtendedXMP, but ExtendedXMP was present.
    ExtendedXmpCouldntFindGuid,

    /// The detected GUID was not 32 bytes long.
    ExtendedXmpGuidNot32Bytes(String),

    /// Failed to write modified XMP back to `Vec`.
    ExtendedXmpWriteFailure,
}

impl core::fmt::Display for JpegConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstMarkerWasNotSoi { marker_code } => write!(
                f,
                "A JPEG file's first marker should be SOI (`0xD8`), \
                but it was: `{marker_code:x?}`"
            ),

            Self::FirstMarkerByteWasWrong(other) => write!(
                f,
                "JPEG marker's first byte was wrong. \
                    expected: `255`; \
                    got: `{other}`",
            ),

            Self::NoMarkerCode => f.write_str(
                "Failed to get a marker code. \
                    Might be out of data!",
            ),

            Self::MarkerCodeDisallowed(code) => write!(
                f,
                "A JPEG marker code had a disallowed value. \
                        expected: any value that's not `0` or `255`; \
                        got: `{code}`",
            ),

            Self::NoLength { marker_code } => write!(
                f,
                "JPEG marker segment with code `{marker_code}` had no length. \
                    (out of data!) ",
            ),

            Self::NegativeLength {
                marker_code,
                original_len,
            } => write!(
                f,
                "JPEG marker segment with code `{marker_code}` had \
                    a length that becomes negative after removing 2: \
                    `{original_len}` bytes"
            ),

            Self::NoDataForPayload {
                marker_code,
                original_len: payload_len,
                remaining_input_len,
            } => write!(
                f,
                "Not enough data left in input for payload. \
                    marker code: `{marker_code}`, \
                    payload len: `{payload_len}` bytes, \
                    remaining input len: `{remaining_input_len}` bytes"
            ),

            Self::OuttaDataForApp1 => f.write_str(
                "Ran out of data when parsing APP1. \
                Not enough data for payload.",
            ),

            Self::OuttaDataForSos => f.write_str(
                "Ran out of data when parsing SOS. \
                Not enough data for payload.",
            ),

            Self::NoOffsetForExtendedXmp => {
                f.write_str("No more data for parsing ExtendedXMP's offset.")
            }

            Self::MultipleStandardXmpBlobs => f.write_str(
                "Found multiple StandardXMP blobs, but these \
                    should be independent.",
            ),

            Self::CantConcatExtendedXmpWithoutStandardXmp => f.write_str(
                "Tried to concatenate ExtendedXMP blobs, but no StandardXMP \
                blob was found to start with!",
            ),

            Self::ExtendedXmpMissingChunk { offset } => write!(
                f,
                "Missing an ExtendedXMP blob at byte offset: \
                    `{offset}`"
            ),

            Self::XmpDidntParse(e) => write!(
                f,
                "XMP failed to parse. \
                Checking this is required when the file contains XMP. err: {e}"
            ),

            Self::ExtendedXmpCouldntFindGuid => f.write_str(
                "No GUID was found for the ExtendedXMP, \
                    but ExtendedXMP was present.",
            ),

            Self::ExtendedXmpGuidNot32Bytes(guid) => write!(
                f,
                "The found GUID was not 32 bytes long! got: `{guid}`, \
                which is `{}` bytes long",
                guid.len()
            ),

            Self::ExtendedXmpWriteFailure => f.write_str("Failed to write back to StandardXMP."),
        }
    }
}

impl core::error::Error for JpegConstructionError {}
