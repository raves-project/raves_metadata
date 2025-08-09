//! This module contains helpers for members of the ISO base media file
//! format (ISOBMFF), or just "BMFF."
//!
//! BMFF contains information used for "timed" presentation of media data.[^1]
//!
//! [^1]: [An Overview of the ISO Base Media File Format by Thomas Stockhammer](https://www.youtube.com/watch?v=CLvR9FVYwWs?t=129)

use std::fmt::Write as _;

pub use box_header::BoxHeader;

mod box_header;
pub mod ftyp;

/// The box UUID used for XMP.
pub const XMP_UUID: [u8; 16] = [
    0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF, 0xAC,
];

/// The box ID sometimes used for XMP, particularly in QuickTime.
pub const XMP_BOX_ID: [u8; 4] = *b"XMP_";

/// A BMFF box's type.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum BoxType {
    /// Uses a short ID. No UUID.
    Id([u8; 4]),

    /// The short ID was b'uuid', so the box's actual type is defined by this
    /// UUID.
    Uuid([u8; 16]),
}

impl core::fmt::Debug for BoxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(id) => {
                f.write_str("Id(\"")?;
                for u in id {
                    f.write_char(*u as char)?;
                }
                f.write_char('"')?;
                f.write_char(')')
            }

            Self::Uuid(uuid) => {
                f.write_str("Id(\"")?;
                for u in uuid {
                    f.write_char(*u as char)?;
                }
                f.write_char('"')?;
                f.write_char(')')
            }
        }
    }
}

/// The size of a box.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum BoxSize {
    /// The box is small. u32::MAX is its maximum length.
    Small(u32),

    /// We got a big box of 64-bit size!
    Large(u64),

    /// This is the last box in the file, so it continues until the very end of
    /// the file.
    Eof,
}
