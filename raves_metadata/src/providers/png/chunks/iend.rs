use crate::providers::png::{
    chunks::{Chunk, ChunkContext, PngChunkHeader},
    error::{PngConstructionError, PngWriteError},
};

/// A chunk stating that the PNG datastream has ended.
///
/// It has no associated data -- just the header.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Iend {
    header: PngChunkHeader,
}

impl Chunk for Iend {
    const TYPE: [u8; 4] = *b"IEND";

    fn read(
        _blob: &mut &[u8],
        header: super::PngChunkHeader,
        _context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        // this chunk must be zero length. so let's check that!
        if header.chunk_length != 0_u32 {
            log::error!("IEND chunk has associated data! That's not allowed.");
            return Err(PngConstructionError::IendHadData {
                chunk_length: header.chunk_length,
            });
        }

        // all good! just store its header
        Ok(Self { header })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        // write header
        self.header.write(buf)?;
        Ok(())
    }
}
