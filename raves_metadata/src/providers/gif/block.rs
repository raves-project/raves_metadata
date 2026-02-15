//! Types for the building "blocks" of GIF files.

use winnow::{Parser, binary::le_u16, binary::u8, error::EmptyError, token::take};

use super::error::GifConstructionError;

/// the magic number should be `b"GIF"`.
pub(super) fn signature(input: &mut &[u8]) -> Result<(), GifConstructionError> {
    log::trace!("Parsing: signature.");

    let bytes: [u8; 3] = take::<_, _, EmptyError>(3_usize)
        .parse_next(input)
        .ok()
        .and_then(|arr| TryInto::<[u8; 3]>::try_into(arr).ok())
        .ok_or_else(|| {
            log::error!("Not enough bytes in the data stream to find magic number!");
            GifConstructionError::NoMagicNumber
        })?;

    // check that it's not ewird
    if bytes != *b"GIF" {
        log::error!("Got a weird magic number -- not a GIF.");
        return Err(GifConstructionError::WeirdMagicNumber(bytes));
    }

    // all good. return nothing
    Ok(())
}

/// Parses a Data Sub-Block.
pub(super) fn data_sub_block(
    input: &mut &[u8],
    output: &mut Vec<u8>,
) -> Result<(), GifConstructionError> {
    log::trace!("Parsing: data sub-block.");

    // grab sub-block data
    let slice: &[u8] = u8
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::warn!("No block size for data sub-block!"))
        .and_then(|block_size: u8| {
            take(block_size).parse_next(input).inspect_err(|_e| {
                log::warn!("Failed to take {block_size} bytes for data sub-block!")
            })
        })
        .map_err(|_e: EmptyError| GifConstructionError::NotEnoughBytes)?;

    // add that to the output list
    output.extend_from_slice(slice);

    // return a happy result
    Ok(())
}

/// Parses a Block Terminator.
pub(super) fn block_terminator(input: &mut &[u8]) -> Result<(), GifConstructionError> {
    // grab the next byte
    let byte: u8 = u8
        .parse_next(input)
        .map_err(|_e: EmptyError| GifConstructionError::NotEnoughBytes)
        .inspect_err(|_e| log::warn!("Failed to parse block terminator! (no byte found)"))?;

    // byte must be 0x00
    if byte != 0x00 {
        return Err(GifConstructionError::BlockTerminatorMismatch(byte));
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct GifHeader {
    pub version: [u8; 3],
}

/// Parses the GIF's Header.
pub(super) fn header(input: &mut &[u8]) -> Result<GifHeader, GifConstructionError> {
    fn gif_version(input: &mut &[u8]) -> Result<[u8; 3], GifConstructionError> {
        // grab the first three bytes
        let arr: [u8; 3] = take::<_, _, EmptyError>(3_usize)
            .parse_next(input)
            .ok()
            .and_then(|arr| TryInto::<[u8; 3]>::try_into(arr).ok())
            .ok_or_else(|| {
                log::error!("Not enough bytes for GIF version.");
                GifConstructionError::NoGifVersion
            })?;

        // warn user if it's an unexpected value
        if ![b"87a", b"89a"].contains(&&arr) {
            let chars: [char; 3] = arr.map(char::from);
            log::warn!("Unknown GIF version provided: `{chars:?}`");
        }

        Ok(arr)
    }

    // ck signature (`GIF`)
    signature(input)?;

    // grab version
    let version: [u8; 3] = gif_version(input)?;

    // return a header
    Ok(GifHeader { version })
}

#[derive(Clone, Debug)]
pub struct LogicalScreenDescriptor {
    pub logical_screen_width: u16,

    pub logical_screen_height: u16,

    pub global_color_table_flag: bool,
    pub color_resolution: u8, // range: 1..=4
    pub sort_flag: bool,
    pub size_of_global_color_table: u8,

    pub background_color_index: u8,
    pub pixel_aspect_ratio: Option<u8>,
}

/// Parses the Logical Screen Descriptor.
pub(super) fn logical_screen_descriptor(
    input: &mut &[u8],
) -> Result<LogicalScreenDescriptor, GifConstructionError> {
    log::trace!("Parsing: logical screen descriptor.");

    let logical_screen_width: u16 = le_u16
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| {
            log::warn!("Logical screen descriptor didn't contain logical screen width!")
        })
        .map_err(|_e: EmptyError| GifConstructionError::LogicalScreenDescriptorMissingData)?;

    let logical_screen_height: u16 = le_u16
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| {
            log::warn!("Logical screen descriptor didn't contain logical screen height!")
        })
        .map_err(|_e: EmptyError| GifConstructionError::LogicalScreenDescriptorMissingData)?;

    let packed: u8 = u8
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::warn!("Logical screen descriptor has no packed field!"))
        .map_err(|_e: EmptyError| GifConstructionError::LogicalScreenDescriptorMissingData)?;
    let global_color_table_flag: bool = (packed & 0b1000_0000) == 0b1000_0000;
    let color_resolution: u8 = ((packed & 0b0111_0000) >> 4) + 1;
    let sort_flag: bool = (packed & 0b0000_1000) == 0b0000_1000;
    let size_of_global_color_table: u8 = packed & 0b0000_0111;

    let background_color_index: u8 = u8
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| {
            log::warn!("Logical screen descriptor has no background color index!")
        })
        .map_err(|_e: EmptyError| GifConstructionError::LogicalScreenDescriptorMissingData)?;

    let pixel_aspect_ratio: u8 = u8
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| {
            log::warn!("Logical screen descriptor has no pixel aspect ratio!")
        })
        .map_err(|_e: EmptyError| GifConstructionError::LogicalScreenDescriptorMissingData)?;

    Ok(LogicalScreenDescriptor {
        logical_screen_width,
        logical_screen_height,
        global_color_table_flag,
        color_resolution,
        sort_flag,
        size_of_global_color_table,
        background_color_index,
        pixel_aspect_ratio: if pixel_aspect_ratio == 0 {
            None
        } else {
            Some(pixel_aspect_ratio)
        },
    })
}

