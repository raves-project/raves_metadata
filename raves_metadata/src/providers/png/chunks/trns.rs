use crate::providers::png::{
    chunks::{Chunk, ChunkContext, PngChunkHeader},
    error::{PngConstructionError, PngWriteError},
};

/// Specifies colors relating to transparency.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Trns {
    header: PngChunkHeader,
    data: TrnsData,
}

/// The data behind the `Trns` type.
///
/// Separated to disallow access/mutation by crate users.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum TrnsData {
    ColorType0 {
        grey_sample_value: u16,
    },

    ColorType2 {
        red_sample_value: u16,
        green_sample_value: u16,
        blue_sample_value: u16,
    },

    ColorType3 {
        alpha_per_palette: Vec<u8>,
    },
}

/// Context given to the tRNS reader.
///
/// Based on the `color_type: ColorType` from the IHDR chunk.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum TrnsContext {
    /// Type 0 doesn't need any info.
    ColorType0,

    /// Neither does type 2.
    ColorType2,

    /// Type 3 needs to know how many entries are in the PLTE chunk.
    ColorType3 { number_of_plte_entries: u32 },

    /// Other color types aren't permitted for tRNS -- error out!
    NotPermitted,
}

impl Chunk for Trns {
    const TYPE: [u8; 4] = *b"tRNS";

    fn read(
        blob: &mut &[u8],
        header: super::PngChunkHeader,
        context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        // ensure context is the right kind
        let ChunkContext::Trns(trns_ctx) = context else {
            log::error!(
                "INTERNAL ERROR: `Trns::read` with wrong type of context! \
                Please create an issue on the `raves-project/raves_metadata` GitHub repo. \
                context: {context:?}"
            );
            std::process::abort();
        };

        // grab data (depending on color type)
        let data: TrnsData = match trns_ctx {
            TrnsContext::ColorType0 => TrnsData::ColorType0 {
                grey_sample_value: Self::read_u16(blob, "grey_sample_value (type 0)")?,
            },

            TrnsContext::ColorType2 => TrnsData::ColorType2 {
                red_sample_value: Self::read_u16(blob, "red_sample_value (type 2)")?,
                green_sample_value: Self::read_u16(blob, "green_sample_value (type 2)")?,
                blue_sample_value: Self::read_u16(blob, "blue_sample_value (type 2)")?,
            },

            TrnsContext::ColorType3 {
                number_of_plte_entries,
            } => {
                let mut v: Vec<_> = Vec::with_capacity(*number_of_plte_entries as usize);
                for _ in 0..*number_of_plte_entries {
                    v.push(Self::read_u8(blob, "alpha (type 3)")?);
                }
                TrnsData::ColorType3 {
                    alpha_per_palette: v,
                }
            }

            // error out on other types
            TrnsContext::NotPermitted => {
                log::error!(
                    "TRNS chunk was found, but the color type (from IHDR) \
                    says that this chunk shouldn't be in the datastream. \
                    Returning an error..."
                );
                return Err(PngConstructionError::TrnsGivenDisallowedType);
            }
        };

        // all good! just store its header
        Ok(Self { header, data })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        // write header
        self.header.write(buf)?;

        // write depending on the data...
        match self.data {
            TrnsData::ColorType0 { grey_sample_value } => {
                Self::write_u16(grey_sample_value, buf, "grey_sample_value (type 0)")?;
            }
            TrnsData::ColorType2 {
                red_sample_value,
                green_sample_value,
                blue_sample_value,
            } => {
                Self::write_u16(red_sample_value, buf, "red_sample_value (type 2)")?;
                Self::write_u16(green_sample_value, buf, "green_sample_value (type 2)")?;
                Self::write_u16(blue_sample_value, buf, "blue_sample_value (type 2)")?;
            }

            TrnsData::ColorType3 {
                ref alpha_per_palette,
            } => {
                for alpha_value in alpha_per_palette.iter() {
                    Self::write_u8(*alpha_value, buf, "alpha_value (type 3)")?;
                }
            }
        }

        Ok(())
    }
}
