use winnow::{
    ModalResult, Parser as _,
    binary::{be_u32, be_u64},
    error::{ContextError, EmptyError, StrContext, StrContextValue},
    token::take,
};

use crate::providers::shared::bmff::{BoxSize, BoxType};

/// A box's header says:
///
/// - what "type" it is (might be UUID)
/// - how large it is
/// - and, optionally, a UUID
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct BoxHeader {
    /// How long the header is.
    pub header_len: u8,

    /// How large the box is. This includes the header's size.
    pub box_size: BoxSize,

    /// The box's type.
    pub box_type: BoxType,
}

impl BoxHeader {
    /// Finds the next header in the file and parses it out.
    ///
    /// This function assumes the byte slice starts at a header. This means you
    /// should pass it a byte slice with the previous header's offset applied, or
    /// the file is starting from the beginning.
    ///
    /// Note that the input is mutated - skip `size - taken_bytes`.
    pub fn new(input: &mut &[u8]) -> ModalResult<BoxHeader, ContextError> {
        parse_header(input)
    }

    /// Finds payload's length (which is everything after the header).
    ///
    /// This is optional since with an EOF case, we don't know how much is
    /// left. We just know to parse the rest of the slice.
    pub fn payload_len(&self) -> Option<u64> {
        match self.box_size {
            BoxSize::Small(n) => Some((n.saturating_sub(self.header_len as u32)) as u64),
            BoxSize::Large(n) => Some(n.saturating_sub(self.header_len as u64)),
            BoxSize::Eof => None,
        }
    }

    /// Attempts to grab the box payload for this box header.
    ///
    /// This function assumes that you haven't modifed/eaten any bytes inside
    /// the payload - if so, you'll need to do this manually.
    pub fn payload<'borrow, 'input: 'borrow>(
        &self,
        input: &'borrow mut &'input [u8],
    ) -> Option<&'input [u8]> {
        let len: u64 = self.payload_len().unwrap_or(input.len() as u64);

        take(len)
            .parse_next(input)
            .inspect_err(|_: &EmptyError| {
                log::warn!("Failed to fetch box payload! payload len: `{len}`")
            })
            .ok()
    }

    /// Attempts to "eat" the box payload for this box header.
    ///
    /// That means the payload won't be kept, so calling this method requires
    /// that you don't need the data you'd get otherwise.
    ///
    /// This function assumes that you haven't modifed/eaten any bytes inside
    /// the payload - if so, you'll need to do this manually.
    pub fn eat_payload(&self, input: &mut &[u8]) -> Option<()> {
        let len: u64 = self.payload_len().unwrap_or(input.len() as u64);
        take(len)
            .void()
            .parse_next(input)
            .inspect_err(|_: &EmptyError| {
                log::warn!("Failed to eat box payload! payload len: `{len}`")
            })
            .ok()
    }
}

pub fn parse_header(input: &mut &[u8]) -> ModalResult<BoxHeader, ContextError> {
    // we're going to track the length of our box as we parse.
    //
    // the amount of bytes we took is given in the `BoxHeader`
    let start_len = input.len();

    // grab the raw "size" and "type" from the input.
    //
    // we parse these more below...
    let raw_size: u32 = be_u32
        .context(StrContext::Expected(StrContextValue::Description(
            "box size",
        )))
        .parse_next(input)?;
    let raw_type: u32 = be_u32
        .context(StrContext::Expected(StrContextValue::Description(
            "box type",
        )))
        .parse_next(input)?;

    // parse the size into something more usable
    let size: BoxSize = match raw_size {
        // special case: we have a largesize to parse.
        //
        // and, well, we've already parsed the first two parts of this header.
        // let's also take the `large_size`, which comes next. it's a `u64`...
        1_u32 => BoxSize::Large(
            be_u64
                .context(StrContext::Expected(StrContextValue::Description(
                    "box size (large)",
                )))
                .parse_next(input)
                .inspect_err(|e| log::error!("Failed to find large box size! err: {e}"))?,
        ),

        // special case: when it's zero, read to EOF (this is the end!)
        0_u32 => BoxSize::Eof,

        // for anything else, it's just a small box, so we use the raw size
        _ => BoxSize::Small(raw_size),
    };

    // now, we'll grab the type.
    //
    // this means we check if we've got a UUID on our hands
    const CASE_UUID: u32 = const { u32::from_be_bytes(*b"uuid") };
    let ty: BoxType = match raw_type {
        // we do have a UUID! keep reading for the full string...
        CASE_UUID => {
            const LEN: usize = 16_usize;
            let chars: [u8; LEN] = TryInto::<[u8; LEN]>::try_into(
                take(LEN)
                    .context(StrContext::Expected(StrContextValue::Description(
                        "grab box UUID",
                    )))
                    .parse_next(input)?,
            )
            .map_err(|e| unreachable!("we always get 16 characters. but err: {e}"))?;

            BoxType::Uuid(chars)
        }

        // alright, we've just got a normal box type.
        //
        // we'll map it into `c_char`.
        //
        // since we're not exposing these values to users, we can keep the
        // ASCII format for a (very modest) perf boost lol
        other => BoxType::Id(other.to_be_bytes()),
    };

    // note: we could perform some `FullBox` parsing now, but that'd require
    // a mapping of `type` to `has_full_box: bool`.
    //
    // ...and we don't really need it for this library lol
    //
    // so yea, we'll just return a naive `Box`...
    Ok(BoxHeader {
        header_len: (start_len.saturating_sub(input.len())) as u8,
        box_size: size,
        box_type: ty,
    })
}