#[derive(Clone, Debug)]
pub struct GlobalColorTable {
    pub rgb_triplets: Vec<(u8, u8, u8)>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum GctMissingColor {
    Red,
    Green,
    Blue,
}

/// Parses the Global Color Table.
///
/// Only present if `LogicalScreenDescriptor.global_color_table_flag` is
/// `true`.
pub(super) fn global_color_table(
    size_of_global_color_table: u8,
    input: &mut &[u8],
) -> Result<GlobalColorTable, GifConstructionError> {
    log::trace!("Parsing: global color table.");

    let triplet_ct: u16 = 2_u16.pow(size_of_global_color_table as u32 + 1_u32);
    let mut v: Vec<(u8, u8, u8)> = Vec::with_capacity(triplet_ct as usize);

    // define color getter (helper closure)
    let mut get_color =
        |color_name: &'static str, color: GctMissingColor, triplet_num: u16| {
            u8.parse_next(input)
            .inspect_err(|_e: &EmptyError| {
                log::warn!(
                    "Global color table missing {color_name} at triplet {triplet_num}/{triplet_ct}!"
                )
            })
            .map_err(|_e: EmptyError| GifConstructionError::GlobalColorTableMissingTriplet {
                expected_triplet_ct: triplet_ct,
                errant_triplet: triplet_num as u8,
                missing_color: color,
            })
        };

    // find and set each triplet
    for triplet_num in 0..triplet_ct {
        // grab each color in triplet
        let (red, green, blue): (u8, u8, u8) = (
            get_color("red", GctMissingColor::Red, triplet_num)?,
            get_color("green", GctMissingColor::Green, triplet_num)?,
            get_color("blue", GctMissingColor::Blue, triplet_num)?,
        );

        // set in the list
        v.insert(triplet_num as usize, (red, green, blue));
    }

    Ok(GlobalColorTable { rgb_triplets: v })
}

