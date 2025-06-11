//! This module contains an IPTC parser for members of the ISO base media file
//! format (ISOBMFF), or just "BMFF."
//!
//! BMFF contains information used for "timed" presentation of media data.[^1]
//!
//! [^1]: [An Overview of the ISO Base Media File Format by Thomas Stockhammer](https://www.youtube.com/watch?v=CLvR9FVYwWs?t=129)

use core::ffi::c_char;

/// A box's header says:
///
/// - what "type" it is (might be UUID)
/// - how large it is
/// - and, optionally, a UUID
#[derive(Debug)]
struct BoxHeader {
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
            BoxSize::Small(n) => Some((n - self.header_len as u32) as u64),
            BoxSize::Large(n) => Some(n - self.header_len as u64),
            BoxSize::Eof => None,
        }
    }
}

/// A BMFF box's type.
#[derive(Debug)]
enum BoxType {
    /// Uses a short ID. No UUID.
    Id([c_char; 4]),

    /// The short ID was b'uuid', so the box's actual type is defined by this
    /// UUID.
    Uuid([c_char; 16]),
}

/// The size of a box.
#[derive(Debug)]
enum BoxSize {
    /// The box is small. u32::MAX is its maximum length.
    Small(u32),

    /// We got a big box of 64-bit size!
    Large(u64),

    /// This is the last box in the file, so it continues until the very end of
    /// the file.
    Eof,
}
