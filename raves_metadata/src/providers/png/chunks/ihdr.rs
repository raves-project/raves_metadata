use crate::providers::png::{
    PngConstructionError,
    chunks::{ChunkContext, PngChunkHeader},
    error::PngWriteError,
};

/// The first chunk in the PNG datastream.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Ihdr {
    header: PngChunkHeader,

    width_px: u32,
    height_px: u32,

    bit_depth: BitDepth,
    color_type: ColorType,

    compression_method: CompressionMethod,
    filter_method: FilterMethod,
    interlace_method: InterlaceMethod,
}

impl super::Chunk for Ihdr {
    const TYPE: [u8; 4] = [0x49, 0x48, 0x44, 0x52];

    fn read(
        blob: &mut &[u8],
        header: PngChunkHeader,
        _context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        let width_px: u32 = Self::read_u32(blob, "width")?;
        let height_px: u32 = Self::read_u32(blob, "height")?;

        let bit_depth: BitDepth = BitDepth::new(Self::read_u8(blob, "bit_depth")?)?;
        let color_type: ColorType = ColorType::new(Self::read_u8(blob, "color_type")?, bit_depth)?;

        let compression_method: CompressionMethod =
            CompressionMethod::new(Self::read_u8(blob, "compression_method")?)?;
        let filter_method: FilterMethod = FilterMethod::new(Self::read_u8(blob, "filter_method")?)?;
        let interlace_method: InterlaceMethod =
            InterlaceMethod::new(Self::read_u8(blob, "interlace_method")?)?;

        Ok(Self {
            header,

            width_px,
            height_px,

            bit_depth,
            color_type,

            compression_method,
            filter_method,
            interlace_method,
        })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        let Self {
            header,
            width_px,
            height_px,
            bit_depth,
            color_type,
            compression_method,
            filter_method,
            interlace_method,
        } = self;

        // write the header
        header.write(buf)?;

        // write the sizes
        Self::write_u32(*width_px, buf, "width_px")?;
        Self::write_u32(*height_px, buf, "height_px")?;

        // write the bit/color stuff
        Self::write_u8(*bit_depth as u8, buf, "bit_depth")?;
        Self::write_u8(*color_type as u8, buf, "color_type")?;

        // write *_methods
        Self::write_u8(*compression_method as u8, buf, "compression_method")?;
        Self::write_u8(*filter_method as u8, buf, "filter_method")?;
        Self::write_u8(*interlace_method as u8, buf, "interlace_method")?;

        Ok(())
    }
}

/// The bit depth of the image.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitDepth {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

impl BitDepth {
    /// Attempts to create a new `BitDepth` given a byte value.
    pub fn new(byte: u8) -> Result<Self, PngConstructionError> {
        Ok(match byte {
            1 => BitDepth::One,
            2 => BitDepth::Two,
            4 => BitDepth::Four,
            8 => BitDepth::Eight,
            16 => BitDepth::Sixteen,

            other => {
                return Err(PngConstructionError::DisallowedBitDepth { found_value: other })
                    .inspect_err(|_e| {
                        log::error!("Disallowed bit depth found in IHDR chunk: `{other}`")
                    });
            }
        })
    }
}

/// The color type of the image.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorType {
    Greyscale = 0,
    Truecolor = 2,
    Indexed = 3,
    GreyscaleAlpha = 4,
    TruecolorAlpha = 6,
}

impl ColorType {
    /// Attempts to create a new `ColorType`.
    pub fn new(byte: u8, bit_depth: BitDepth) -> Result<Self, PngConstructionError> {
        let color_type: ColorType = match byte {
            0 => Self::Greyscale,
            2 => Self::Truecolor,
            3 => Self::Indexed,
            4 => Self::GreyscaleAlpha,
            6 => Self::TruecolorAlpha,

            other => {
                return Err(PngConstructionError::DisallowedColorType { found_value: other })
                    .inspect_err(|_e| {
                        log::error!("Disallowed color type found in IHDR chunk: `{other}`")
                    });
            }
        };

        let compatible: bool = match color_type {
            Self::Greyscale => true,
            Self::Truecolor => matches!(bit_depth, BitDepth::Eight | BitDepth::Sixteen),
            Self::Indexed => matches!(
                bit_depth,
                BitDepth::One | BitDepth::Two | BitDepth::Four | BitDepth::Eight
            ),
            Self::GreyscaleAlpha | Self::TruecolorAlpha => {
                matches!(bit_depth, BitDepth::Eight | BitDepth::Sixteen)
            }
        };

        if !compatible {
            return Err(PngConstructionError::BitDepthIncompatibleWithColorType {
                bit_depth,
                incompatible_color_type: color_type,
            });
        }

        Ok(color_type)
    }
}

/// The compression method used to compress the image data.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompressionMethod {
    Deflate = 0,
}

impl CompressionMethod {
    /// Attempts to create a new `CompressionMethod`.
    pub fn new(byte: u8) -> Result<Self, PngConstructionError> {
        if byte == 0 {
            Ok(Self::Deflate)
        } else {
            Err(PngConstructionError::DisallowedCompressionMethod { found_value: byte })
        }
    }
}

/// The filter method used to filter the image data before compression.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FilterMethod {
    Adaptive = 0,
}

impl FilterMethod {
    /// Attempts to create a new `FilterMethod`.
    pub fn new(byte: u8) -> Result<Self, PngConstructionError> {
        if byte == 0 {
            Ok(Self::Adaptive)
        } else {
            Err(PngConstructionError::DisallowedFilterMethod { found_value: byte })
        }
    }
}

/// The interlace method used to interlace the image data, if any.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterlaceMethod {
    None = 0,
    Adam7 = 1,
}

impl InterlaceMethod {
    /// Attempts to create a new `InterlaceMethod`.
    pub fn new(byte: u8) -> Result<Self, PngConstructionError> {
        if byte == 0 {
            Ok(Self::None)
        } else if byte == 7 {
            Ok(Self::Adam7)
        } else {
            Err(PngConstructionError::DisallowedInterlaceMethod { found_value: byte })
        }
    }
}