#[derive(Clone, Debug)]
pub struct ImageDescriptor {
    pub image_left_position: u16,

    pub image_top_position: u16,

    pub image_width: u16,

    pub image_height: u16,

    pub local_color_table_flag: bool,
    pub interlace_flag: bool,
    pub sort_flag: bool,

    pub size_of_local_color_table: u8,
}

/// Parses the Image Descriptor block.
pub(super) fn image_descriptor(input: &mut &[u8]) -> Result<ImageDescriptor, GifConstructionError> {
    log::trace!("Parsing: image descriptor.");

    // grab and check image separator (constant value)
    const IMAGE_SEPARATOR: u8 = 0x2c;
    let image_separator: u8 = u8
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::error!("Image descriptor had no image separator!"))
        .map_err(|_e: EmptyError| GifConstructionError::ImageDescriptorNoSeparator)?;
    if image_separator != IMAGE_SEPARATOR {
        log::error!(
            "Image descriptor had wrong image separator! \
            got: `0x{image_separator:x}`, expected: 0x{IMAGE_SEPARATOR:x} "
        );
        return Err(GifConstructionError::ImageDescriptorSeparatorWrong(
            image_separator,
        ));
    }

    // image left position
    let image_left_position: u16 = le_u16
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::error!("Image descriptor had no image left position!"))
        .map_err(|_e: EmptyError| GifConstructionError::ImageDescriptorMissingData)?;

    // image top position
    let image_top_position: u16 = le_u16
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::error!("Image descriptor had no image top position!"))
        .map_err(|_e: EmptyError| GifConstructionError::ImageDescriptorMissingData)?;

    // image width
    let image_width: u16 = le_u16
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::error!("Image descriptor had no image width!"))
        .map_err(|_e: EmptyError| GifConstructionError::ImageDescriptorMissingData)?;

    // image height
    let image_height: u16 = le_u16
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::error!("Image descriptor had no image height!"))
        .map_err(|_e: EmptyError| GifConstructionError::ImageDescriptorMissingData)?;

    let packed: u8 = u8
        .parse_next(input)
        .inspect_err(|_e: &EmptyError| log::error!("Image descriptor had no packed field!"))
        .map_err(|_e: EmptyError| GifConstructionError::ImageDescriptorMissingData)?;

    let local_color_table_flag: bool = packed & 0b1000_0000 == 0b1000_0000;
    let interlace_flag: bool = packed & 0b0100_0000 == 0b0100_0000;
    let sort_flag: bool = packed & 0b0010_0000 == 0b0010_0000;
    let _reserved = ();
    let size_of_local_color_table: u8 = packed & 0b0000_0111;

    Ok(ImageDescriptor {
        image_left_position,
        image_top_position,
        image_width,
        image_height,
        local_color_table_flag,
        interlace_flag,
        sort_flag,
        size_of_local_color_table,
    })
}

pub type LocalColorTable = GlobalColorTable;

/// Parses the Local Color Table block.
pub(super) fn local_color_table(
    size: u8,
    input: &mut &[u8],
) -> Result<LocalColorTable, GifConstructionError> {
    log::trace!("Parsing: local color table.");
    global_color_table(size, input)
}

/// Parses table-based image data.
pub(super) fn table_based_image_data(input: &mut &[u8]) -> Result<(), GifConstructionError> {
    log::trace!("Parsing: table-based image data.");

    let _lzw_min_code_size: u8 = u8.parse_next(input).map_err(|_e: EmptyError| {
        log::error!("Table-based image data is missing its LWZ minimum code size field!");
        GifConstructionError::TableBasedImageDataNoLzw
    })?;

    // parse sub-blocks til we find the terminator.
    //
    // TODO: store offsets/indices for later rewriting
    let mut _buf: Vec<u8> = vec![];
    while let Some(b) = input.first()
        && *b != 0x00
    {
        data_sub_block(input, &mut _buf)?;
    }

    // eat the terminator
    block_terminator.parse_next(input)?;

    Ok(())
}

