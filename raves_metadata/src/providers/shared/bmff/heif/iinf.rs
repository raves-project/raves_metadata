//! Related to parsing the `meta` box, which provides metadata in HEIC-like
//! files.
//!
//! Unfortunately, the metadata is often intermingled with the primary image
//! data (as if the XMP or Exif could last forever), so we have to do more
//! parsing once we have this data.
//!
//! Nonetheless, it's required, and we encapsulate it all here.

use winnow::{
    ModalResult, Parser,
    binary::{be_u16, be_u32, u8},
    combinator::fail,
    error::ContextError,
    token::take,
};

use crate::providers::shared::{
    bmff::{BoxHeader, BoxType},
    desc,
};

#[derive(Clone)]
pub struct FullBox {
    pub extends: BoxHeader,

    pub version: u8,
    pub _flags: [u8; 3],
}

impl FullBox {
    /// Parses out a new full box.
    pub fn new(input: &mut &[u8]) -> ModalResult<Self, ContextError> {
        let extends: BoxHeader = BoxHeader::new(input).inspect_err(|e| {
            log::warn!("Failed to parse out extended `class Box` for `class FullBox`. err: {e}")
        })?;

        let version: u8 = u8
            .context(desc("version"))
            .parse_next(input)
            .inspect_err(|e| log::error!("Failed to grab full box's version! err: {e}"))?;

        let Ok(flags) = take(3_usize)
            .context(desc("flags"))
            .parse_next(input)
            .inspect_err(|e| log::error!("Couldn't get full box's flags! err: {e}"))?
            .try_into()
        else {
            unreachable!("already checked that we have three elements in slice");
        };

        Ok(FullBox {
            extends,
            version,
            _flags: flags,
        })
    }
}

pub struct ItemInfoBox {
    pub _extends: FullBox,

    pub _entry_count: u32,
    pub item_infos: Vec<ItemInfoEntry>,
}

impl ItemInfoBox {
    pub fn new(input: &mut &[u8]) -> ModalResult<Self, ContextError> {
        // grab "full box", which this box extends
        let extends: FullBox = FullBox::new(input)?;

        // depending on the version number, grab entry count as u16 or u32
        let entry_count: u32 = match extends.version {
            0 => be_u16
                .context(desc("entry count (u16)"))
                .parse_next(input)? as u32,
            _ => be_u32
                .context(desc("entry count (u32)"))
                .parse_next(input)?,
        };

        // now, parse each item info contained
        let mut item_infos: Vec<ItemInfoEntry> = Vec::with_capacity(entry_count as usize);
        for entry_num in 0..(entry_count as usize) {
            item_infos.push(ItemInfoEntry::new.parse_next(input).inspect_err(|e| {
                log::error!("Failed to parse HEIF `meta` entry `#{entry_num}`. err: {e}")
            })?);
        }

        // return self
        Ok(Self {
            _extends: extends,
            _entry_count: entry_count,
            item_infos,
        })
    }
}

/// An entry of "item info". (from MPEG-21)
///
/// Used to describe large blobs of data, like images or metadata.
#[derive(Clone)]
pub struct ItemInfoEntry {
    /// This class extends `FullBox`, meaning we gotta parse a `FullBox` out
    /// before we get this class parsed out.
    pub _extends: FullBox,

    pub _item_protection_index: u16,
    pub _item_name: String,

    /// Version-specific info.
    pub version_specific: ItemInfoEntryVersioned,
}

