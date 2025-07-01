use crate::exif::primitives::PrimitiveTy;

macro_rules! create_known_fields_enum {
    ($($variant_ident:ident = $variant_tag:expr,)+) => {
        /// A list of all known Exif fields.
        #[repr(u16)]
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
        pub enum KnownField {
            $(
              $variant_ident = $variant_tag,
            )+
        }

        impl core::convert::TryFrom<u16> for KnownField {
            type Error = ();

            fn try_from(value: u16) -> Result<Self, Self::Error> {
                    $( $variant_tag => Ok(KnownField::$variant_ident), )+
                    _ => Err(()),
                }
            }
        }
    }
}

create_known_fields_enum! {
/*
     *
     *
     *  TIFF Rev. 6.0 Attribute List
     *
     *
     */
    //
    // image data structure
    ImageWidth = 256,
    ImageLength = 257,
    BitsPerSample = 258,
    Compression = 259,
    PhotometricInterpretation = 262,
    Orientation = 274,
    SamplesPerPixel = 277,
    XResolution = 282,
    YResolution = 283,
    PlanarConfiguration = 284,
    ResolutionUnit = 296,
    YCbCrSubSampling = 530,
    YCbCrPositioning = 531,

    //
    // recording offset
    StripOffsets = 273,
    RowsPerStrip = 278,
    StripByteCounts = 279,
    JPEGInterchangeFormat = 513,
    JPEGInterchangeFormatLength = 514,

    //
    // image data characteristics
    TransferFunction = 301,
    WhitePoint = 318,
    PrimaryChromaticities = 319,
    YCbCrCoefficients = 529,
    ReferenceBlackWhite = 532,

    //
    // other tags
    ImageDescription = 270,
    Make = 271,
    Model = 272,
    Software = 305,
    DateTime = 306,
    Artist = 315,
    Copyright = 33432,
}

impl core::convert::From<KnownField> for u16 {
    fn from(tag: KnownField) -> Self {
        tag as u16
    }
}

