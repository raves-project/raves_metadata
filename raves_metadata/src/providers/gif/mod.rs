//! Provider implementation for GIF, the Graphics Interchange Format.

mod blocks;
mod error;

use parking_lot::RwLock;
use std::sync::Arc;
use winnow::{Parser, binary::le_u16, binary::u8, error::EmptyError, token::take};

use crate::{MaybeParsedXmp, MetadataProvider, MetadataProviderRaw};
use error::GifConstructionError;

/// A parsed GIF (Graphics Interchange Format) file.
#[derive(Clone, Debug)]
pub struct Gif {
    /// The version number of the GIF.
    ///
    /// This should be `87a` or `89a`, but can be something else.
    version: [char; 3],

    /// Stored XMP.
    ///
    /// May not be parsed yet, but the bytes are in there regardless, if the
    /// GIF blob had any XMP to provide.
    xmp: Arc<RwLock<Option<MaybeParsedXmp>>>,
}

impl Gif {
    /// Returns the "version" string of this GIF.
    ///
    /// Note that it can _technically_ be any value, though GIF only intends
    /// the value to be `87a` or `89a`.
    pub fn version(&self) -> &[char; 3] {
        &self.version
    }
}

impl MetadataProvider for Gif {
    type ConstructionError = GifConstructionError;

    fn magic_number(input: &[u8]) -> bool {
        // there must be three bytes in the input
        let Some(slice) = input.get(0..=2) else {
            return false;
        };

        // then, those bytes must equal `GIF`
        if slice != b"GIF" {
            return false;
        }

        true
    }

    fn new(input: &impl AsRef<[u8]>) -> Result<Self, GifConstructionError> {
        let input: &mut &[u8] = &mut input.as_ref();

        // parse header
        let header = header.parse_next(input)?;

        // ignore logical screen desc (required to be after header)
        let logical_screen_descriptor = logical_screen_descriptor.parse_next(input)?;

        // parse (ignore) the gct, if it's there
        if let Some(gct_size) = maybe_gct_size {
            gct(gct_size, input)?;
        }

        // now, skip through all the images and other nonsense to find metadata
        let mut xmp: Option<MaybeParsedXmp> = None;
        while !input.is_empty() {
            // check first byte of each block
            let Ok(first_byte): Result<u8, EmptyError> = u8.parse_next(input) else {
                log::trace!("Outta data for extension! (no first byte)");
                break;
            };

            // based on that byte, run different parser...
            match first_byte {
                0x21 => {
                    // the next byte is the extension ident byte
                    let Ok(second_byte): Result<u8, EmptyError> = u8.parse_next(input) else {
                        log::error!("Outta data for extension! (no second byte)");
                        return Err(GifConstructionError::ExtensionStoppedAbruptly(first_byte));
                    };

                    // check second byte to check the kind of extension
                    match second_byte {
                        // application extension (can contain XMP!)
                        0xFF => {
                            if ver != *b"89a" {
                                log::error!(
                                    "An extension was found, but the version is `87a`, which doesn't support them."
                                );
                                return Err(GifConstructionError::ExtensionFoundInGif87);
                            }

                            let maybe_xmp = application_extension(input)?;
                            if let Some(found_xmp) = maybe_xmp {
                                xmp = Some(found_xmp);
                            }
                            break;
                        }

                        _ => {
                            let t = todo!();
                        }
                    }
                }

                other => {
                    log::error!("Unknown block identifier: `0x{other:x}`")
                }
            }
        }

        Ok(Gif {
            version: ver.map(char::from),
            xmp: Arc::new(RwLock::new(xmp)),
        })
    }
}

impl MetadataProviderRaw for Gif {
    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::clone(&self.xmp)
    }
}