impl ItemInfoEntry {
    /// Parses one `ItemInfoEntry`.
    pub fn new(input: &mut &[u8]) -> ModalResult<Self, ContextError> {
        // parse out the `FullBox` class that this one extends
        let extends: FullBox = FullBox::new(input)?;

        // ensure name is right
        if extends.extends.box_type != BoxType::Id(*b"infe") {
            log::error!("`{:?}` was not an `infe` box!", extends.extends.box_type);
            fail.context(desc("not an `infe` box!")).parse_next(input)?;
        }

        // alright, now do different things depending on version...
        log::trace!("ItemInfoBox is version `{}`.", extends.version);
        let ret: Self = match extends.version {
            0 => {
                // item id
                let item_id = item_info_entry::item_id_u16.parse_next(input)?;
                // item_protection_index
                let item_protection_index: u16 =
                    item_info_entry::item_protection_index.parse_next(input)?;
                // item_name
                let item_name: &str = item_info_entry::item_name.parse_next(input)?;
                // content_type
                let content_type: String = item_info_entry::content_type.parse_next(input)?;
                // content_encoding (might be just '\0')
                let content_encoding: Option<String> =
                    item_info_entry::content_encoding.parse_next(input)?;

                Self {
                    _extends: extends,
                    _item_protection_index: item_protection_index,
                    _item_name: item_name.to_string(),
                    version_specific: ItemInfoEntryVersioned::V0 {
                        item_id,
                        _content_type: content_type,
                        _content_encoding: content_encoding,
                    },
                }
            }

            1 => {
                // item id
                let item_id: u16 = item_info_entry::item_id_u16.parse_next(input)?;
                // item_protection_index
                let item_protection_index: u16 =
                    item_info_entry::item_protection_index.parse_next(input)?;
                // item_name
                let item_name: &str = item_info_entry::item_name.parse_next(input)?;
                // content_type
                let content_type: String = item_info_entry::content_type.parse_next(input)?;
                // content_encoding (might be just '\0')
                let content_encoding: Option<String> =
                    item_info_entry::content_encoding.parse_next(input)?;
                //
                //////// V1 ONLY
                //
                // extension_type (None if `0`)
                let extension_type: Option<[u8; 4]> = take(4_usize)
                    .try_map(TryInto::<[u8; 4]>::try_into)
                    .parse_next(input)
                    .map(|arr| if arr == *b"    " { None } else { Some(arr) })?;

                Self {
                    _extends: extends,
                    _item_protection_index: item_protection_index,
                    _item_name: item_name.to_string(),
                    version_specific: ItemInfoEntryVersioned::V1 {
                        item_id,
                        _content_type: content_type,
                        _content_encoding: content_encoding,
                        _extension_type: extension_type,
                    },
                }
            }

            2 => {
                // item_id
                let item_id: u16 = item_info_entry::item_id_u16.parse_next(input)?;
                // item_protection_index
                let item_protection_index: u16 =
                    item_info_entry::item_protection_index.parse_next(input)?;
                // item_type
                let item_type: [u8; 4] = item_info_entry::item_type.parse_next(input)?;
                // item_name
                let item_name: &str = item_info_entry::item_name.parse_next(input)?;

                // MIME + URI PARSING HERE
                let mime_or_uri: Option<V2OrV3MimeOrUrl> =
                    item_info_entry::mime_or_uri(input, item_type)?;

                Self {
                    _extends: extends,
                    _item_protection_index: item_protection_index,
                    _item_name: item_name.to_string(),
                    version_specific: ItemInfoEntryVersioned::V2 {
                        item_type,
                        item_id,
                        mime_or_uri,
                    },
                }
            }

            3 => {
                // item_id (u32)
                let item_id: u32 = item_info_entry::item_id_u32.parse_next(input)?;
                // item_protection_index
                let item_protection_index: u16 =
                    item_info_entry::item_protection_index.parse_next(input)?;
                // item_type
                let item_type: [u8; 4] = item_info_entry::item_type.parse_next(input)?;
                // item_name
                let item_name: &str = item_info_entry::item_name.parse_next(input)?;

                // MIME_OR_URI
                let mime_or_uri: Option<V2OrV3MimeOrUrl> =
                    item_info_entry::mime_or_uri(input, item_type)?;

                Self {
                    _extends: extends,
                    _item_protection_index: item_protection_index,
                    _item_name: item_name.to_string(),
                    version_specific: ItemInfoEntryVersioned::V3 {
                        item_type,
                        item_id,
                        mime_or_uri,
                    },
                }
            }

            other => fail
                .context(desc("unsupported item version"))
                .parse_next(input)
                .inspect_err(|_| {
                    log::error!(
                        "Unsupported HEIC item version detected! (version: `{other}`). \
                        Please report this to the `raves_metadata` team!"
                    )
                })?,
        };

        Ok(ret)
    }

