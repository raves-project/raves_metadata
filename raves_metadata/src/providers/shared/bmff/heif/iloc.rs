use winnow::{
    ModalResult, Parser as _,
    binary::{be_u16, be_u32, u8},
    combinator::fail,
    error::ContextError,
    token::take,
};

use crate::providers::shared::{bmff::heif::iinf::FullBox, desc};

#[derive(Clone)]
pub struct ItemLocationBox {
    pub _extends_full_box: FullBox,

    // technically only available for v1 and v2
    pub _index_size: u8,  // u4
    pub _item_count: u32, // note: MUST parse out as u16 from `v1`

    pub _offset_size: u8, // u4
    pub _length_size: u8, // u4

    /// Says how long the `base_offset` field (on `ItemLocationEntry`) is.
    ///
    /// Must be one of: `{0, 4, 8}`.
    pub _base_offset_size: u8, // u4

    pub items: Vec<ItemLocationEntry>,
}

impl ItemLocationBox {
    pub fn new(input: &mut &[u8]) -> ModalResult<Self, ContextError> {
        let extends_full_box: FullBox = FullBox::new.parse_next(input)?;

        // we're grabbing these in order of where they appear
        let byte_1: u8 = u8
            .context(desc("offset size and length size"))
            .parse_next(input)?;
        let offset_size: u8 = byte_1 >> 4;
        if ![0, 4, 8].contains(&offset_size) {
            log::error!(
                "Offset size is a weird value! \
                Should be one of [0, 4, 8], but was: `{offset_size}`"
            );
            return fail
                .context(desc("offset size was a weird value"))
                .parse_next(input);
        }

        let length_size: u8 = 0b_0000_1111 & byte_1;
        if ![0, 4, 8].contains(&length_size) {
            log::error!(
                "Length size is a weird value! \
                Should be one of [0, 4, 8], but was: `{length_size}`"
            );
            return fail
                .context(desc("length size was a weird value"))
                .parse_next(input);
        }

        let byte_2: u8 = u8
            .context(desc("base offset size + index size"))
            .parse_next(input)?;

        let base_offset_size: u8 = byte_2 >> 4;
        if ![0, 4, 8].contains(&base_offset_size) {
            log::error!(
                "Base offset size is a weird value! \
                Should be one of [0, 4, 8], but was: `{base_offset_size}`"
            );
            return fail
                .context(desc("base offset size was a weird value"))
                .parse_next(input);
        }

        let index_size: u8 = if extends_full_box.version == 0 {
            0 // we already took the bits, so just leave it as a `0`
        } else {
            0b_0000_1111 & byte_2
        };
        if ![0, 4, 8].contains(&index_size) {
            log::error!(
                "Index size is a weird value! \
                Should be one of [0, 4, 8], but was: `{index_size}`"
            );
            return fail
                .context(desc("index size was a weird value"))
                .parse_next(input);
        }

        let item_count: u32 = if extends_full_box.version < 2 {
            be_u16.context(desc("item count (u16)")).parse_next(input)? as u32
        } else {
            be_u32.context(desc("item count (u32)")).parse_next(input)?
        };

        let mut items = Vec::with_capacity(item_count as usize);
        for _ in 0..item_count {
            items.push(ItemLocationEntry::new(
                input,
                extends_full_box.version,
                base_offset_size,
                index_size,
                offset_size,
                length_size,
            )?);
        }

        Ok(Self {
            _extends_full_box: extends_full_box,

            _index_size: index_size,
            _item_count: item_count,
            _offset_size: offset_size,
            _length_size: length_size,
            _base_offset_size: base_offset_size,
            items,
        })
    }
}

#[repr(u8)]
#[derive(Clone, PartialEq, PartialOrd)]
pub enum ConstructionMethod {
    /// Go exactly to the offset in the file.
    Set0 = 0_u8,

    /// There'll be an `idat` box in this `meta` box.
    ///
    /// The data will be at that offset in the box, using box offsets.
    Idat = 1_u8,

    /// It's an item, so we'll find it via item offset.
    Item = 2_u8,
}

#[derive(Clone)]
pub struct ItemLocationEntry {
    pub item_id: u32, // note: MUST parse out as u16 from `v1`
    pub construction_method: ConstructionMethod,
    pub _data_reference_index: u16,

    pub base_offset: u64,

    pub extents: Vec<ItemExtent>,
}

impl ItemLocationEntry {
    pub fn new(
        input: &mut &[u8],
        version: u8,

        // used to calc right # of bytes in this entry
        base_offset_size: u8,

        // these are all used to calc right # of bytes in extents
        index_size: u8,
        offset_size: u8,
        length_size: u8,
    ) -> ModalResult<Self, ContextError> {
        let item_id: u32 = if version < 2 {
            be_u16.context(desc("item id (u16)")).parse_next(input)? as u32
        } else {
            be_u32.context(desc("item id (u32)")).parse_next(input)?
        };

        let construction_method: ConstructionMethod = if version == 0 {
            // if the version is 0, the file is using an older version of
            // the spec.
            //
            // we'll use an absolute offset by default for this case
            log::trace!("Version is `0`. Using `ConstructionMethod::Set0` by default...");
            ConstructionMethod::Set0
        } else {
            // throw away first byte
            take(1_usize).context(desc("padding")).parse_next(input)?;

            // grab next byte
            let cm_byte: u8 = u8
                .context(desc("construction method byte"))
                .parse_next(input)?
                & 0b_0000_1111; // only pick up the last 4 bits. just in case someone messed up somewhere

            // map via enum value
            match cm_byte {
                0 => ConstructionMethod::Set0,
                1 => ConstructionMethod::Idat,
                2 => ConstructionMethod::Item,
                _ => fail
                    .context(desc("construction method byte must be <=2."))
                    .parse_next(input)?,
            }
        };

        let data_reference_index = be_u16
            .context(desc("data reference index"))
            .parse_next(input)?;

        let base_offset: u64 = take(base_offset_size)
            .context(desc("base offset (without mult.)"))
            .map(map_bytes_to_u64)
            .parse_next(input)?;

        let extent_count: u16 = be_u16.parse_next(input)?;

        // handle extents
        let mut extents = Vec::with_capacity(extent_count.into());
        for _ in 0..extent_count {
            let extent: ItemExtent = ItemExtent {
                _extent_index: if index_size > 0 && version > 0 {
                    Some(
                        take(index_size)
                            .context(desc("extent index"))
                            .map(map_bytes_to_u64)
                            .parse_next(input)?,
                    )
                } else {
                    None
                },
                extent_offset: take(offset_size)
                    .context(desc("extent_offset"))
                    .map(map_bytes_to_u64)
                    .parse_next(input)?,
                extent_length: take(length_size)
                    .context(desc("extent_length"))
                    .map(map_bytes_to_u64)
                    .parse_next(input)?,
            };

            extents.push(extent);
        }

        Ok(ItemLocationEntry {
            item_id,
            construction_method,
            _data_reference_index: data_reference_index,
            base_offset,
            extents,
        })
    }
}

/// An extent on an item.
///
/// Explains where to find it in the file.
#[derive(Clone)]
pub struct ItemExtent {
    pub _extent_index: Option<u64>,
    pub extent_offset: u64,
    pub extent_length: u64,
}

/// Given some BE bytes, this maps those bytes into `u64`.
///
/// Used since some fields are of (moderately) variable size.
fn map_bytes_to_u64(slice: &[u8]) -> u64 {
    let mut acc: u64 = 0;

    for byte in slice {
        // shift left 8b, then OR w/ byte, which contains all bits on the right
        acc = (acc << 8) | *byte as u64;
    }

    acc
}
