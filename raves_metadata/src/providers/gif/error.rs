use crate::providers::gif::GctMissingColor;

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
    NoGct {
        /// The number of RGB triplets expected in the GCT.
        expected_triplet_ct: u8,

        /// The triplet that wasn't found.
        errant_triplet: u8,

        /// The color that wasn't found.
        missing_color: GctMissingColor,
    },

    /// Found an extension block, but it ended earlier than it should have!
    ///
    /// If other programs work well with this file, please report this!
    ExtensionStoppedAbruptly(
        /// The number of bytes expected to continue parsing.
        u8,
    ),

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

    /// The trailer block was missing.
    TrailerMissing,

    /// The Traler block had an incorrect (unexpected) value.
    ///
    /// It should be 0x3b.
    TrailerIncorrectValue(
        /// The (incorrect) value we found.
        u8,
    ),
}

impl core::error::Error for GifConstructionError {}

impl core::fmt::Display for GifConstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let t = todo!();
    }
}
