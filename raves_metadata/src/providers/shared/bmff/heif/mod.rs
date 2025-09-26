//! A parser generic over HEIC, AVIF, and other formats with identical
//! structure.
//!
//! Here's a nice resource explaining the file structure:
//!
//! https://github.com/spacestation93/heif_howto

use std::{collections::HashMap, fmt::Write as _, sync::Arc};

use parking_lot::RwLock;
use winnow::{Parser as _, binary::be_u32, combinator::peek, error::EmptyError};

use crate::{
    MetadataProvider, MetadataProviderRaw,
    providers::shared::{
        bmff::{
            BoxHeader, BoxType,
            ftyp::FtypBox,
            heif::{
                iinf::{FullBox, ItemInfoBox, ItemInfoEntry},
                iloc::{ConstructionMethod, ItemExtent, ItemLocationBox, ItemLocationEntry},
                pitm::PrimaryItemBox,
            },
        },
        desc,
    },
    util::{MaybeParsedExif, MaybeParsedXmp},
};

mod iinf;
mod iloc;
mod pitm;
mod search;

/// A HEIF-like file.
#[derive(Clone, Debug)]
pub struct HeifLike {
    exif: Arc<RwLock<Option<MaybeParsedExif>>>,
    xmp: Arc<RwLock<Option<MaybeParsedXmp>>>,
}

impl HeifLike {
    pub fn parse(
        input: &mut &[u8],
        supported_ftyp_entries: &[[u8; 4]],
    ) -> Result<HeifLike, HeifLikeConstructionError> {
        parse_heif_like(input, supported_ftyp_entries)
    }
}

impl MetadataProviderRaw for HeifLike {
    fn exif_raw(&self) -> Arc<RwLock<Option<MaybeParsedExif>>> {
        Arc::clone(&self.exif)
    }

    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::clone(&self.xmp)
    }
}

impl<'input> MetadataProvider for HeifLike {
    type ConstructionError = HeifLikeConstructionError;

    /// DO NOT CALL THIS.
    ///
    /// Call `Self::parse` instead.
    fn new(
        _input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        unreachable!(
            "this is an implementation detail that's effectively private. \
            please call the `parse` method instead."
        )
    }
}