#[derive(Clone, Debug)]
pub struct GraphicControlExtension {
    pub disposal_method: u8,
    pub user_input_flag: bool,
    pub transparent_color_flag: bool,

    pub delay_time: u16,

    pub transparent_color_index: u8,
}

pub(super) fn graphic_control_extension(
    input: &mut &[u8],
) -> Result<GraphicControlExtension, GifConstructionError> {
    log::trace!("Parsing: graphic control extension.");

    // extension introducer
    helpers::extension_introducer.parse_next(input)?;

    // extension label
    helpers::extension_label(input, "Graphic Control Extension", 0xF9)?;

    // block size
    {
        let block_size = helpers::block_size.parse_next(input)?;
        if block_size != 4 {
            log::error!(
                "Graphic control extension had incorrect block size!\
            expected: `4`, got: `{block_size}`"
            );
            return Err(GifConstructionError::GraphicExtMissingData);
        };
    }

    let packed: u8 = u8
        .parse_next(input)
        .map_err(|_: EmptyError| GifConstructionError::GraphicExtMissingData)
        .inspect_err(|_| log::error!("Graphic control extension missing packed field!"))?;
    let disposal_method: u8 = (packed & 0b0011_1000) >> 3;
    let user_input_flag: bool = (packed & 0b0100_0000) == 0b0100_0000;
    let transparent_color_flag: bool = (packed & 0b1000_0000) == 0b1000_0000;

    let delay_time: u16 = le_u16.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Graphic control extension is missing delay time!");
        GifConstructionError::GraphicExtMissingData
    })?;

    let transparent_color_index: u8 = u8.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Graphic control extension is missing transparent color index!");
        GifConstructionError::GraphicExtMissingData
    })?;

    block_terminator.parse_next(input).inspect_err(|_| {
        log::error!("Graphic control extension is missing block terminator!");
    })?;

    Ok(GraphicControlExtension {
        disposal_method,
        user_input_flag,
        transparent_color_flag,
        delay_time,
        transparent_color_index,
    })
}

#[derive(Clone, Debug)]
pub struct CommentExtension {
    pub data: Vec<u8>,
}

/// Parses a Comment Extension block.
pub(super) fn comment_extension(
    input: &mut &[u8],
) -> Result<CommentExtension, GifConstructionError> {
    log::trace!("Parsing: comment extension.");

    // extension introducer
    helpers::extension_introducer.parse_next(input)?;

    // extension label
    helpers::extension_label(input, "Comment Extension", 0xFE)?;

    // keep reading subblock til we find the terminator
    let mut buf: Vec<u8> = Vec::new();
    while input[0] != 0x00 {
        data_sub_block(input, &mut buf)?;
    }

    block_terminator(input)?;

    Ok(CommentExtension { data: buf })
}

#[derive(Clone, Debug)]
pub struct PlainTextExtension {
    pub text_grid_left_position: u16,
    pub text_grid_top_position: u16,
    pub text_grid_width: u16,
    pub text_grid_height: u16,

    pub character_cell_width: u8,
    pub character_cell_height: u8,

    pub text_foreground_color_index: u8,
    pub text_background_color_index: u8,

    pub plain_text_data: Vec<u8>,
}

