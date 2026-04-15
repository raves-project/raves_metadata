use winnow::{Parser, error::EmptyError, token::take};

use crate::providers::png::{
    PngConstructionError,
    chunks::{Chunk, ChunkContext, PngChunkHeader},
    error::PngWriteError,
};

/// A chunk of image data.
///
/// Compressed and unaltered -- this library isn't a decoder or encoder.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Idat {
    header: PngChunkHeader,
    data: Vec<u8>,
}

impl Chunk for Idat {
    const TYPE: [u8; 4] = *b"IDAT";

    fn read(
        blob: &mut &[u8],
        header: super::PngChunkHeader,
        _context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        let mut entries: Vec<u8> = Vec::with_capacity(header.chunk_length as usize);

        let slice_len: usize = blob.len();
        let slice: &[u8] =
            take(header.chunk_length)
                .parse_next(blob)
                .map_err(|_e: EmptyError| {
                    log::error!(
                        "When parsing `IDAT` chunk, failed to read all {} bytes.",
                        header.chunk_length
                    );
                    PngConstructionError::OuttaBytes {
                        chunk_type: Idat::TYPE_STR,
                        expected: header.chunk_length,
                        remaining: slice_len as u32,
                    }
                })?;

        entries.extend_from_slice(slice);

        Ok(Self {
            header,
            data: entries,
        })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        // write header
        self.header.write(buf)?;

        // write all the bytes
        Self::write_byte_slice(&self.data, buf, "data")?;
        Ok(())
    }
}
