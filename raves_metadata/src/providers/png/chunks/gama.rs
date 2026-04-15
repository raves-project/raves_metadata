use crate::providers::png::{
    chunks::{Chunk, ChunkContext, PngChunkHeader},
    error::{PngConstructionError, PngWriteError},
};

/// The gAMA chunk provides a basic gamma (brightness) value.
///
/// Other chunks do this better -- this chunk is pretty basic.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Gama {
    header: PngChunkHeader,
    gamma: u32,
}

impl Chunk for Gama {
    const TYPE: [u8; 4] = *b"gAMA";

    fn read(
        blob: &mut &[u8],
        header: PngChunkHeader,
        context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        // check that there's no context
        debug_assert_eq!(context, ChunkContext::Other);

        // grab gamma value
        let gamma: u32 = Self::read_u32(blob, "gamma")?;
        Ok(Self { header, gamma })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        // write the header, then the gamma
        self.header.write(buf)?;
        Self::write_u32(self.gamma, buf, "gamma")?;

        Ok(())
    }
}