/// Parses a Plain Text Extension block.
pub(super) fn plain_text_extension(
    input: &mut &[u8],
) -> Result<PlainTextExtension, GifConstructionError> {
    log::trace!("Parsing: plain text extension.");

    // extension introducer
    helpers::extension_introducer.parse_next(input)?;

    // plain text label (0x01)
    helpers::extension_label(input, "Plain Text Extension", 0x01)?;

    // block size
    match helpers::block_size(input) {
        Err(e) => return Err(e),
        Ok(12) => (),
        Ok(other) => {
            log::error!(
                "Plain text extension had a wrong block size! \
                Expected `12`, got `{other}`."
            );
            return Err(GifConstructionError::ExtensionHasWeirdBlockSize {
                got: other,
                expected: 12_u8,
            });
        }
    };

    let text_grid_left_position: u16 = le_u16.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing text grid left position.");
        GifConstructionError::PlainTextExtMissingData
    })?;
    let text_grid_top_position: u16 = le_u16.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing text grid top position.");
        GifConstructionError::PlainTextExtMissingData
    })?;
    let text_grid_width: u16 = le_u16.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing text grid width.");
        GifConstructionError::PlainTextExtMissingData
    })?;
    let text_grid_height: u16 = le_u16.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing text grid height.");
        GifConstructionError::PlainTextExtMissingData
    })?;

    let character_cell_width: u8 = u8.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing character cell width.");
        GifConstructionError::PlainTextExtMissingData
    })?;
    let character_cell_height: u8 = u8.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing character cell height.");
        GifConstructionError::PlainTextExtMissingData
    })?;

    let text_foreground_color_index: u8 = u8.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing text foreground color index.");
        GifConstructionError::PlainTextExtMissingData
    })?;
    let text_background_color_index: u8 = u8.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Plain text extension missing text background color index.");
        GifConstructionError::PlainTextExtMissingData
    })?;

    // actually read the text data
    let mut plain_text_data: Vec<u8> = vec![];
    while let Some(b) = input.first()
        && *b != 0x00
    {
        data_sub_block(input, &mut plain_text_data)?;
    }

    // finally, eat the block terminator
    block_terminator.parse_next(input)?;

    Ok(PlainTextExtension {
        text_grid_left_position,
        text_grid_top_position,
        text_grid_width,
        text_grid_height,
        character_cell_width,
        character_cell_height,
        text_foreground_color_index,
        text_background_color_index,
        plain_text_data,
    })
}

#[derive(Clone, Debug)]
pub struct ApplicationExtension {
    pub application_identifier: [u8; 8],
    pub application_authentication_code: [u8; 3],
    pub application_data: Vec<u8>,
}

/// Parses an Application Extension block.
pub(super) fn application_extension(
    input: &mut &[u8],
) -> Result<ApplicationExtension, GifConstructionError> {
    log::trace!("Parsing: application extension.");

    // extension introducer
    helpers::extension_introducer.parse_next(input)?;

    // extension label
    helpers::extension_label(input, "Application Extension", 0xFF)?;

    // block size
    {
        let block_size: u8 = helpers::block_size.parse_next(input)?;
        if block_size != 11 {
            log::error!(
                "App extension had incorrect block size!\
            expected: `11`, got: `{block_size}`"
            );
            return Err(GifConstructionError::AppExtMissingData);
        }
    }

    // application ident
    let app_ident: [u8; 8] = take(8_usize)
        .parse_next(input)
        .map_err(|_: EmptyError| GifConstructionError::AppExtMissingData)
        .and_then(|slice: &[u8]| -> Result<[u8; 8], GifConstructionError> {
            TryFrom::try_from(slice).map_err(|_| GifConstructionError::AppExtMissingData)
        })
        .inspect_err(|_| log::error!("App extension missing application identifier!"))?;

    // application auth code
    let app_auth_code: [u8; 3] = take(3_usize)
        .parse_next(input)
        .map_err(|_: EmptyError| GifConstructionError::AppExtMissingData)
        .and_then(|slice: &[u8]| -> Result<[u8; 3], GifConstructionError> {
            TryFrom::try_from(slice).map_err(|_| GifConstructionError::AppExtMissingData)
        })
        .inspect_err(|_| log::error!("App extension missing application auth code!"))?;

    // non-compliant writers can omit the subblocks and just use a "raw" byte
    // stream.
    //
    // soo, prepare for that, too
    if app_ident == *b"XMP Data" && app_auth_code == *b"XMP" {
        const MAGIC_TRAILER_NO_BLOCK_TERMINATOR: [u8; 257] = {
            let mut arr: [u8; 257] = [0x00; 257];

            arr[0] = 0x01;

            let mut idx: usize = 1;
            let mut k: u8 = 0xFF;
            loop {
                arr[idx] = k;
                idx += 1;
                if k == 0x00 {
                    break;
                }
                k -= 1;
            }

            arr
        };

        let remaining: &[u8] = input;
        if let Some(trailer_start) = remaining
            .windows(MAGIC_TRAILER_NO_BLOCK_TERMINATOR.len())
            .position(|window| window == MAGIC_TRAILER_NO_BLOCK_TERMINATOR)
        {
            let data_end: usize = trailer_start + MAGIC_TRAILER_NO_BLOCK_TERMINATOR.len();
            let application_data: Vec<u8> = remaining[..data_end].to_vec();
            *input = &remaining[data_end..];

            // eat gif block terminator after the raw XMP payload
            block_terminator(input)?;

            return Ok(ApplicationExtension {
                application_identifier: app_ident,
                application_authentication_code: app_auth_code,
                application_data,
            });
        }
    }

    // read data sub-blocks until we reach this block's terminator
    let mut buf: Vec<u8> = Vec::new();
    while input[0] != 0x00 {
        data_sub_block(input, &mut buf)?;
    }

    // end with block terminator
    block_terminator(input)?;

    Ok(ApplicationExtension {
        application_identifier: app_ident,
        application_authentication_code: app_auth_code,
        application_data: buf,
    })
}