fn parse_heif_like<'input>(
    input: &mut &'input [u8],
    supported_ftyp_entries: &[[u8; 4]],
) -> Result<HeifLike, HeifLikeConstructionError> {
    // save the "original" input so we can use it later when parsing w/
    // offsets.
    //
    // required since MPEG-21 ("items") require parsing w/ this style, which
    // includes HEIF files...
    let original_input: &'input [u8] = input;

    // grab the `ftyp` box, which must be the first in the file.
    let ftyp: FtypBox = FtypBox::new(input).ok_or_else(|| {
        log::error!(
            "The `ftyp` box was not found. It may not be the first \
            box in the file."
        );
        HeifLikeConstructionError::NoFtypBox
    })?;
    log::trace!("found ftyp box! major brand: {:?}", ftyp.major_brand);

    // ensure the brand is correct
    if !supported_ftyp_entries.contains(&ftyp.major_brand)
        && !ftyp
            .compatible_brands
            .iter()
            .any(|c| supported_ftyp_entries.contains(c))
    {
        log::error!("Not a HEIF-like file. Returning error.");
        return Err(HeifLikeConstructionError::NotAHeifLike {
            major_brand: ftyp.major_brand,
        });
    }

    // try to grab the `meta` box.
    //
    // if we don't find one, then the media probably doesn't have metadata.
    log::trace!("Looking for meta boxes...");
    let mut meta_box_list: Vec<(FullBox, &[u8])> = search::find_meta_boxes(input);

    // ensure we've just got one
    let mut meta_box: (FullBox, &[u8]) = match meta_box_list.len() {
        // zero metadata boxes?
        //
        // then we've got no metadata. ez pz for this library
        0 => {
            log::debug!(
                "The `meta` box had no children. \
                No metadata to find, so returning!"
            );
            return Ok(HeifLike {
                exif: Arc::new(const { RwLock::new(None) }),
                xmp: Arc::new(const { RwLock::new(None) }),
            });
        }

        // ah, perfect.
        //
        // let's steal the box
        1 => meta_box_list.remove(0),

        // more than one metabox isn't currently supported.
        //
        // print + return an error
        other => {
            log::error!(
                "Multiple `meta` boxes detected! This structure isn't \
                currently supported. Please create an issue and upload your \
                image if you encounter this error. \
                expected `1` meta box, but found `{other}`..!"
            );
            return Err(HeifLikeConstructionError::MultipleMetaBoxes { n: other as u32 });
        }
    };
    log::trace!("Found one meta box.");
    let meta_blob = &mut meta_box.1;

    // try to find the `ItemInfoBox` and `ItemLocationBox`.
    //
    // - `ItemInfoBox` contains info about what items will be present
    // - `ItemLocationBox` says where things will be in the file
    // - `PrimaryItemBox` notes which item is the "primary" one
    // - `ItemDataBox` contains metadata, if `construction_method` specifies
    let mut maybe_item_info: Option<ItemInfoBox> = None;
    let mut maybe_item_location: Option<ItemLocationBox> = None;
    let mut maybe_item_data: Option<&[u8]> = None;
    let mut maybe_primary_item: Option<PrimaryItemBox> = None;
    while !meta_blob.is_empty() {
        if maybe_item_info.is_some() && maybe_item_location.is_some() {
            break;
        }

        // parse next box (without consuming its data)
        let box_header: BoxHeader = match peek(BoxHeader::new).parse_next(meta_blob) {
            Ok(bh) => bh,
            Err(e) => {
                log::warn!("Failed to parse box header! err: {e}");
                break;
            }
        };

        match box_header.box_type {
            // ItemInfoBox (`iinf`)
            ty if ty == BoxType::Id(*b"iinf") => {
                maybe_item_info = Some(
                    ItemInfoBox::new
                        .parse_next(meta_blob)
                        .inspect_err(|e| {
                            log::error!("Failed to parse `ItemInfoBox` inside `MetaBox`. err: {e}")
                        })
                        .map_err(|_| HeifLikeConstructionError::CantParseItemInfoBox)?,
                );
            }

            // ItemLocationBox (`iloc`)
            ty if ty == BoxType::Id(*b"iloc") => {
                maybe_item_location = Some(
                    ItemLocationBox::new
                        .parse_next(meta_blob)
                        .inspect_err(|e| {
                            log::error!(
                                "Failed to parse `ItemLocationBox` inside `MetaBox`. err: {e}"
                            )
                        })
                        .map_err(|_| HeifLikeConstructionError::CantParseItemLocationBox)?,
                );
            }

            // ItemDataBox (`idat`)
            ty if ty == BoxType::Id(*b"idat") => {
                if let Some(blob) = BoxHeader::new
                    .context(desc("item data box header"))
                    .parse_next(meta_blob)
                    .ok()
                    .and_then(|header: BoxHeader| header.payload(input))
                {
                    maybe_item_data = Some(blob);
                } else {
                    log::error!("Failed to build `idat`.");
                };
            }

            // PrimaryItemBox (`pitm`)
            ty if ty == BoxType::Id(*b"pitm") => {
                maybe_primary_item = Some(
                    PrimaryItemBox::new
                        .parse_next(meta_blob)
                        .inspect_err(|e| {
                            log::error!(
                                "Failed to parse `PrimaryItemBox` inside `MetaBox`. err: {e}"
                            )
                        })
                        .map_err(|_| HeifLikeConstructionError::CantParsePrimaryItemBox)?,
                );
            }

            unsupported_box_type => {
                log::trace!("Skipping unsupported box type: `{unsupported_box_type:?}`");

                // skip it
                _ = BoxHeader::new
                    .parse_next(meta_blob)
                    .ok()
                    .and_then(|header| header.eat_payload(meta_blob));
            }
        }
    }
    log::trace!("Item info found? {}", maybe_item_info.is_some());
    log::trace!("Item location found? {}", maybe_item_location.is_some());
    log::trace!("Item data found? {}", maybe_item_data.is_some());
    log::trace!("Primary item found? {}", maybe_primary_item.is_some());

    // ensure we have item info
    let Some(item_info) = maybe_item_info else {
        log::debug!(
            "No item info detected, so there can't be any metadata. \
            Returning blank metadata."
        );
        return Ok(HeifLike {
            exif: Arc::new(const { RwLock::new(None) }),
            xmp: Arc::new(const { RwLock::new(None) }),
        });
    };

    //  loc info
    let Some(item_location) = maybe_item_location else {
        log::debug!(
            "No item locations detected, so we can't find any metadata. \
            Returning blank metadata."
        );
        return Ok(HeifLike {
            exif: Arc::new(const { RwLock::new(None) }),
            xmp: Arc::new(const { RwLock::new(None) }),
        });
    };

    let metadata_blobs = find_metadata(
        original_input,
        item_info,
        item_location,
        maybe_item_data,
        maybe_primary_item,
    )
    .inspect_err(|e| log::error!("Failed to parse final metadata blobs. err: {e}"))
    .inspect(|t| {
        log::trace!("Found Exif? {}", t.exif.is_some());
        log::trace!("Found XMP? {}", t.xmp.is_some());
    })?;

    Ok(HeifLike {
        exif: Arc::new(RwLock::new(
            metadata_blobs
                .exif
                .map(|raw| MaybeParsedExif::Raw(raw.to_vec())),
        )),
        xmp: Arc::new(RwLock::new(
            metadata_blobs
                .xmp
                .map(|raw| MaybeParsedXmp::Raw(raw.to_vec())),
        )),
    })
}

