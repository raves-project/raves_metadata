//! Provider implementation for GIF, the Graphics Interchange Format.

pub mod block;
pub mod error;

use parking_lot::RwLock;
use std::sync::Arc;
use winnow::Parser;

use crate::{MaybeParsedXmp, MetadataProvider, MetadataProviderRaw};
use block::{
    ApplicationExtension, CommentExtension, GifHeader, GlobalColorTable, GraphicControlExtension,
    ImageDescriptor, LocalColorTable, LogicalScreenDescriptor, PlainTextExtension,
};
use error::GifConstructionError;

/// A parsed GIF (Graphics Interchange Format) file.
#[derive(Clone, Debug)]
pub struct Gif {
    /// The GIF's header.
    header: GifHeader,

    /// This file's logical screen descriptor block.
    logical_screen_descriptor: LogicalScreenDescriptor,

    /// The file's global color table block, if present.
    global_color_table: Option<GlobalColorTable>,

    /// A number of repeatable blocks.
    ///
    /// This may be empty, but likely contains at least the image data of the
    /// GIF. It may also contain things like metadata and other information.
    ///
    /// **NOTE**: If XMP was found in the file, it was removed from this list
    /// and provided through the typical API. **Please refrain from iterating
    /// over this list to find XMP data -- you won't find any.**
    repeatable_blocks: Vec<RepeatableBlock>,

    /// Stored XMP.
    ///
    /// May not be parsed yet, but the bytes are in there regardless, if the
    /// GIF blob had any XMP to provide.
    xmp: Arc<RwLock<Option<MaybeParsedXmp>>>,
}

/// Any block in the GIF file after the header, logical screen descriptor, and
/// (optional) global color table.
///
/// Any of these blocks can be repeated multiple times until we hit the trailer
/// (end) block.
#[derive(Clone, Debug)]
pub enum RepeatableBlock {
    /// The block(s) that come after a graphic control extension was found, if
    /// present, or are simply present themselves.
    Graphic {
        /// The graphic control extension itself, if present.
        graphic_control_extension: Option<GraphicControlExtension>,

        /// Either an image data block or a plain text extension.
        suffix: RepeatableGraphicBlock,
    },

    /// An application extension block.
    ApplicationExtension(ApplicationExtension),

    /// A comment extension block.
    CommentExtension(CommentExtension),
}

/// Either the image data or a plain text extension.
///
/// Comes after the graphic control extension, or on its own, in the "repeating
/// blocks" part of the file.
#[derive(Clone, Debug)]
pub enum RepeatableGraphicBlock {
    /// Image data.
    Image {
        /// Image descriptor.
        image_descriptor: ImageDescriptor,

        /// The local color table, if present.
        local_color_table: Option<LocalColorTable>,

        // TODO: hold input buf bounds? or should we actually store data here?
        //
        // (gifs tend to be small, at least for the heap, so idk...)
        /// Image data.
        image_data: (),
    },

    /// A plain text extension.
    PlainTextExtension(
        /// The inner [`PlainTextExtension`].
        ///
        /// Stored in-line.
        PlainTextExtension,
    ),
}

impl RepeatableGraphicBlock {
    fn image(input: &mut &[u8]) -> Result<Self, GifConstructionError> {
        // parse the image descriptor
        let image_descriptor = block::image_descriptor(input)?;

        // if the image descriptor notes that there should be a
        // local color table, parse that
        let local_color_table = if image_descriptor.local_color_table_flag {
            Some(block::local_color_table(
                image_descriptor.size_of_local_color_table,
                input,
            )?)
        } else {
            None
        };

        // then, skip over the image data
        block::table_based_image_data(input)?;

        Ok(Self::Image {
            image_descriptor,
            local_color_table,
            image_data: (),
        })
    }

    fn plain_text(input: &mut &[u8]) -> Result<Self, GifConstructionError> {
        Ok(Self::PlainTextExtension(block::plain_text_extension(
            input,
        )?))
    }
}

