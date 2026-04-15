use crate::providers::png::{
    PngConstructionError,
    chunks::{ChunkContext, PngChunkHeader},
    error::PngWriteError,
};

/// A collection of palettes.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Plte {
    header: PngChunkHeader,
    entries: Vec<Palette>,
}

/// The maximum number of palettes a `PLTE` chunk may carry.
pub const MAX_PALETTES: u32 = 256_u32;

impl super::Chunk for Plte {
    const TYPE: [u8; 4] = *b"PLTE";

    fn read(
        blob: &mut &[u8],
        header: super::PngChunkHeader,
        _context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        // check if chunk len is messed up
        if !header.chunk_length.is_multiple_of(3) || blob.is_empty() {
            log::error!(
                "PLTE chunk length is not a multiple of 3! \
                Can't continue parsing it..."
            );
            return Err(PngConstructionError::PlteNotMultipleOfThree {
                found_chunk_length: header.chunk_length,
            });
        }

        // check if we've got too many palettes
        let palette_count: u32 = header.chunk_length / 3;
        if palette_count > MAX_PALETTES {
            log::error!("PLTE chunk had too many palettes! ({palette_count})");
            return Err(PngConstructionError::PlteTooManyPalettes {
                palette_ct: palette_count,
            });
        }

        // nope, all good! write each palette
        let mut entries: Vec<Palette> = Vec::with_capacity(palette_count as usize);
        for _ in 0..palette_count {
            entries.push(Palette {
                red: Self::read_u8(blob, "red")?,
                green: Self::read_u8(blob, "green")?,
                blue: Self::read_u8(blob, "blue")?,
            });
        }

        Ok(Self { header, entries })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        // check palette ct before writing
        if self.entries.is_empty() || self.entries.len() as u32 > MAX_PALETTES {
            return Err(PngWriteError::PltePaletteCount {
                palette_ct: self.entries.len() as u32,
            });
        }

        // write header
        self.header.write(buf)?;

        // write the palettes
        for Palette { red, green, blue } in &self.entries {
            Self::write_u8(*red, buf, "red")?;
            Self::write_u8(*green, buf, "green")?;
            Self::write_u8(*blue, buf, "blue")?;
        }

        Ok(())
    }
}

/// A color.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Palette {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}