impl KnownField {
    /// Grabs a field's tag name as defined in the standard.
    ///
    /// ```
    /// use raves_metadata_types::exif::parse_table::KnownField;
    ///
    /// let image_width: KnownField = KnownField::ImageWidth;
    /// assert_eq!(image_width.tag_name(), "Image width");
    /// ```
    pub const fn tag_name(&self) -> &'static str {
        match self {
            /*
             *
             *
             *  TIFF Rev. 6.0 Attribute List
             *
             *
             */
            //
            // image data structure
            KnownField::ImageWidth => "Image width",
            KnownField::ImageLength => "Image height",
            KnownField::BitsPerSample => "Number of bits per component",
            KnownField::Compression => "Compression scheme",
            KnownField::PhotometricInterpretation => "Pixel composition",
            KnownField::Orientation => "Orientation of image",
            KnownField::SamplesPerPixel => "Number of components",
            KnownField::XResolution => "Image resolution in width direction",
            KnownField::YResolution => "Image resolution in height direction",
            KnownField::PlanarConfiguration => "Image data arrangement",
            KnownField::ResolutionUnit => "Unit of X and Y resolution",
            KnownField::YCbCrSubSampling => "Subsampling ratio of Y to C",
            KnownField::YCbCrPositioning => "Y and C positioning",

            //
            // recording offset
            KnownField::StripOffsets => "Offset to strip",
            KnownField::RowsPerStrip => "Number of rows per strip",
            KnownField::StripByteCounts => "Bytes per compressed strip",
            KnownField::JPEGInterchangeFormat => "Offset to JPEG SOI",
            KnownField::JPEGInterchangeFormatLength => "Bytes of JPEG data",

            //
            // image data characteristics
            KnownField::TransferFunction => "Transfer function",
            KnownField::WhitePoint => "White point chromaticity",
            KnownField::PrimaryChromaticities => "Chromaticities of primaries",
            KnownField::YCbCrCoefficients => "Color space transformation matrix coefficients",
            KnownField::ReferenceBlackWhite => "Pair of black and white reference values",

            //
            // other tags
            KnownField::ImageDescription => "Description of Image",
            KnownField::Make => "Image input equipment manufacturer",
            KnownField::Model => "Image input equipment model",
            KnownField::Software => "Software used",
            KnownField::DateTime => "File change date and time",
            KnownField::Artist => "Person who created the image",
            KnownField::Copyright => "Copyright holder",
        }
    }

    /// Returns this field's tag ID.
    ///
    /// ```
    /// use raves_metadata_types::exif::parse_table::KnownField;
    ///
    /// let image_width: KnownField = KnownField::ImageWidth;
    /// assert_eq!(image_width.tag_id(), 256_u16);
    /// ```
    pub const fn tag_id(&self) -> u16 {
        *self as u16
    }

    /// Returns the type(s) this field may take.
    ///
    /// ```
    /// use raves_metadata_types::exif::{
    ///     parse_table::KnownField,
    ///     primitives::PrimitiveTy,
    /// };
    ///
    /// let image_width: KnownField = KnownField::ImageWidth;
    /// assert_eq!(image_width.types(), &[PrimitiveTy::Short, PrimitiveTy::Long]);
    /// ```
    pub const fn types(&self) -> &'static [PrimitiveTy] {
        use super::PrimitiveTy as Pt;

        match self {
            /*
             *
             *
             *  TIFF Rev. 6.0 Attribute List
             *
             *
             */
            //
            // image data structure
            KnownField::ImageWidth => &[Pt::Short, Pt::Long],
            KnownField::ImageLength => &[Pt::Short, Pt::Long],
            KnownField::BitsPerSample => &[Pt::Short],
            KnownField::Compression => &[Pt::Short],
            KnownField::PhotometricInterpretation => &[Pt::Short],
            KnownField::Orientation => &[Pt::Short],
            KnownField::SamplesPerPixel => &[Pt::Short],
            KnownField::XResolution => &[Pt::Rational],
            KnownField::YResolution => &[Pt::Rational],
            KnownField::PlanarConfiguration => &[Pt::Short],
            KnownField::ResolutionUnit => &[Pt::Short],
            KnownField::YCbCrSubSampling => &[Pt::Short],
            KnownField::YCbCrPositioning => &[Pt::Short],

            //
            // recording offset
            KnownField::StripOffsets => &[Pt::Short, Pt::Long],
            KnownField::RowsPerStrip => &[Pt::Short, Pt::Long],
            KnownField::StripByteCounts => &[Pt::Short, Pt::Long],
            KnownField::JPEGInterchangeFormat => &[Pt::Long],
            KnownField::JPEGInterchangeFormatLength => &[Pt::Long],

            //
            // image data characteristics
            KnownField::TransferFunction => &[Pt::Short],
            KnownField::WhitePoint => &[Pt::Rational],
            KnownField::PrimaryChromaticities => &[Pt::Rational],
            KnownField::YCbCrCoefficients => &[Pt::Rational],
            KnownField::ReferenceBlackWhite => &[Pt::Rational],

            //
            // other tags
            KnownField::ImageDescription => &[PrimitiveTy::Ascii, PrimitiveTy::Utf8],
            KnownField::Make => &[PrimitiveTy::Ascii, PrimitiveTy::Utf8],
            KnownField::Model => &[PrimitiveTy::Ascii, PrimitiveTy::Utf8],
            KnownField::Software => &[PrimitiveTy::Ascii, PrimitiveTy::Utf8],
            KnownField::DateTime => &[PrimitiveTy::Ascii],
            KnownField::Artist => &[PrimitiveTy::Ascii, PrimitiveTy::Utf8],
            KnownField::Copyright => &[PrimitiveTy::Ascii, PrimitiveTy::Utf8],
        }
    }

    /// Returns the number of primitives this field may have.
    ///
    /// ```
    /// use raves_metadata_types::exif::parse_table::{
    ///     KnownField,
    ///     PrimitiveCount,
    /// };
    ///
    /// let image_width: KnownField = KnownField::ImageWidth;
    /// assert_eq!(image_width.count(), PrimitiveCount::Known(1));
    /// ```
    pub const fn count(&self) -> PrimitiveCount {
        use PrimitiveCount as Pc;

        match self {
            /*
             *
             *
             *  TIFF Rev. 6.0 Attribute List
             *
             *
             */
            //
            // image data structure
            KnownField::ImageWidth | KnownField::ImageLength => Pc::Known(1),
            KnownField::BitsPerSample => Pc::Known(3),
            KnownField::Compression
            | KnownField::PhotometricInterpretation
            | KnownField::Orientation
            | KnownField::SamplesPerPixel
            | KnownField::XResolution
            | KnownField::YResolution
            | KnownField::PlanarConfiguration
            | KnownField::ResolutionUnit => Pc::Known(1),
            KnownField::YCbCrSubSampling => Pc::Known(2),
            KnownField::YCbCrPositioning => Pc::Known(1),

            //
            // recording offset
            KnownField::StripOffsets => Pc::SpecialHandling,
            KnownField::RowsPerStrip => Pc::Known(1),
            KnownField::StripByteCounts => Pc::SpecialHandling,
            KnownField::JPEGInterchangeFormat | KnownField::JPEGInterchangeFormatLength => {
                Pc::Known(1)
            }

            //
            // image data characteristics
            KnownField::TransferFunction => Pc::Known(3 * 256),
            KnownField::WhitePoint => Pc::Known(2),
            KnownField::PrimaryChromaticities => Pc::Known(6),
            KnownField::YCbCrCoefficients => Pc::Known(3),
            KnownField::ReferenceBlackWhite => Pc::Known(6),

            //
            // other tags
            KnownField::ImageDescription
            | KnownField::Make
            | KnownField::Model
            | KnownField::Software => Pc::Any,
            KnownField::DateTime => Pc::Known(20),
            KnownField::Artist | KnownField::Copyright => Pc::Any,
        }
    }
}

/// The number of primitives a field should have.
///
/// These are used to sanity-check parsed values.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum PrimitiveCount {
    /// There are `n` primitives.
    Known(u32),

    /// The number of primitives is within this range, inclusive.
    KnownRange { lower: u32, upper: u32 },

    /// This field requires special handling for its count.
    ///
    /// For instance, `StripOffsets` has a variable count based on the value
    /// of `RowsPerStrip`.
    ///
    /// So, we can't quite know the count beforehand. It's better to just ask
    /// the parser to do some special handling for such fields.
    SpecialHandling,

    /// Any number of primitives.
    Any,
}
