//! Includes support for all the different kinds of chunks in a PNG image.
//!
//! Each chunk here has the ability to be read and written.

use winnow::{Parser, binary::be_u32, error::EmptyError, token::take};

use crate::providers::png::{PngConstructionError, error::PngWriteError};

pub mod chrm;
pub mod gama;
pub mod iccp;
pub mod idat;
pub mod iend;
pub mod ihdr;
pub mod plte;
pub mod trns;

/// A "chunk" of PNG data.
pub trait Chunk: Sized {
    /// The "type" identifier for a PNG chunk.
    ///
    /// Each chunk has a unique type, meaning that this value identifies each
    /// chunk exactly.
    const TYPE: [u8; 4];

    /// A printable, stringy version of the chunk type.
    ///
    /// Used for error messages and logging.
    const TYPE_STR: &str = const {
        // convert the `TYPE` byte array into a string.
        //
        // note that we have to handle the option that it's not UTF-8, but we
        // do so in `const`, meaning that a panic here can only happen at
        // compile-time ;D
        let Ok(type_str) = core::str::from_utf8(&Self::TYPE) else {
            panic!()
        };

        type_str
    };

    /// Reads this `Chunk` from the `blob`, given a chunk header that's already
    /// been parsed out of the blob.
    fn read(
        blob: &mut &[u8],
        header: PngChunkHeader,
        context: ChunkContext,
    ) -> Result<Self, PngConstructionError>;

    /// Writes this chunk into the given buffer, `buf`.
    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError>;

    /// Reads a byte from the given `blob`.
    fn read_u8(blob: &mut &[u8], field: &'static str) -> Result<u8, PngConstructionError> {
        winnow::binary::u8
            .parse_next(blob)
            .map_err(|_e: EmptyError| {
                log::error!(
                    "Can't get `u8`! Outta bytes when attempting to read chunk: `{}` \
                    for field: `{field}`.",
                    Self::TYPE_STR
                );
                PngConstructionError::OuttaBytes {
                    chunk_type: Self::TYPE_STR,
                    expected: 1_u32,
                    remaining: 0_u32,
                }
            })
    }

    /// Writes a byte to the given `buf`.
    fn write_u8<W: std::io::Write>(
        value: u8,
        buf: &mut W,
        field: &'static str,
    ) -> Result<(), PngWriteError> {
        buf.write_all(&[value]).map_err(|_| {
            log::error!(
                "Failed to write `u8` value to buffer! \
                Buffer type: {}, \
                Chunk type: {}, \
                Field: {field}",
                core::any::type_name_of_val(buf),
                Self::TYPE_STR
            );
            PngWriteError::CantWriteToBuf {}
        })
    }

    /// Reads a `u16` from the given `blob`.
    fn read_u16(blob: &mut &[u8], field: &'static str) -> Result<u16, PngConstructionError> {
        winnow::binary::be_u16
            .parse_next(blob)
            .map_err(|_e: EmptyError| {
                log::error!(
                    "Can't get `u16`! Outta bytes when attempting to read chunk: `{}` \
                    for field: `{field}`.",
                    Self::TYPE_STR
                );
                PngConstructionError::OuttaBytes {
                    chunk_type: Self::TYPE_STR,
                    expected: 2_u32,
                    remaining: blob.len() as u32,
                }
            })
    }

    /// Writes a `u16` to the given `buf`.
    fn write_u16<W: std::io::Write>(
        value: u16,
        buf: &mut W,
        field: &'static str,
    ) -> Result<(), PngWriteError> {
        buf.write_all(&value.to_be_bytes()).map_err(|_| {
            log::error!(
                "Failed to write `u16` value to buffer! \
                Buffer type: {}, \
                Chunk type: {}, \
                Field: {field}",
                core::any::type_name_of_val(buf),
                Self::TYPE_STR
            );
            PngWriteError::CantWriteToBuf {}
        })
    }

    /// Reads a `u32` from the given `blob`.
    fn read_u32(blob: &mut &[u8], field: &'static str) -> Result<u32, PngConstructionError> {
        winnow::binary::be_u32
            .parse_next(blob)
            .map_err(|_e: EmptyError| {
                log::error!(
                    "Can't get `u32`! Outta bytes when attempting to read chunk: `{}` \
                    for field: `{field}`.",
                    Self::TYPE_STR
                );
                PngConstructionError::OuttaBytes {
                    chunk_type: Self::TYPE_STR,
                    expected: 4_u32,
                    remaining: blob.len() as u32,
                }
            })
    }

    /// Writes a `u32` to the given `buf`.
    fn write_u32<W: std::io::Write>(
        value: u32,
        buf: &mut W,
        field: &'static str,
    ) -> Result<(), PngWriteError> {
        buf.write_all(&value.to_be_bytes()).map_err(|_| {
            log::error!(
                "Failed to write `u32` value to buffer! \
                Buffer type: {}, \
                Chunk type: {}, \
                Field: {field}",
                core::any::type_name_of_val(buf),
                Self::TYPE_STR
            );
            PngWriteError::CantWriteToBuf {}
        })
    }

    /// Writes a byte slice to the given buffer.
    fn write_byte_slice<W: std::io::Write>(
        value: &[u8],
        buf: &mut W,
        field: &'static str,
    ) -> Result<(), PngWriteError> {
        buf.write_all(value).map_err(|_| {
            log::error!(
                "Failed to write byte slice to buffer! \
                Buffer type: {}, \
                Chunk type: {}, \
                Field: {field}",
                core::any::type_name_of_val(buf),
                Self::TYPE_STR
            );
            PngWriteError::CantWriteToBuf {}
        })
    }
}

/// Context relating to parsing/writing a chunk.
///
/// Note that the 'par lifetime is related to parser borrows -- meaning the
/// crate's internal parser provides the data.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum ChunkContext<'par> {
    /// The primary transparency info chunk.
    Trns(&'par trns::TrnsContext),

    /// This chunk doesn't take context.
    Other,
}

/// A header for a PNG chunk.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct PngChunkHeader {
    pub chunk_length: u32,    // in bytes
    pub chunk_ident: [u8; 4], // four ascii letters
}

impl PngChunkHeader {
    /// Reads the chunk header from the blob (byte slice).
    pub fn read(blob: &mut &[u8]) -> Result<Self, PngConstructionError> {
        let chunk_length: u32 = be_u32.parse_next(blob).map_err(|_e: EmptyError| {
            log::error!(
                "Chunk length couldn't be parsed from the blob. \
                    Outta bytes to consume!"
            );
            PngConstructionError::NotEnoughBytesForChunkHeader
        })?;

        let chunk_ident: [u8; 4] = take(4_usize)
            .parse_next(blob)
            .map_err(|_e: EmptyError| {
                log::error!(
                    "Chunk identifier couldn't be parsed from the blob. \
                        Outta bytes to consume!"
                );
                PngConstructionError::NotEnoughBytesForChunkHeader
            })?
            .try_into()
            .unwrap_or_else(|e| {
                unreachable!("winnow already said this must be 4 bytes. but err: {e}")
            });

        Ok(PngChunkHeader {
            chunk_length,
            chunk_ident,
        })
    }

    /// Writes a chunk header to the buffer, `buf`.
    pub fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        buf.write_all(&self.chunk_length.to_be_bytes())
            .map_err(|_| {
                log::error!("Failed to write chunk length!");
                PngWriteError::CantWriteToBuf {}
            })?;

        buf.write_all(&self.chunk_ident).map_err(|_| {
            log::error!("Failed to write chunk length!");
            PngWriteError::CantWriteToBuf {}
        })?;

        Ok(())
    }
}