/// Parses the Trailer block.
pub(super) fn trailer(input: &mut &[u8]) -> Result<(), GifConstructionError> {
    let value: u8 = u8
        .parse_next(input)
        .map_err(|_: EmptyError| GifConstructionError::TrailerMissing)
        .inspect_err(|_| log::error!("Trailer block is completely missing!"))?;

    if value != 0x3b {
        log::error!(
            "Found an unexpected trailer value. \
            Found `0x{value:x}`, but expected `0x3b`. \
            This is an implementation problem, so please report this message it as a bug!"
        );
        return Err(GifConstructionError::TrailerMissing);
    }

    Ok(())
}

pub(super) mod helpers {
    use winnow::{Parser, binary::u8, error::EmptyError};

    use super::super::error::GifConstructionError;

    /// Parses out an Extension Introducer.
    pub fn extension_introducer(input: &mut &[u8]) -> Result<(), GifConstructionError> {
        log::trace!("Parsing: extension introducer.");

        let extension_introducer: u8 = u8
            .parse_next(input)
            .map_err(|_: EmptyError| GifConstructionError::NotEnoughBytes)
            .inspect_err(|_| log::error!("Extension missing introducer!"))?;

        if extension_introducer != 0x21 {
            log::error!(
                "Extension had incorrect introducer! \
            expected: `0x21`, got: `0x{extension_introducer:x}`. \
            This is a bug. Please report it on GitHub."
            );
            return Err(GifConstructionError::NotEnoughBytes);
        }

        Ok(())
    }

    /// Parses out a label for the given extension type.
    pub fn extension_label(
        input: &mut &[u8],
        extension_type: &'static str,
        expected_label_value: u8,
    ) -> Result<(), GifConstructionError> {
        log::trace!("Parsing: extension label.");

        let extension_label: u8 = u8
            .parse_next(input)
            .map_err(|_: EmptyError| GifConstructionError::NotEnoughBytes)
            .inspect_err(|_| log::error!("{extension_type} missing label!"))?;

        if extension_label != expected_label_value {
            log::error!(
                "{extension_type} had incorrect label! \
            expected: `0x{expected_label_value:x}`, got: `0x{extension_label:x}`"
            );
            return Err(GifConstructionError::UnknownExtensionFound {
                label: extension_label,
            });
        }

        Ok(())
    }

    /// Parses out the block size byte for an extension block.
    pub fn block_size(input: &mut &[u8]) -> Result<u8, GifConstructionError> {
        log::trace!("Parsing: block size.");

        u8.parse_next(input).map_err(|_: EmptyError| {
            log::error!("Graphic control extension missing block size!");
            GifConstructionError::NotEnoughBytes
        })
    }
}