/// the magic number should be `b"GIF"`.
fn signature(input: &mut &[u8]) -> Result<(), GifConstructionError> {
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
fn data_sub_block(input: &mut &[u8], output: &mut Vec<u8>) -> Result<(), GifConstructionError> {
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
fn block_terminator(input: &mut &[u8]) -> Result<(), GifConstructionError> {
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

struct GifHeader {
    version: [u8; 3],
}

/// Parses the GIF's Header.
fn header(input: &mut &[u8]) -> Result<GifHeader, GifConstructionError> {
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
    _ = signature(input)?;

    // grab version
    let version: [u8; 3] = gif_version(input)?;

    // return a header
    Ok(GifHeader { version })
}

struct LogicalScreenDescriptor {
    logical_screen_width: u16,

    logical_screen_height: u16,

    global_color_table_flag: bool,
    color_resolution: u8, // range: 1..=4
    sort_flag: bool,
    size_of_global_color_table: u8,

    background_color_index: u8,
    pixel_aspect_ratio: Option<u8>,
}

/// Parses the Logical Screen Descriptor.
fn logical_screen_descriptor(
    input: &mut &[u8],
) -> Result<LogicalScreenDescriptor, GifConstructionError> {
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
    let global_color_table_flag: bool = (packed & 0b0000_0001) == 0b0000_0001;
    let color_resolution: u8 = ((packed & 0b0000_1110) >> 1) + 1; // TODO: should this add +1?
    let sort_flag: bool = (packed & 0b0001_0000) == 0b0001_0000;
    let size_of_global_color_table: u8 = (packed & 0b1110_0000) >> 5;

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

struct GlobalColorTable {
    rgb_triplets: Vec<(u8, u8, u8)>,
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
fn global_color_table(
    size_of_global_color_table_plus_one: u8,
    todo_SHOULD_THE_ABOVE_HAVE_PLUS_ONE_OR_DOES_THAT_OFFSET_TO_256_I_THINK_IT_DOES: (),
    input: &mut &[u8],
) -> Result<GlobalColorTable, GifConstructionError> {
    let triplet_ct: u8 = 2_u8.pow(size_of_global_color_table_plus_one as u32);
    let mut v: Vec<(u8, u8, u8)> = Vec::with_capacity(triplet_ct as usize);

    // define color getter (helper closure)
    let mut get_color = |color_name: &'static str, color: GctMissingColor, triplet_num: u8| {
        u8.parse_next(input)
            .inspect_err(|_e: &EmptyError| {
                log::warn!(
                    "Global color table missing {color_name} at triplet {triplet_num}/{triplet_ct}!"
                )
            })
            .map_err(|_e: EmptyError| GifConstructionError::NoGct {
                expected_triplet_ct: triplet_ct,
                errant_triplet: triplet_num,
                missing_color: color,
            })
    };

    // find and set each triplet
    for triplet_num in 0..=triplet_ct {
        // grab each color in triplet
        let (red, green, blue): (u8, u8, u8) = (
            get_color("red", GctMissingColor::Red, triplet_num)?,
            get_color("green", GctMissingColor::Green, triplet_num)?,
            get_color("blue", GctMissingColor::Blue, triplet_num)?,
        );

        // set in the list
        debug_assert!(triplet_num as usize <= v.len());
        match v.get_mut(triplet_num as usize) {
            Some(s) => *s = (red, green, blue),
            None => log::error!(
                "Global color table: implementation error. \
                Index out of bounds. Please report this!"
            ),
        }
    }

    Ok(GlobalColorTable { rgb_triplets: v })
}

struct ImageDescriptor {
    image_left_position: u16,

    image_top_position: u16,

    image_width: u16,

    image_height: u16,

    local_color_table_flag: bool,
    interlace_flag: bool,
    sort_flag: bool,

    size_of_local_color_table: u8,
}

/// Parses the Image Descriptor block.
fn image_descriptor(input: &mut &[u8]) -> Result<ImageDescriptor, GifConstructionError> {
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

    let local_color_table_flag: bool = packed & 0b0000_0001 == 0b0000_0001;
    let interlace_flag: bool = packed & 0b0000_0010 == 0b0000_0010;
    let sort_flag: bool = packed & 0b0000_0100 == 0b0000_0100;
    let _reserved = ();
    let size_of_local_color_table: u8 = (packed & 0b1110_0000) >> 5;

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

type LocalColorTable = GlobalColorTable;

/// Parses the Local Color Table block.
fn local_color_table(size: u8, input: &mut &[u8]) -> Result<LocalColorTable, GifConstructionError> {
    global_color_table(size, (), input)
}

/// Parses table-based image data.
fn table_based_image_data(input: &mut &[u8]) -> Result<(), GifConstructionError> {
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

struct GraphicControlExtension {
    disposal_method: u8,
    user_input_flag: bool,
    transparent_color_flag: bool,

    delay_time: u16,

    transparent_color_index: u8,
}

fn graphic_control_extension(
    input: &mut &[u8],
) -> Result<GraphicControlExtension, GifConstructionError> {
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

struct CommentExtension {
    data: Vec<u8>,
}

/// Parses a Comment Extension block.
fn comment_extension(input: &mut &[u8]) -> Result<CommentExtension, GifConstructionError> {
    // extension introducer
    helpers::extension_introducer.parse_next(input)?;

    // extension label
    helpers::extension_label(input, "Comment Extension", 0xFE);

    // keep reading subblock til we find the terminator
    let mut buf: Vec<u8> = Vec::new();
    while input[0] != 0x00 {
        data_sub_block(input, &mut buf)?;
    }

    block_terminator(input)?;

    Ok(CommentExtension { data: buf })
}

struct PlainTextExtension {
    text_grid_left_position: u16,
    text_grid_top_position: u16,
    text_grid_width: u16,
    text_grid_height: u16,

    character_cell_width: u8,
    character_cell_height: u8,

    text_foreground_color_index: u8,
    text_background_color_index: u8,

    plain_text_data: Vec<u8>,
}

/// Parses a Plain Text Extension block.
fn plain_text_extension(input: &mut &[u8]) -> Result<PlainTextExtension, GifConstructionError> {
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
            return Err(GifConstructionError::ExtensionHasWeirdBlockSize(other));
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

struct ApplicationExtension {
    application_identifier: [u8; 8],
    application_authentication_code: [u8; 3],
    application_data: Vec<u8>,
}

/// Parses an Application Extension block.
fn application_extension(input: &mut &[u8]) -> Result<ApplicationExtension, GifConstructionError> {
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

    // read data sub-blocks until we reach this block's terminator
    let mut buf: Vec<u8> = Vec::new();
    while input[0] != 0x00 {
        data_sub_block(input, &mut buf)?;
    }

    // end with block terminator
    block_terminator(input)?;

    return Ok(ApplicationExtension {
        application_identifier: app_ident,
        application_authentication_code: app_auth_code,
        application_data: buf,
    });
}

/// Parses the Trailer block.
fn trailer(input: &mut &[u8]) -> Result<(), GifConstructionError> {
    let value: u8 = u8
        .parse_next(input)
        .map_err(|_: EmptyError| GifConstructionError::TrailerMissing)
        .inspect_err(|_| log::error!("Trailer block is completely missing!"))?;

    if value != 0x3b {
        return Err(GifConstructionError::TrailerIncorrectValue(value));
    }

    Ok(())
}

mod helpers {
    use winnow::{Parser, binary::u8, error::EmptyError};

    use super::error::GifConstructionError;

    /// Parses out an Extension Introducer.
    pub fn extension_introducer(input: &mut &[u8]) -> Result<(), GifConstructionError> {
        let extension_introducer: u8 = u8
            .parse_next(input)
            .map_err(|_: EmptyError| GifConstructionError::ExtensionMissingIntroducer)
            .inspect_err(|_| log::error!("Extension missing introducer!"))?;

        if extension_introducer != 0x21 {
            log::error!(
                "Extension had incorrect introducer!\
            expected: `0x21`, got: `0x{extension_introducer:x}`"
            );
            return Err(GifConstructionError::ExtensionMissingLabel);
        }

        Ok(())
    }

    /// Parses out a label for the given extension type.
    pub fn extension_label(
        input: &mut &[u8],
        extension_type: &'static str,
        expected_label_value: u8,
    ) -> Result<(), GifConstructionError> {
        let extension_label: u8 = u8
            .parse_next(input)
            .map_err(|_: EmptyError| GifConstructionError::ExtensionMissingLabel)
            .inspect_err(|_| log::error!("{extension_type} missing label!"))?;

        if extension_label != expected_label_value {
            log::error!(
                "{extension_type} had incorrect label!\
            expected: `0x{expected_label_value:x}`, got: `0x{extension_label:x}`"
            );
            return Err(GifConstructionError::ExtensionMissingLabel);
        }

        Ok(())
    }

    /// Parses out the block size byte for an extension block.
    pub fn block_size(input: &mut &[u8]) -> Result<u8, GifConstructionError> {
        u8.parse_next(input).map_err(|_: EmptyError| {
            log::error!("Graphic control extension missing block size!");
            GifConstructionError::ExtensionStoppedAbruptly(1_u8)
        })
    }
}
