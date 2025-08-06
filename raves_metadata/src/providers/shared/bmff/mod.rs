//! This module contains helpers for members of the ISO base media file
//! format (ISOBMFF), or just "BMFF."
//!
//! BMFF contains information used for "timed" presentation of media data.[^1]
//!
//! [^1]: [An Overview of the ISO Base Media File Format by Thomas Stockhammer](https://www.youtube.com/watch?v=CLvR9FVYwWs?t=129)

use winnow::{
    binary::{be_u32, be_u64},
    error::ContextError,
    prelude::*,
    token::take,
};

/// Finds the next header in the file and parses it out.
///
/// This function assumes the byte slice starts at a header. This means you
/// should pass it a byte slice with the previous header's offset applied, or
/// the file is starting from the beginning.
///
/// Note that the input is mutated - skip `size - taken_bytes`.
pub fn parse_header(input: &mut &[u8]) -> ModalResult<BoxHeader, ContextError> {
    // we're going to track the length of our box as we parse.
    //
    // the amount of bytes we took is given in the `BoxHeader`
    let start_len = input.len();

    // grab the raw "size" and "type" from the input.
    //
    // we parse these more below...
    let raw_size: u32 = be_u32.parse_next(input)?;
    let raw_type: u32 = be_u32.parse_next(input)?;

    // parse the size into something more usable
    let size: BoxSize = match raw_size {
        // special case: we have a largesize to parse.
        //
        // and, well, we've already parsed the first two parts of this header.
        // let's also take the `large_size`, which comes next. it's a `u64`...
        1_u32 => BoxSize::Large(be_u64.parse_next(input)?),

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
            let chars: [u8; LEN] = TryInto::<[u8; LEN]>::try_into(take(LEN).parse_next(input)?)
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
        header_len: (start_len - input.len()) as u8,
        box_size: size,
        box_type: ty,
    })
}

/// A box's header says:
///
/// - what "type" it is (might be UUID)
/// - how large it is
/// - and, optionally, a UUID
#[derive(Debug)]
pub struct BoxHeader {
    /// How long the header is.
    pub header_len: u8,

    /// How large the box is. This includes the header's size.
    pub box_size: BoxSize,

    /// The box's type.
    pub box_type: BoxType,
}

impl BoxHeader {
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
}

/// A BMFF box's type.
#[derive(Debug)]
pub enum BoxType {
    /// Uses a short ID. No UUID.
    Id([u8; 4]),

    /// The short ID was b'uuid', so the box's actual type is defined by this
    /// UUID.
    Uuid([u8; 16]),
}

/// The size of a box.
#[derive(Debug)]
pub enum BoxSize {
    /// The box is small. u32::MAX is its maximum length.
    Small(u32),

    /// We got a big box of 64-bit size!
    Large(u64),

    /// This is the last box in the file, so it continues until the very end of
    /// the file.
    Eof,
}
