use crate::providers::png::{
    chunks::{Chunk, ChunkContext, PngChunkHeader},
    error::{PngConstructionError, PngWriteError},
};

/// The cHRM chunk has info about chromaticities and the white point.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Chrm {
    header: PngChunkHeader,

    // note: these are (x, y) pairs. saves some space/typing.
    white_point: (u32, u32),
    red: (u32, u32),
    green: (u32, u32),
    blue: (u32, u32),
}

impl Chunk for Chrm {
    const TYPE: [u8; 4] = *b"cHRM";

    fn read(
        blob: &mut &[u8],
        header: PngChunkHeader,
        context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        // check that there's no context
        debug_assert_eq!(context, ChunkContext::Other);

        // grab white point
        let white_point: (u32, u32) = (
            Self::read_u32(blob, "white_point_x")?,
            Self::read_u32(blob, "white_point_y")?,
        );

        // grab red
        let red: (u32, u32) = (
            Self::read_u32(blob, "red_x")?,
            Self::read_u32(blob, "red_y")?,
        );

        // green
        let green: (u32, u32) = (
            Self::read_u32(blob, "green_x")?,
            Self::read_u32(blob, "green_y")?,
        );

        // blue
        let blue: (u32, u32) = (
            Self::read_u32(blob, "blue_x")?,
            Self::read_u32(blob, "blue_y")?,
        );

        Ok(Self {
            header,
            white_point,
            red,
            green,
            blue,
        })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        // write the header
        self.header.write(buf)?;

        // white point
        Self::write_u32(self.white_point.0, buf, "white_point_x")?;
        Self::write_u32(self.white_point.1, buf, "white_point_y")?;

        // red
        Self::write_u32(self.red.0, buf, "red_x")?;
        Self::write_u32(self.red.1, buf, "red_y")?;

        // green
        Self::write_u32(self.green.0, buf, "green_x")?;
        Self::write_u32(self.green.1, buf, "green_y")?;

        // blue
        Self::write_u32(self.blue.0, buf, "blue_x")?;
        Self::write_u32(self.blue.1, buf, "blue_y")?;

        Ok(())
    }
}
