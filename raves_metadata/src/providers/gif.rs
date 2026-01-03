//! Provider implementation for GIF, the Graphics Interchange Format.

use parking_lot::RwLock;
use std::sync::Arc;
use winnow::{Parser, binary::u8, error::EmptyError, token::take};

use crate::{MaybeParsedXmp, MetadataProvider, MetadataProviderRaw};

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

    fn new(input: &impl AsRef<[u8]>) -> Result<Self, GifConstructionError> {
        let input: &mut &[u8] = &mut input.as_ref();

        // header: check for gif's magic number
        magic_number.parse_next(input)?;

        // header: check for gif version
        let ver: [u8; 3] = gif_version.parse_next(input)?;

        // ignore logical screen desc (required to be after header)
        let maybe_gct_size: Option<u32> = logical_screen_descriptor.parse_next(input)?;

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

///
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
    NoLsd,

    /// The LSD said that there should be a GCT, but it was not present.
    ///
    /// Unable to continue parsing for this reason.
    NoGct,

    /// Found an extension block, but it ended earlier than it should have!
    ///
    /// If other programs work well with this file, please report this!
    ExtensionStoppedAbruptly(u8),

    /// The GIF 87a (1987 rev. a) specification does not support extension
    /// blocks, but one was present anyway.
    ExtensionFoundInGif87,
}

impl core::error::Error for GifConstructionError {}

impl core::fmt::Display for GifConstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let t = todo!();
    }
}

/// the magic number should be `b"GIF"`.
fn magic_number(input: &mut &[u8]) -> Result<(), GifConstructionError> {
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

/// grab GIF version, which is required
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

/// takes 7 bytes for the LSD, checking if the GCT comes next
fn logical_screen_descriptor(input: &mut &[u8]) -> Result<Option<u32>, GifConstructionError> {
    let bytes = take(7_usize).parse_next(input).map_err(|_: EmptyError| {
        log::error!("GIF did not have enough bytes for logical screen descriptor.");
        GifConstructionError::NoLsd
    })?;

    let Some(flags) = bytes.get(4) else {
        return Err(GifConstructionError::NoLsd);
    };

    // check if the GCT (Global Color Table) is after the LSD
    let has_gct_next: bool = (flags & 0b1000_0000) != 0;

    // if so, grab + return the GCT's size.
    //
    // otherwise, just return `None`
    if has_gct_next {
        Ok(Some(2_u32.saturating_pow(
            ((flags & 0b0000_0111).saturating_add(1)) as u32,
        )))
    } else {
        Ok(None)
    }
}

/// skip the gct
fn gct(gct_size: u32, input: &mut &[u8]) -> Result<(), GifConstructionError> {
    _ = take(gct_size).parse_next(input).map_err(|_: EmptyError| {
        log::error!("GCT was not found, but the LSD said it would be present!");
        GifConstructionError::NoGct
    })?;

    Ok(())
}

/// parses an application extension to see if it has XMP data.
fn application_extension(
    input: &mut &[u8],
) -> Result<Option<MaybeParsedXmp>, GifConstructionError> {
    // we've already parsed the first two bytes.
    //
    // let's grab the other parts...

    //

    todo!()
}
