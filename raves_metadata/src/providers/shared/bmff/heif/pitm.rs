use winnow::{
    ModalResult, Parser,
    binary::{be_u16, be_u32},
    combinator::{fail, peek},
    error::ContextError,
};

use crate::providers::shared::{
    bmff::{BoxType, heif::iinf::FullBox},
    desc,
};

/// A box that indicates the primary item of the file.
pub struct PrimaryItemBox {
    pub extends_full_box: FullBox,

    pub item_id: u32, // u16 if ver. 0
}

impl PrimaryItemBox {
    pub fn new(input: &mut &[u8]) -> ModalResult<Self, ContextError> {
        // peek to check box type
        let extends_full_box: FullBox = peek(FullBox::new)
            .context(desc("full box"))
            .parse_next(input)?;

        // handle wrong box type
        if extends_full_box.extends.box_type != BoxType::Id(*b"pitm") {
            log::error!(
                "Box of type `{:?}` is not a `PrimaryItemBox`.",
                extends_full_box.extends.box_type
            );
            fail.context(desc("not a PrimaryItemBox"))
                .parse_next(input)?;
        }

        // take the input (we peek'd before)
        _ = FullBox::new.parse_next(input)?;

        // return self
        Ok(Self {
            item_id: if extends_full_box.version == 0 {
                be_u16.context(desc("item id (u16)")).parse_next(input)? as u32
            } else {
                be_u32.context(desc("item id (u32)")).parse_next(input)?
            },

            extends_full_box,
        })
    }
}