    /// Grabs this entry's item ID.
    pub fn item_id(&self) -> u32 {
        match self.version_specific {
            ItemInfoEntryVersioned::V0 { item_id, .. }
            | ItemInfoEntryVersioned::V1 { item_id, .. }
            | ItemInfoEntryVersioned::V2 { item_id, .. } => item_id as u32,
            ItemInfoEntryVersioned::V3 { item_id, .. } => item_id,
        }
    }

    /// Grab this entry's MIME type.
    pub fn mime(&self) -> Option<String> {
        let maybe_mime_or_uri = match self.version_specific {
            ItemInfoEntryVersioned::V2 {
                ref mime_or_uri, ..
            }
            | ItemInfoEntryVersioned::V3 {
                ref mime_or_uri, ..
            } => mime_or_uri,

            _ => return None,
        };

        let Some(mime_or_uri) = maybe_mime_or_uri else {
            return None;
        };

        let V2OrV3MimeOrUrl::Mime { content_type, .. } = mime_or_uri else {
            return None;
        };

        Some(content_type.clone())
    }

    /// Returns this entry's FourCC representation ("item type").
    ///
    /// Only available to v2 + v3.
    pub fn item_type(&self) -> Option<[u8; 4]> {
        match self.version_specific {
            ItemInfoEntryVersioned::V2 { item_type, .. }
            | ItemInfoEntryVersioned::V3 { item_type, .. } => Some(item_type),
            _ => None,
        }
    }
}

mod item_info_entry {
    use crate::providers::shared::{
        bmff::heif::iinf::V2OrV3MimeOrUrl, desc, parse_nul_terminated_str,
    };
    use winnow::{
        ModalResult, Parser as _,
        binary::{be_u16, be_u32},
        error::ContextError,
        token::take,
    };

    /// Grabs the item ID as a `u16`.
    pub fn item_id_u16(input: &mut &[u8]) -> ModalResult<u16, ContextError> {
        be_u16.context(desc("item id")).parse_next(input)
    }

    /// Grabs the item ID as a `u32`.
    pub fn item_id_u32(input: &mut &[u8]) -> ModalResult<u32, ContextError> {
        be_u32.context(desc("item id")).parse_next(input)
    }

    /// Parses out the item protection index as a `u16`.
    pub fn item_protection_index(input: &mut &[u8]) -> ModalResult<u16, ContextError> {
        be_u16
            .context(desc("item protection index"))
            .parse_next(input)
    }

