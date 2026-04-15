use crate::providers::png::chunks::{
    ihdr::{BitDepth, ColorType},
    trns::TrnsContext,
};

/// An error that occurs when constructing a [`Png`] for its metadata.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum PngConstructionError {
    /// The file ran out of bytes before we could check for a signature.
    ///
    /// It might be empty.
    NoSignature,

    /// No PNG signature was detected.
    NotAPng {
        /// The signature that was found instead.
        found: [u8; 8],
    },

    /// Unexpectedly ran out of bytes when parsing a chunk.
    OuttaBytes {
        /// The chunk's name. (e.g., `IHDR`)
        chunk_type: &'static str,
        /// The number of bytes that we tried to get.
        expected: u32,
        /// The number of bytes that were actually remaining.
        remaining: u32,
    },

    /// Failed to parse chunk header.
    NotEnoughBytesForChunkHeader,

    /// In the IHDR chunk, a disallowed bit depth was found.
    DisallowedBitDepth {
        /// The weird bit depth value we found.
        found_value: u8,
    },

    /// In the IHDR chunk, a disallowed bit depth and color type pair was
    /// found.
    BitDepthIncompatibleWithColorType {
        /// The bit depth value provided.
        bit_depth: BitDepth,

        /// The expected color type.
        incompatible_color_type: ColorType,
    },

    /// In the IHDR chunk, a disallowed color type byte was found.
    DisallowedColorType {
        /// The weird color type byte we found.
        found_value: u8,
    },

    /// In the IHDR chunk, a weird compression method was found.
    DisallowedCompressionMethod {
        /// The disallowed byte value.
        found_value: u8,
    },

    /// In the IHDR chunk, a weird filter method was found.
    DisallowedFilterMethod {
        /// The disallowed byte value.
        found_value: u8,
    },

    /// In the IHDR chunk, a weird interlace method was found.
    DisallowedInterlaceMethod {
        /// The disallowed byte value.
        found_value: u8,
    },

    /// In the PLTE chunk, the chunk length was not a multiple of 3.
    PlteNotMultipleOfThree {
        /// The chunk length we got.
        found_chunk_length: u32,
    },

    /// In the PLTE chunk, there were too many palettes (>256).
    PlteTooManyPalettes {
        /// The number of palettes found.
        palette_ct: u32,
    },

    /// The IEND chunk had a non-zero chunk length, but that's not allowed.
    IendHadData {
        /// The (non-zero) chunk length for this chunk.
        chunk_length: u32,
    },

    /// The tRNS chunk should not exist if the color type is disallowed, but
    /// the parser found a tRNS chunk in the datastream anyway.
    TrnsGivenDisallowedType,

    /// An iCCP chunk profile name should be fully Latin-1-encoded, but a
    /// character wasn't in Latin-1 bounds.
    IccpProfileNameNotLatin1 {
        /// The violating character.
        c: u8,
    },

    /// The iCCP chunk profile name violated PNG's structural requirements.
    IccpInvalidProfileName {
        /// The reason the profile name was invalid.
        reason: &'static str,
    },

    /// The iCCP chunk's compression method should always be zero, but another
    /// value was found.
    IccpUnknownCompressionValue {
        /// The non-zero value that was found.
        value: u8,
    },
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

            Self::OuttaBytes {
                chunk_type,
                expected,
                remaining,
            } => write!(
                f,
                "Unexpectedly ran out of bytes when parsing chunk: `{chunk_type}`. \
                Expected `{expected}` more bytes, \
                but only found `{remaining}` bytes.",
            ),

            other_TODO => todo!(),
        }
    }
}

impl core::error::Error for PngConstructionError {}

/// An error that can occur when writing a PNG back to disk.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum PngWriteError {
    CantWriteToBuf {},

    /// On the PLTE chunk, the number of palettes was either 0 or >256.
    PltePaletteCount {
        /// The number of palettes.
        palette_ct: u32,
    },

    /// An iCCP chunk profile name should be fully Latin-1-encoded, but a
    /// character wasn't in Latin-1 bounds.
    IccpProfileNameNotLatin1 {
        /// The violating character.
        c: u8,
    },

    /// The iCCP chunk profile name violated PNG's structural requirements.
    IccpInvalidProfileName {
        /// The reason the profile name was invalid.
        reason: &'static str,
    },
}

fn TODO_impl_error_for_PngWriteError() {}
