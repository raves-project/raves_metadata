use crate::exif::primitives::PrimitiveTy;

/// Creates the entire `KnownFields` enum from all its variants.
///
/// However, this macro also allows moving lots of data into one place, which
/// is nice for maintenance!
macro_rules! create_known_fields_enum {
    ($( $variant_ident:ident = $variant_tag:expr => {
        name: $tag_name:expr,
        types: $types:expr,
        count: $count:expr,
    }, )+) => {
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
                match value {
                    $( $variant_tag => Ok(KnownField::$variant_ident), )+
                    _ => Err(()),
                }
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
                    $( KnownField::$variant_ident => $tag_name, )+
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
                match self {
                    $( KnownField::$variant_ident => $types, )+
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
                match self {
                    $( KnownField::$variant_ident => $count, )+
                }
            }
        }
    }
}

use {PrimitiveCount as Pc, PrimitiveTy as Pt};

create_known_fields_enum! {
    /*
     *
     *
     *
     *
     *
     *
     *
     *  TIFF Rev. 6.0 Attribute List
     *
     *
     *
     *
     *
     *
     *
     *
     */
    //
    // image data structure
    ImageWidth = 256 => {
        name: "Image width",
        types: &[Pt::Short, Pt::Long],
        count: Pc::Known(1),
    },
    ImageLength = 257 => {
        name: "Image height",
        types: &[Pt::Short, Pt::Long],
        count: Pc::Known(1),
    },
    BitsPerSample = 258 => {
        name: "Number of bits per component",
        types: &[Pt::Short],
        count: Pc::Known(3),
    },
    Compression = 259 => {
        name: "Compression scheme",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    PhotometricInterpretation = 262 => {
        name: "Pixel composition",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    Orientation = 274 => {
        name: "Orientation of image",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    SamplesPerPixel = 277 => {
        name: "Number of components",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    XResolution = 282 => {
        name: "Image resolution in width direction",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    YResolution = 283 => {
        name: "Image resolution in height direction",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    PlanarConfiguration = 284 => {
        name: "Image data arrangement",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    ResolutionUnit = 296 => {
        name: "Unit of X and Y resolution",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    YCbCrSubSampling = 530 => {
        name: "Subsampling ratio of Y to C",
        types: &[Pt::Short],
        count: Pc::Known(2),
    },
    YCbCrPositioning = 531 => {
        name: "Y and C positioning",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },

    //
    // recording offset
    StripOffsets = 273 => {
        name: "Offset to strip",
        types: &[Pt::Short, Pt::Long],
        count: Pc::SpecialHandling,
    },
    RowsPerStrip = 278 => {
        name: "Number of rows per strip",
        types: &[Pt::Short, Pt::Long],
        count: Pc::Known(1),
    },
    StripByteCounts = 279 => {
        name: "Bytes per compressed strip",
        types: &[Pt::Short, Pt::Long],
        count: Pc::SpecialHandling,
    },
    JPEGInterchangeFormat = 513 => {
        name: "Offset to JPEG SOI",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    JPEGInterchangeFormatLength = 514 => {
        name: "Bytes of JPEG data",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },

    //
    // image data characteristics
    TransferFunction = 301 => {
        name: "Transfer function",
        types: &[Pt::Short],
        count: Pc::Known(3 * 256),
    },
    WhitePoint = 318 => {
        name: "White point chromaticity",
        types: &[Pt::Rational],
        count: Pc::Known(2),
    },
    PrimaryChromaticities = 319 => {
        name: "Chromaticities of primaries",
        types: &[Pt::Rational],
        count: Pc::Known(6),
    },
    YCbCrCoefficients = 529 => {
        name: "Color space transformation matrix coefficients",
        types: &[Pt::Rational],
        count: Pc::Known(3),
    },
    ReferenceBlackWhite = 532 => {
        name: "Pair of black and white reference values",
        types: &[Pt::Rational],
        count: Pc::Known(6),
    },

    //
    // other tags
    ImageDescription = 270 => {
        name: "Description of Image",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    Make = 271 => {
        name: "Image input equipment manufacturer",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    Model = 272 => {
        name: "Image input equipment model",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    Software = 305 => {
        name: "Software used",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    DateTime = 306 => {
        name: "File change date and time",
        types: &[Pt::Ascii],
        count: Pc::Known(20),
    },
    Artist = 315 => {
        name: "Person who created the image",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    Copyright = 33432 => {
        name: "Copyright holder",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
}

impl core::convert::From<KnownField> for u16 {
    fn from(tag: KnownField) -> Self {
        tag as u16
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