    pub fn item_name<'input>(input: &mut &'input [u8]) -> ModalResult<&'input str, ContextError> {
        parse_nul_terminated_str
            .context(desc("item name"))
            .parse_next(input)
    }

    /// Parses out the content type.
    ///
    /// Errors if the input string isn't NUL-terminated.
    pub fn content_type(input: &mut &[u8]) -> ModalResult<String, ContextError> {
        parse_nul_terminated_str
            .context(desc("content type"))
            .map(ToString::to_string)
            .parse_next(input)
    }

    /// Grabs the content encoding. If it's:
    ///
    /// - a blank string, returns `Ok(None)`.
    /// - a string with context, returns `Ok(Some(str))`.
    /// - lacking a NUL terminator, returns `Err`.
    pub fn content_encoding(input: &mut &[u8]) -> ModalResult<Option<String>, ContextError> {
        parse_nul_terminated_str
            .context(desc("content encoding"))
            .parse_next(input)
            .map(|s| {
                // when it's an empty string, it's `None`
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            })
    }

    /// Gets the item type (in V2/V3) as an array.
    pub fn item_type(input: &mut &[u8]) -> ModalResult<[u8; 4], ContextError> {
        take(4_usize)
            .context(desc("item type"))
            .parse_next(input)
            .map(|slice| match TryInto::<[u8; 4]>::try_into(slice) {
                Ok(arr) => arr,
                Err(_) => unreachable!(),
            })
    }

    /// Gets the optional MIME/URI type (in V2/V3).
    pub fn mime_or_uri(
        input: &mut &[u8],
        item_type: [u8; 4],
    ) -> ModalResult<Option<V2OrV3MimeOrUrl>, ContextError> {
        let ascii: [char; 4] = (item_type).map(char::from);
        log::trace!(
            "Given item type of: `{item_type:?}`. ASCII form: `{}{}{}{}`",
            ascii[0],
            ascii[1],
            ascii[2],
            ascii[3]
        );

        match &item_type {
            // MIME type
            b"mime" => ModalResult::Ok(Some(V2OrV3MimeOrUrl::Mime {
                content_type: content_type.parse_next(input)?,
                _content_encoding: content_encoding.parse_next(input)?,
            })),

            // URI
            b"uri " => ModalResult::Ok(Some(V2OrV3MimeOrUrl::Uri {
                _item_uri_type: parse_nul_terminated_str
                    .context(desc("item URI type"))
                    .map(ToString::to_string)
                    .parse_next(input)?,
            })),

            // empty is None
            b"    " => ModalResult::Ok(None),

            // any others are None, but with a warning
            other => {
                log::warn!(
                    "Got an unknown MIME or URI type: `{other:?}` (ASCII: `{}{}{}{}`)",
                    ascii[0],
                    ascii[1],
                    ascii[2],
                    ascii[3]
                );
                ModalResult::Ok(None)
            }
        }
    }
}

/// Version-specific item info.
#[derive(Clone)]
pub enum ItemInfoEntryVersioned {
    V0 {
        // shared between v0 and v1
        item_id: u16,
        _content_type: String,
        _content_encoding: Option<String>,
    },

    V1 {
        // shared between v0 and v1
        item_id: u16,
        _content_type: String,
        _content_encoding: Option<String>,

        // v1 explicitly
        _extension_type: Option<[u8; 4]>,
    },

    V2 {
        item_type: [u8; 4],
        item_id: u16,
        mime_or_uri: Option<V2OrV3MimeOrUrl>,
    },

    V3 {
        item_type: [u8; 4],
        item_id: u32,
        mime_or_uri: Option<V2OrV3MimeOrUrl>,
    },
}

/// Either a MIME or URI, used only in `ItemInfoEntryVersioned::V2OrV3`.
#[derive(Clone, Debug, PartialEq)]
pub enum V2OrV3MimeOrUrl {
    /// The item is described by a MIME type.
    Mime {
        content_type: String,
        _content_encoding: Option<String>,
    },

    /// The item is described by a URI.
    Uri { _item_uri_type: String },
}

#[cfg(test)]
mod tests {
    use crate::{providers::shared::bmff::heif::iinf::V2OrV3MimeOrUrl, util::logger};

    /// Ensures we can parse out a MIME type.
    #[test]
    fn parse_out_mime_type() {
        logger();

        // define a mime item type
        const ITEM_TYPE: [u8; 4] = *b"mime";

        // define the blob.
        //
        // 1. `content_type`.
        // 2. `content_encoding`, but it's blank, so should become `None`.
        let blob: &mut &[u8] = &mut b"application/rdf+xml\0\0".as_slice();

        // ensure they're equal
        assert_eq!(
            super::item_info_entry::mime_or_uri(blob, ITEM_TYPE)
                .unwrap()
                .unwrap(),
            V2OrV3MimeOrUrl::Mime {
                content_type: "application/rdf+xml".into(),
                _content_encoding: None
            }
        );
    }
}