struct FindMetadataReturnValues<'input> {
    exif: Option<&'input [u8]>,
    xmp: Option<&'input [u8]>,
}

#[derive(Clone)]
struct ItemData {
    item_id: u32,
    item_location: ItemLocationEntry,
    item_info: ItemInfoEntry,
}

fn find_metadata<'input>(
    // blobs:
    //
    // 1. original file blob (for file-based indexing)
    // 2. original `meta` blob
    original_file_blob: &'input [u8],

    // data on _what_ will be _where_ in the blobs
    item_info: ItemInfoBox,
    item_location: ItemLocationBox,
    maybe_item_data: Option<&'input [u8]>,
    maybe_primary_item: Option<PrimaryItemBox>,
) -> Result<FindMetadataReturnValues<'input>, HeifLikeConstructionError> {
    // make an index of what items we've got
    let item_infos_len = item_info.item_infos.len();
    let mut item_infos: HashMap<u32, ItemInfoEntry> = item_info.item_infos.into_iter().fold(
        HashMap::with_capacity(item_infos_len),
        |mut map, item_info_entry| {
            log::debug!(
                "Found item_info with ID: {}, item_type: {:?}",
                item_info_entry.item_id(),
                item_info_entry.item_type()
            );
            map.insert(item_info_entry.item_id(), item_info_entry);
            map
        },
    );
    log::trace!("Num. of items: `{item_infos_len}`");

    // create a list of all items
    let items: Vec<ItemData> = item_location
        .items
        .into_iter()
        .filter_map(|item_location| {
            log::debug!(
                "Processing item_location with ID: {}",
                item_location.item_id
            );
            if let Some(item_info) = item_infos.remove(&item_location.item_id) {
                log::debug!("Successfully matched item ID: {}", item_location.item_id);
                Some(ItemData {
                    item_id: item_location.item_id,
                    item_info,
                    item_location,
                })
            } else {
                log::warn!(
                    "No item_info found for item_location ID: {}",
                    item_location.item_id
                );
                None
            }
        })
        .collect();
    log::debug!("After filtering, we have `{}` items!", items.len());

    // we'll want to find both exif and xmp data
    let mut ret = FindMetadataReturnValues {
        exif: None,
        xmp: None,
    };

    // try to find both exif and xmp in there.
    for (i, item) in items.iter().enumerate() {
        log::debug!(
            "Processing item {}/{}: ID={}, item_type={:?}",
            i + 1,
            items.len(),
            item.item_id,
            item.item_info.item_type()
        );

        // stop looping (so... return) if we've found both
        if ret.exif.is_some() && ret.xmp.is_some() {
            break;
        }

        // based on item's construction method, we'll choose what to do...
        match item.item_location.construction_method {
            // easy!
            //
            // go to this offset and read a box...
            ConstructionMethod::Set0 => {
                log::trace!("Construction method: File offsets (Set0)");

                // update `ret` w/ the parsed item
                update_with_item(&mut ret, item.clone(), original_file_blob)?;
            }

            // oooh. little bit harder.
            //
            // use the `idat` stream above and find it in there
            ConstructionMethod::Idat => {
                log::trace!("Construction method: Item data");

                // ensure we have the item data
                let Some(item_data) = maybe_item_data else {
                    log::warn!("File specified that `idat` should be present, but it wasn't...");
                    continue;
                };

                // update `ret` w/ the parsed item
                update_with_item(&mut ret, item.clone(), item_data)?;
            }

            // not doing this right now, but this is the `extent`-based one.
            //
            // basically, imagine pointers to random items and having to read
            // off those. sounds annoying.
            //
            // if you have a need for this, please submit an issue with a test
            // file you own the rights to (e.g., CC0). I'll implement it! :D
            ConstructionMethod::Item => {
                log::trace!("Construction method: Item");
            }
        }
    }

    Ok(ret)
}