impl Gif {
    /// Returns the "version" string of this GIF.
    ///
    /// Note that it can _technically_ be any value, though GIF only intends
    /// the value to be `87a` or `89a`.
    pub fn version(&self) -> [char; 3] {
        self.header.version.map(From::from)
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
        let header: GifHeader = block::header.parse_next(input)?;

        // parse out the logical screen desc (required to be after header)
        let logical_screen_descriptor: LogicalScreenDescriptor =
            block::logical_screen_descriptor.parse_next(input)?;

        // parse the gct, if present
        let global_color_table: Option<GlobalColorTable> =
            if logical_screen_descriptor.global_color_table_flag {
                Some(block::global_color_table(
                    logical_screen_descriptor.size_of_global_color_table,
                    input,
                )?)
            } else {
                None
            };

        // parse all the "repeatable blocks"
        let mut repeatable_blocks: Vec<RepeatableBlock> = Vec::new();
        loop {
            // grab a new byte.
            //
            // if we're at the end of the file, that's very bad -- we don't
            // have a trailer (end) block!
            //
            // error and warn the user!
            let Some(first_byte) = input.first().copied() else {
                log::error!("The GIF ended suddenly, doing so without a trailer (end) block!");
                return Err(GifConstructionError::NotEnoughBytes);
            };

            // if we've found the trailer, stop parsing!
            if first_byte == 0x3b {
                log::trace!("Found trailer (end) block for GIF file. Stopping.");
                block::trailer(input)?;
                break;
            }

            const EXTENSION_INTRODUCER: u8 = 0x21;
            const IMAGE_DESCRIPTOR_INTRODUCER: u8 = 0x2c;

            const GRAPHIC_CONTROL_EXTENSION_LABEL: u8 = 0xf9;
            const PLAIN_TEXT_EXTENSION_LABEL: u8 = 0x01;
            const APPLICATION_EXTENSION_LABEL: u8 = 0xff;
            const COMMENT_EXTENSION_LABEL: u8 = 0xfe;

            // alright, let's grab the next available repeatable block...
            let repeatable_block: RepeatableBlock;
            match first_byte {
                //
                //
                //
                //
                // image descriptor
                b if b == IMAGE_DESCRIPTOR_INTRODUCER => {
                    repeatable_block = RepeatableBlock::Graphic {
                        graphic_control_extension: None,
                        suffix: RepeatableGraphicBlock::image(input)?,
                    }
                }

                //
                //
                //
                //
                // any extension
                b if b == EXTENSION_INTRODUCER => {
                    // GIF 87a doesn't support extensions. error if such a GIF tries to use em
                    if header.version == *b"87a" {
                        return Err(GifConstructionError::ExtensionFoundInGif87);
                    }

                    // grab the second byte to see which extension it is!
                    let Some(second_byte) = input.get(1).copied() else {
                        log::error!("GIF should have another byte for extension/image data.");
                        return Err(GifConstructionError::NotEnoughBytes);
                    };

                    match second_byte {
                        c if c == GRAPHIC_CONTROL_EXTENSION_LABEL => {
                            // parse the graphic control extension
                            let graphic_control_extension =
                                Some(block::graphic_control_extension(input)?);

                            // grab the next byte to see if it's an image desc. or a
                            // plain text ext.
                            let Some(next_byte) = input.first().copied() else {
                                log::error!(
                                    "GIF should have another byte for extension/image data."
                                );
                                return Err(GifConstructionError::NotEnoughBytes);
                            };

                            repeatable_block = RepeatableBlock::Graphic {
                                graphic_control_extension,
                                suffix: match next_byte {
                                    b if b == IMAGE_DESCRIPTOR_INTRODUCER => {
                                        RepeatableGraphicBlock::image(input)?
                                    }

                                    b if b == PLAIN_TEXT_EXTENSION_LABEL => {
                                        RepeatableGraphicBlock::plain_text(input)?
                                    }

                                    other => {
                                        log::error!(
                                            "After parsing the graphic control extension, \
                                            found an unexpected block type: `0x{other:x}`"
                                        );
                                        return Err(GifConstructionError::UnknownBlockFound {
                                            byte: other,
                                        });
                                    }
                                },
                            };
                        }

                        c if c == PLAIN_TEXT_EXTENSION_LABEL => {
                            repeatable_block = RepeatableBlock::Graphic {
                                graphic_control_extension: None,
                                suffix: RepeatableGraphicBlock::PlainTextExtension(
                                    block::plain_text_extension(input)?,
                                ),
                            };
                        }

                        c if c == APPLICATION_EXTENSION_LABEL => {
                            repeatable_block = RepeatableBlock::ApplicationExtension(
                                block::application_extension(input)?,
                            );
                        }

                        c if c == COMMENT_EXTENSION_LABEL => {
                            repeatable_block =
                                RepeatableBlock::CommentExtension(block::comment_extension(input)?);
                        }

                        other => {
                            log::error!(
                                "In repeatable block section, found an unexpected \
                                extension type: `0x{other:x}`"
                            );
                            return Err(GifConstructionError::UnknownExtensionFound {
                                label: other,
                            });
                        }
                    }
                }

                //
                //
                //
                //
                // error! unexpected block found...
                other => {
                    log::error!(
                        "In repeatable block section, found an unexpected block \
                        type: `0x{other:x}`"
                    );
                    return Err(GifConstructionError::UnknownBlockFound { byte: other });
                }
            }

            repeatable_blocks.push(repeatable_block);
        }

        // parse out any potential XMP from what we've found.
        //
        // this parsing is kinda weird:
        //
        // 1. get app extensions
        // 2. find a b"XMP Data" app id
        // 3. look for insane "magic trailer" (who came up with this, maaaan)
        // 4. parse out the XMP blob
        let mut xmp = None;
        for block_idx in 0..repeatable_blocks.len() {
            // grab block from vec
            let block: &RepeatableBlock = &repeatable_blocks[block_idx];

            let RepeatableBlock::ApplicationExtension(ext) = block else {
                continue;
            };

            if ext.application_identifier == *b"XMP Data" {
                log::trace!("Found XMP data application identifier!");
            } else {
                continue;
            }

            if ext.application_authentication_code == *b"XMP" {
                log::trace!("Found XMP data authentication code!");
            } else {
                continue;
            }

            const MAGIC_TRAILER: [u8; 258] = {
                let mut arr: [u8; 258] = [0x00; 258];

                // set the first byte (0x01)
                arr[0] = 0x01;

                // state
                let mut idx: usize = 1;
                let mut k: u8 = 0xFF;

                // count down from 0xFF to 0x00.
                //
                // ("magic trailer" takes advantage of GIF spec's blocks)
                while k != 0x00 {
                    arr[idx] = k;
                    k -= 1;
                    idx += 1;
                }

                // note: the last two bytes are already 0x00.
                // return the prep'd array
                arr
            };

            if ext
                .application_data
                .ends_with(&[0x03, 0x02, 0x01, 0x00, 0x00])
            {
                log::trace!("Found likely XMP packet! Confirming...");
            } else {
                continue;
            }

            if ext.application_data.ends_with(&MAGIC_TRAILER) {
                log::trace!("XMP packet found!");
                xmp = Some(MaybeParsedXmp::Raw(
                    ext.application_data[..ext.application_data.len() - MAGIC_TRAILER.len()]
                        .to_vec(),
                ));
                repeatable_blocks.remove(block_idx);
            } else {
                continue;
            }
        }

        Ok(Gif {
            header,
            logical_screen_descriptor,
            global_color_table,
            repeatable_blocks: vec![],
            xmp: Arc::new(RwLock::new(xmp)),
        })
    }
}

impl MetadataProviderRaw for Gif {
    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::clone(&self.xmp)
    }
}

#[cfg(test)]
mod tests {
    use crate::{MetadataProvider, util::logger};

    #[test]
    fn sample_gif() {
        logger();

        const GIF_FROM_GIFLIB: &[u8] = &[
            0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x0A, 0x00, 0x0A, 0x00, 0x91, 0x00, 0x00, 0xFF,
            0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x21, 0xF9, 0x04,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x0A, 0x00,
            0x00, 0x02, 0x16, 0x8C, 0x2D, 0x99, 0x87, 0x2A, 0x1C, 0xDC, 0x33, 0xA0, 0x02, 0x75,
            0xEC, 0x95, 0xFA, 0xA8, 0xDE, 0x60, 0x8C, 0x04, 0x91, 0x4C, 0x01, 0x00, 0x3B,
        ];

        let gif = super::Gif::new(&GIF_FROM_GIFLIB).unwrap();
        println!("{gif:?}");
    }
}
