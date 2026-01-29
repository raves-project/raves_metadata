use crate::providers::gif::block::GctMissingColor;

/// An error obtained when parsing a GIF file.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum GifConstructionError {
    /// Attempted to parse out magic number, but none was present.
    NoMagicNumber,

    /// The magic number was incorrect.
    WeirdMagicNumber([u8; 3]),

    /// There weren't enough bytes in the stream to get the GIF version for the
    /// file's header.
    ///
    /// This feature is required, so the parse failed.
    NoGifVersion,

    /// The GIF did not contain the required logical screen descriptor.
    LogicalScreenDescriptorMissingData,

    /// The LSD said that there should be a GCT, but a triplet was missing.
    GlobalColorTableMissingTriplet {
        /// The number of RGB triplets expected in the GCT.
        expected_triplet_ct: u16,

        /// The triplet that wasn't found.
        errant_triplet: u8,

        /// The color that wasn't found.
        missing_color: GctMissingColor,
    },

    /// Unknown block found during the repeatable block section.
    UnknownBlockFound {
        /// The block's first byte.
        byte: u8,
    },

    /// Unknown extension type found during the repeatable block section.
    UnknownExtensionFound {
        /// The block's label byte.
        label: u8,
    },

    /// Extension had an unexpected block size.
    ExtensionHasWeirdBlockSize {
        /// The block size reported.
        got: u8,

        /// The block size expected by the extension type.
        expected: u8,
    },

    /// The GIF 87a (1987 rev. a) specification does not support extension
    /// blocks, but one was present anyway.
    ExtensionFoundInGif87,

    /// Not enough bytes were found.
    NotEnoughBytes,

    /// Block terminator was not `0x00`, but something else.
    BlockTerminatorMismatch(
        /// The value found instead of `0x00`.
        u8,
    ),

    /// The image descriptor had no separator.
    ImageDescriptorNoSeparator,

    /// The image descriptor has the wrong separator.
    ///
    /// It should be `0x2c`.
    ImageDescriptorSeparatorWrong(
        /// The incorrect separator that we found.
        u8,
    ),

    /// The image descriptor is missing data.
    ///
    /// Please check logs for more information.
    ImageDescriptorMissingData,

    /// The table-based image data block was missing its LZW
    /// size field.
    TableBasedImageDataNoLzw,

    /// The graphics control extension block is missing data.
    ///
    /// Please check logs for more information.
    GraphicExtMissingData,

    /// Comment extension was missing data.
    ///
    /// Please check logs for more information.
    CommentExtMissingData,

    /// The app extension is missing data.
    ///
    /// Please check logs for more information.
    AppExtMissingData,

    /// The plain text extension is missing data.
    ///
    /// Please check the logs for more information.
    PlainTextExtMissingData,

    /// The trailer block was missing.
    TrailerMissing,
}

impl core::error::Error for GifConstructionError {}

impl core::fmt::Display for GifConstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            GifConstructionError::NoMagicNumber => {
                f.write_str("Attempted to parse out magic number, but none was present.")
            }
            GifConstructionError::WeirdMagicNumber(found) => write!(
                f,
                "The given file was not a GIF. \
                First bytes should be `[G, I, F]`, but got: `{found:x?}`!",
            ),
            GifConstructionError::NoGifVersion => f.write_str(
                "There weren't enough bytes in the stream to get the \
                GIF version for the file's header.",
            ),
            GifConstructionError::LogicalScreenDescriptorMissingData => {
                f.write_str("The GIF did not contain the required logical screen descriptor.")
            }
            GifConstructionError::GlobalColorTableMissingTriplet {
                expected_triplet_ct,
                errant_triplet,
                missing_color,
            } => write!(
                f,
                "The global color table is missing a triplet. \
                Errant triplet was {errant_triplet}/{expected_triplet_ct} \
                on color {missing_color:?}."
            ),
            GifConstructionError::UnknownBlockFound { byte } => write!(
                f,
                "Unknown block found when parsing repeatable blocks. \
                    Block's first byte: `0x{byte:x}`",
            ),
            GifConstructionError::UnknownExtensionFound { label } => write!(
                f,
                "Unknown extension found when parsing repeatable blocks. \
                    Extension's label: `0x{label:x}`",
            ),
            GifConstructionError::ExtensionHasWeirdBlockSize { got, expected } => {
                write!(
                    f,
                    "Extension had an unexpected block size! \
                    Got: {got} bytes, but expected: {expected} bytes."
                )
            }
            GifConstructionError::ExtensionFoundInGif87 => f.write_str(
                "A GIF of version 87a contained an extension, which violates the standard.",
            ),
            GifConstructionError::NotEnoughBytes => {
                f.write_str("Parser expected more bytes, but the input ran out of data.")
            }
            GifConstructionError::BlockTerminatorMismatch(got) => {
                write!(
                    f,
                    "Expected block terminator (`0x00`), but found: `0x{got:x}`."
                )
            }
            GifConstructionError::ImageDescriptorNoSeparator => {
                f.write_str("An image descriptor had no separator.")
            }
            GifConstructionError::ImageDescriptorSeparatorWrong(got) => write!(
                f,
                "Expected separator in image descriptor (`0x2c`), but found: `0x{got:x}`."
            ),
            GifConstructionError::ImageDescriptorMissingData => {
                f.write_str("An image descriptor was missing required fields.")
            }
            GifConstructionError::TableBasedImageDataNoLzw => {
                f.write_str("Table-based image data is missing its LZW size field.")
            }
            GifConstructionError::GraphicExtMissingData => {
                f.write_str("Graphic control extension is missing data.")
            }
            GifConstructionError::CommentExtMissingData => {
                f.write_str("Comment extension is missing data.")
            }
            GifConstructionError::AppExtMissingData => {
                f.write_str("Application extension is missing data.")
            }
            GifConstructionError::PlainTextExtMissingData => {
                f.write_str("Plain-text extension is missing data.")
            }
            GifConstructionError::TrailerMissing => f.write_str(
                "The provided GIF file abrupted stopped without its required trailer (end) block.",
            ),
        }
    }
}