/// Creates the range we'll use to slice above.
fn make_slice_range(
    item: &ItemData,
    extent: &ItemExtent,
) -> Result<core::ops::Range<usize>, HeifLikeConstructionError> {
    let start: u64 = item
        .item_location
        .base_offset
        .saturating_add(extent.extent_offset);
    let end: u64 = start.saturating_add(extent.extent_length);

    Ok(start
        .try_into()
        .map_err(|_| HeifLikeConstructionError::ParserBugSlicesTooSmall)?
        ..end
            .try_into()
            .map_err(|_| HeifLikeConstructionError::ParserBugSlicesTooSmall)?)
}

/// Updates the metadata return values with the given item.
fn update_with_item<'input>(
    ret: &mut FindMetadataReturnValues<'input>,
    item: ItemData,
    blob: &'input [u8],
) -> Result<(), HeifLikeConstructionError> {
    log::trace!("Updating found metadata w/ item. ID: `#{}`", item.item_id);

    // ensure only one extent present
    let [single_extent] = item.item_location.extents.as_slice() else {
        log::error!("Multiple extents are not currently supported.");
        if cfg!(debug_assertions) {
            panic!("Multiple extents are not currently supported.");
        } else {
            return Ok(());
        }
    };

    // grab our single extent as a blob (via slicing)
    let slice_range = make_slice_range(&item, single_extent)
        .inspect_err(|e| log::error!("Failed to make slice range! err: {e}"))?;
    log::trace!("Slice range: {slice_range:?}");

    let blob: &mut &[u8] = &mut &blob[slice_range];

    // exif
    if item.item_info.item_type() == Some(*b"Exif") {
        let blob_len: usize = blob.len();

        // handle some literal nonsense
        //
        // (some images can omit the required header)
        let first_two_bytes: [u8; 2] = [blob[0], blob[1]];
        let exif_tiff_header_offset = if (first_two_bytes == *b"MM" || first_two_bytes == *b"II")
            && blob_len < u16::from_be_bytes(first_two_bytes).into()
        {
            log::warn!(
                "Malformed Exif header detected. Missing `exif_tiff_header_offset`. \
                Assuming value of zero..."
            );
            0
        } else {
            // parse out the u32 explaining how many bytes to skip
            let Ok::<_, EmptyError>(exif_tiff_header_offset) = be_u32
                .context(desc("exif_tiff_header_offset"))
                .parse_next(blob)
                .map(|off| off as usize)
            else {
                log::error!("Failed to grab `exif_tiff_header_offset` for Exif! Skipping...");
                return Ok(());
            };
            exif_tiff_header_offset
        };

        // check bounds
        log::trace!("`exif_tiff_header_offset` is: `{exif_tiff_header_offset}`");
        if blob_len < exif_tiff_header_offset {
            log::warn!(
                "`exif_tiff_header_offset` was larger than the blob. \
                blob len: `{blob_len}`, \
                offset: `{exif_tiff_header_offset}`",
            );
            if cfg!(debug_assertions) {
                panic!();
            }
        }

        // move to offset (or start from beginning)
        ret.exif = Some(&blob[exif_tiff_header_offset..]);

        log::trace!("Updated w/ Exif!");
        return Ok(());
    }
    log::debug!("passed Exif");

    // xmp (using mime)
    if let Some(mime) = item.item_info.mime() {
        if mime == "application/rdf+xml" || mime == "application/xmp+xml" {
            log::trace!("Updated w/ XMP when using MIME!");
            ret.xmp = Some(blob);
            return Ok(());
        }
    }
    log::debug!("passed XMP (MIME)");

    // xmp (when using item)
    if let Some(item_type) = item.item_info.item_type() {
        if [b"xif\0", b"XMP ", b"xmp "].contains(&&item_type) {
            log::trace!("Updated w/ XMP using item type: {item_type:?}");
            ret.xmp = Some(blob);
            return Ok(());
        }
    }
    log::debug!("passed XMP (item)");

    Ok(())
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum HeifLikeConstructionError {
    /// All HEIF-like files must provide `ftyp` box as soon as possible.
    ///
    /// However, this file didn't.
    NoFtypBox,

    /// The `ftyp` box indicated that this isn't a HEIF-like file.
    NotAHeifLike { major_brand: [u8; 4] },

    /// HEIF files are only expected to have one `meta` box, but the given file
    /// had more than one!
    MultipleMetaBoxes { n: u32 },

    /// Failed to parse the `ItemInfoBox`.
    CantParseItemInfoBox,

    /// Failed to parse the `ItemLocationBox`.
    CantParseItemLocationBox,

    /// Failed to parse the `PrimaryItemBox`.
    CantParsePrimaryItemBox,

    /// Multiple extents aren't currently supported.
    ///
    /// They're annoying to implement. If you own the rights to a file and can
    /// include it here for testing, please create an issue and I will
    /// implement multi-extent parsing.
    ParserBugMultipleExtentsNotSupported { extent_ct: u32 },

    /// Rust slices cannot represent giant files on 32-bit systems.
    ///
    /// This is a limitation of the parser. Please make an issue if you see
    /// this pop up.
    ParserBugSlicesTooSmall,
}

impl core::fmt::Display for HeifLikeConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFtypBox => f.write_str("File did not start with an `ftyp` box."),
            Self::NotAHeifLike { major_brand } => {
                f.write_str("The provided file was not a known HEIF-like. major brand: `")?;
                for c in major_brand {
                    f.write_char(*c as char)?;
                }
                f.write_char('`')
            }
            Self::MultipleMetaBoxes { n } => write!(
                f,
                "HEIF-like file had more than one `meta` box, but this isn't \
                allowed. \
                box ct: `{n}`"
            ),
            Self::CantParseItemInfoBox => f.write_str("Failed to parse `ItemInfoBox`."),
            Self::CantParseItemLocationBox => f.write_str("Failed to parse `ItemLocationBox`."),
            Self::CantParsePrimaryItemBox => f.write_str("Failed to parse `PrimaryItemBox`."),

            Self::ParserBugMultipleExtentsNotSupported { extent_ct } => write!(
                f,
                "This library does not currently support multi-extent parsing. \
                Please see variant docs for more info. \
                num. of extents: `{extent_ct}`"
            ),
            Self::ParserBugSlicesTooSmall => f.write_str(
                "Slice cannot represent this range on your system. \
                Please see variant docs for more info.",
            ),
        }
    }
}

impl core::error::Error for HeifLikeConstructionError {}
