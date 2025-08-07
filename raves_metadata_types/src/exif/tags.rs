//! Definitions for tags in an IFD group.
//!
//! # `tags`
//!
//! Contains all the tags stored in an [`IfdGroup`].
//!
//! Tags, which are within IFDs, have a key and >=1 value(s).
//!
//! ## What's this for?
//!
//! This module effectively contains a parse table for an Exif-supporting
//! metadata library.
//!
//! ## For contributors
//!
//! <div class="warning">
//! The rest of this documentation is here to assist contributors to
//! `raves_metadata_types`.
//!
//! It won't be helpful unless you're trying to add support for new tags.
//! </div>
//!
//! ### Adding new groups
//!
//! Expansion with new IFD groups is simple - for each newly supported IFD
//! group, complete the following steps:
//!
//! 1. add the group's name to [`IfdGroup`]
//! 2. create a new call to the `make_key_list_for_group!` macro
//! 3. point it toward the `IfdGroup::NewIfd` you've made
//!     - name the enum something like `NewIfdKey`
//!     - that'll look something like: `enum NewIfdKey => IfdGroup::NewIfd`
//! 4. add all the available tags
//!     - i.e., define keys and their value types
//! 5. add a new variant on [`KnownTag`]
//! 6. fill in the [`KnownTag`] methods by forwarding to `NewGroup`
//!
//! ### Adding new tags
//!
//! Let's say we want to add a new tag to `IfdGroup::NotReal`. Let's call it
//! `YourNewKey`; please assume it has a tag ID of `1000`, uses type text, and
//! only appears once.
//!
//! Under the `make_key_list_for_group` macro for `IfdGroup::NotReal`, add a
//! new listing:
//!
//! ```no_compile
//! make_key_list_for_group!(enum NotRealKey => IfdGroup::NotReal,
//!     // ...snip!
//!     // other tags would already be here.
//!     // ...
//!
//!     YourNewKey = 1000 => {
//!         name: "Your New Key",
//!         types: &[Pt::Text],
//!         count: Pc::Known(1),
//!     },
//! );
//! ```

use crate::exif::{
    FieldTag,
    ifd::IfdGroup,
    primitives::{PrimitiveCount, PrimitiveTy},
};

/// Creates a "key list" for an IFD group.
///
/// These may include duplicate tag IDs from other groups.
macro_rules! make_key_list_for_group {
    (enum $enum_name:ident => $ifd_group:expr,
        $( $key_ident:ident = $key_tag:expr => {
            name: $tag_name:expr,
            types: $types:expr,
            count: $count:expr,
        },
    )+) => {
        #[doc = "A list of all keys present in the matching `IfdGroup` variant."]
        #[repr(u16)]
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
        pub enum $enum_name {
            $(
              $key_ident = $key_tag,
            )+
        }

        impl $enum_name {
            /// Returns the number of primitives this key's value may store.
            pub const fn count(&self) -> PrimitiveCount {
                match self {
                    $( Self::$key_ident => $count, )+
                }
            }

            /// Returns the `IfdGroup` that this enum represents.
            pub const fn ifd_group() -> IfdGroup {
                $ifd_group
            }

            /// Returns this key's tag ID.
            pub const fn tag_id(&self) -> u16 {
                *self as u16
            }

            /// Grabs a key's tag name as defined in the standard.
            pub const fn tag_name(&self) -> &'static str {
                match self {
                    $( Self::$key_ident => $tag_name, )+
                }
            }


            /// Returns the type(s) this key's value may have.
            pub const fn types(&self) -> &'static [PrimitiveTy] {
                match self {
                    $( Self::$key_ident => $types, )+
                }
            }


        }

        impl core::convert::TryFrom<u16> for $enum_name {
            type Error = ();

            fn try_from(value: u16) -> Result<Self, Self::Error> {
                match value {
                    $( $key_tag => Ok($enum_name::$key_ident), )+
                    _ => Err(()),
                }
            }
        }
    }
}

use {PrimitiveCount as Pc, PrimitiveTy as Pt};

/// A set of all known tags and their IFD groups.
#[derive(Copy, Clone, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum KnownTag {
    Ifd0Tag(Ifd0Tag),
    ExifIfdTag(ExifIfdTag),
    GpsIfdTag(GpsIfdTag),
    InteropIfdTag(InteropIfdTag),
}

impl KnownTag {
    /// Returns the number of primitives this tag's value may store.
    ///
    /// ```
    /// use raves_metadata_types::exif::{
    ///     tags::{KnownTag, Ifd0Tag},
    ///     primitives::PrimitiveCount
    /// };
    ///
    /// let image_width: KnownTag = KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth);
    /// assert_eq!(image_width.count(), PrimitiveCount::Known(1));
    /// ```
    pub const fn count(&self) -> PrimitiveCount {
        match self {
            KnownTag::Ifd0Tag(k) => k.count(),
            KnownTag::ExifIfdTag(k) => k.count(),
            KnownTag::GpsIfdTag(k) => k.count(),
            KnownTag::InteropIfdTag(k) => k.count(),
        }
    }

    /// Returns the `IfdGroup` that this enum represents.
    ///
    /// ```
    /// use raves_metadata_types::exif::{tags::{KnownTag, Ifd0Tag}, ifd::IfdGroup};
    ///
    /// let image_width: KnownTag = KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth);
    /// assert_eq!(image_width.ifd_group(), IfdGroup::_0);
    /// ```
    pub const fn ifd_group(self) -> IfdGroup {
        match self {
            KnownTag::Ifd0Tag(_) => Ifd0Tag::ifd_group(),
            KnownTag::ExifIfdTag(_) => ExifIfdTag::ifd_group(),
            KnownTag::GpsIfdTag(_) => GpsIfdTag::ifd_group(),
            KnownTag::InteropIfdTag(_) => InteropIfdTag::ifd_group(),
        }
    }

    /// Returns this tag's tag ID.
    ///
    /// ```
    /// use raves_metadata_types::exif::tags::{KnownTag, Ifd0Tag};
    ///
    /// let image_width: KnownTag = KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth);
    /// assert_eq!(image_width.tag_id(), 256_u16);
    /// ```
    pub const fn tag_id(&self) -> u16 {
        match self {
            KnownTag::Ifd0Tag(k) => *k as u16,
            KnownTag::ExifIfdTag(k) => *k as u16,
            KnownTag::GpsIfdTag(k) => *k as u16,
            KnownTag::InteropIfdTag(k) => *k as u16,
        }
    }

    /// Grabs a tag's name as defined in the standard.
    ///
    /// ```
    /// use raves_metadata_types::exif::tags::{KnownTag, Ifd0Tag};
    ///
    /// let image_width: KnownTag = KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth);
    /// assert_eq!(image_width.tag_name(), "Image width");
    /// ```
    pub const fn tag_name(&self) -> &'static str {
        match self {
            KnownTag::Ifd0Tag(k) => k.tag_name(),
            KnownTag::ExifIfdTag(k) => k.tag_name(),
            KnownTag::GpsIfdTag(k) => k.tag_name(),
            KnownTag::InteropIfdTag(k) => k.tag_name(),
        }
    }

    /// Returns the type(s) this tag's value may have.
    ///
    /// ```
    /// use raves_metadata_types::exif::{
    ///     tags::{KnownTag, Ifd0Tag},
    ///     primitives::PrimitiveTy
    /// };
    ///
    /// let image_width: KnownTag = KnownTag::Ifd0Tag(Ifd0Tag::ImageWidth);
    /// assert_eq!(image_width.types(), &[PrimitiveTy::Short, PrimitiveTy::Long]);
    /// ```
    pub const fn types(&self) -> &'static [PrimitiveTy] {
        match self {
            KnownTag::Ifd0Tag(k) => k.types(),
            KnownTag::ExifIfdTag(k) => k.types(),
            KnownTag::GpsIfdTag(k) => k.types(),
            KnownTag::InteropIfdTag(k) => k.types(),
        }
    }
}

impl TryFrom<(IfdGroup, u16)> for KnownTag {
    type Error = ();

    fn try_from(value: (IfdGroup, u16)) -> Result<Self, Self::Error> {
        let (ifd_group, tag_id): (IfdGroup, u16) = value;

        match ifd_group {
            IfdGroup::_0 => Ifd0Tag::try_from(tag_id).map(KnownTag::Ifd0Tag),
            IfdGroup::Exif => ExifIfdTag::try_from(tag_id).map(KnownTag::ExifIfdTag),
            IfdGroup::Gps => GpsIfdTag::try_from(tag_id).map(KnownTag::GpsIfdTag),
            IfdGroup::Interop => InteropIfdTag::try_from(tag_id).map(KnownTag::InteropIfdTag),
        }
    }
}

/// A list of all the "pointer tags" used to indicate other IFDs.
pub const SUB_IFD_POINTER_TAGS: &[FieldTag] = &[
    FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::ExifIfdPointer)),
    FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::GpsInfoIfdPointer)),
    FieldTag::Known(KnownTag::Ifd0Tag(Ifd0Tag::InteroperabilityIfdPointer)),
];

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
make_key_list_for_group!(enum Ifd0Tag => IfdGroup::_0,
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

    // these are IFD pointers.
    //
    // they're poorly placed in the Exif v3.0 spec, but each of them is a tag
    // under the "0th IFD" (IFD0).
    //
    // `ExifIfdPointer` and `GpsInfoIfdPointer` are actually included in TIFF
    // according to the standard, while `InteroperabilityIfdPointer` is a
    // private extension from Exif.
    //
    // WARNING: if you add any additional pointer tags here, YOU MUST add them
    // to the `SUB_IFD_POINTER_TAGS` const at the top of this file.
    //
    // otherwise, parser logic will be incorrect!
    ExifIfdPointer = 34665 => {
        name: "Exif IFD Pointer",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    GpsInfoIfdPointer = 34853 => {
        name: "GPSInfo IFD Pointer",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    InteroperabilityIfdPointer = 40965 => {
        name: "Interoperability IFD Pointer",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
);

/*
 *
 *
 *
 *
 *
 *
 *
 *  Exif IFD Attribute List
 *
 *
 *
 *
 *
 *
 *
 *
 */
make_key_list_for_group!(enum ExifIfdTag  => IfdGroup::Exif,
    //
    // tags relating to version
    ExifVersion = 36864 => {
           name: "Exif version",
           types: &[Pt::Undefined],
           count: Pc::Known(4),
    },
    FlashpixVersion = 40960 => {
        name: "Supported Flashpix version",
        types: &[Pt::Undefined],
        count: Pc::Known(4),
    },

    //
    // image data characteristics
    ColorSpace = 40961 => {
        name: "Color space information",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    Gamma = 42240 => {
        name: "Gamma",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },

    //
    // image configuration
    ComponentsConfiguration = 37121 => {
        name: "Meaning of each component",
        types: &[Pt::Undefined],
        count: Pc::Known(4),
    },
    CompressedBitsPerPixel = 37122 => {
        name: "Image compression mode",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    PixelXDimension = 40962 => {
        name: "Valid image width",
        types: &[Pt::Short, Pt::Long],
        count: Pc::Known(1),
    },
    PixelYDimension = 40963 => {
        name: "Valid image height",
        types: &[Pt::Short, Pt::Long],
        count: Pc::Known(1),
    },

    //
    // user information
    MakerNote = 37500 => {
        name: "Manufacturer notes",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
    UserComment = 37510 => {
        name: "User comments",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },

    //
    // related file information
    RelatedSoundFile = 40964 => {
        name: "Related audio file",
        types: &[Pt::Ascii],
        count: Pc::Known(13),
    },

    //
    // date and time
    DateTimeOriginal = 36867 => {
        name: "Date and time of original data generation",
        types: &[Pt::Ascii],
        count: Pc::Known(20),
    },
    DateTimeDigitized = 36868 => {
        name: "Date and time of digital data generation",
        types: &[Pt::Ascii],
        count: Pc::Known(20),
    },
    OffsetTime = 36880 => {
        name: "Offset data of DateTime",
        types: &[Pt::Ascii],
        count: Pc::Known(7),
    },
    OffsetTimeOriginal = 36881 => {
        name: "Offset data of DateTimeOriginal",
        types: &[Pt::Ascii],
        count: Pc::Known(7),
    },
    OffsetTimeDigitized = 36882 => {
        name: "Offset data of DateTimeDigitized",
        types: &[Pt::Ascii],
        count: Pc::Known(7),
    },
    SubSecTime = 37520 => {
        name: "DateTime sub-seconds",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },
    SubSecTimeOriginal  = 37521 => {
        name: "DateTimeOriginal sub-seconds",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },
    SubSecTimeDigitized = 37522 => {
        name: "DateTimeDigitized sub-seconds",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },

    //
    // shooting situation (sounds serious man!)
    Temperature = 37888 => {
        name: "Temperature",
        types: &[Pt::SRational],
        count: Pc::Known(1),
    },
    Humidity = 37889 => {
        name: "Humidity",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    Pressure = 37890 => {
        name: "Pressure",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    WaterDepth = 37891 => {
        name: "WaterDepth",
        types: &[Pt::SRational],
        count: Pc::Known(1),
    },
    Acceleration = 37892 => {
        name: "Acceleration",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    CameraElevationAngle = 37893 => {
        name: "Camera elevation angle",
        types: &[Pt::SRational],
        count: Pc::Known(1),
    },

    //
    // other tags
    ImageUniqueID = 42016 => {
        name: "Unique image ID",
        types: &[Pt::Ascii],
        count: Pc::Known(33),
    },
    CameraOwnerName = 42032 => {
        name: "Camera Owner Name",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    BodySerialNumber = 42033 => {
        name: "Body Serial Number",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },
    LensSpecification = 42034 => {
        name: "Lens Specification",
        types: &[Pt::Rational],
        count: Pc::Known(4),
    },
    LensMake = 42035 => {
        name: "Lens Make",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    LensModel = 42036 => {
        name: "Lens Model",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    LensSerialNumber = 42037 => {
        name: "Lens Serial Number",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },
    ImageTitle = 42038 => {
        name: "Tiele name of Image", // FIXME: typo in standard?
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    Photographer = 42039 => {
        name: "Photographer name",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    ImageEditor = 42040 => {
        name: "Person who edited the image",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    CameraFirmware = 42041 => {
        name: "Camera Firmware",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    RAWDevelopingSoftware = 42042 => {
        name: "RAW Developing Software",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    ImageEditingSoftware = 42043 => {
        name: "Image Editing Software",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },
    MetadataEditingSoftware = 42044 => {
        name: "Metadata Editing Software",
        types: &[Pt::Ascii, Pt::Utf8],
        count: Pc::Any,
    },

    //
    // picture-taking conditions
    ExposureTime = 33434 => {
        name: "Exposure time",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    FNumber = 33437 => {
        name: "F number",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    ExposureProgram = 34850 => {
        name: "Exposure program",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    SpectralSensitivity = 34852 => {
        name: "Spectral sensitivity",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },
    PhotographicSensitivity = 34855 => {
        name: "Photographic Sensitivity",
        types: &[Pt::Short],
        count: Pc::Any,
    },
    OECF = 34856 => {
        name: "Optoelectric conversion factor",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
    SensitivityType = 34864 => {
        name: "Sensitivity Type",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    StandardOutputSensitivity = 34865 => {
        name: "Standard Output Sensitivity",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    RecommendedExposureIndex = 34866 => {
        name: "Recommended ExposureIndex",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    ISOSpeed = 34867 => {
        name: "ISO Speed",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    ISOSpeedLatitudeyyy = 34868 => {
        name: "ISO Speed Latitude yyy",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    ISOSpeedLatitudezzz = 34869 => {
        name: "ISO Speed Latitude zzz",
        types: &[Pt::Long],
        count: Pc::Known(1),
    },
    ShutterSpeedValue = 37377 => {
        name: "Shutter speed",
        types: &[Pt::SRational],
        count: Pc::Known(1),
    },
    ApertureValue = 37378 => {
        name: "Aperture",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    BrightnessValue = 37379 => {
        name: "Brightness",
        types: &[Pt::SRational],
        count: Pc::Known(1),
    },
    ExposureBiasValue = 37380 => {
        name: "Exposure bias",
        types: &[Pt::SRational],
        count: Pc::Known(1),
    },
    MaxApertureValue = 37381 => {
        name: "Maximum lens aperture",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    SubjectDistance = 37382 => {
        name: "Subject distance",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    MeteringMode = 37383 => {
        name: "Metering mode",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    LightSource = 37384 => {
        name: "Light source",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    Flash = 37385 => {
        name: "Flash",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    FocalLength = 37386 => {
        name: "Lens focal length",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    SubjectArea = 37396 => {
        name: "Subject area",
        types: &[Pt::Short],
        count: Pc::KnownRange { lower: 2, upper: 4 },
    },
    FlashEnergy = 41483 => {
        name: "Flash energy",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    SpatialFrequencyResponse = 41484 => {
        name: "Spatial frequency response",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
    FocalPlaneXResolution = 41486 => {
        name: "Focal plane X resolution",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    FocalPlaneYResolution = 41487 => {
        name: "Focal plane Y resolution",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    FocalPlaneResolutionUnit = 41488 => {
        name: "Focal plane resolution unit",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    SubjectLocation = 41492 => {
        name: "Subject location",
        types: &[Pt::Short],
        count: Pc::Known(2),
    },
    ExposureIndex = 41493 => {
        name: "Exposure index",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    SensingMethod = 41495 => {
        name: "Sensing method",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    FileSource = 41728 => {
        name: "File source",
        types: &[Pt::Undefined],
        count: Pc::Known(1),
    },
    SceneType = 41729 => {
        name: "Scene type",
        types: &[Pt::Undefined],
        count: Pc::Known(1),
    },
    CFAPattern = 41730 => {
        name: "CFA pattern",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
    CustomRendered = 41985 => {
        name: "Custom image processing",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    ExposureMode = 41986 => {
        name: "Exposure mode",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    WhiteBalance = 41987 => {
        name: "White balance",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    DigitalZoomRatio = 41988 => {
        name: "Digital zoom ratio",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    FocalLengthIn35mmFilm = 41989 => {
        name: "Focal length in 35 mm film",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    SceneCaptureType = 41990 => {
        name: "Scene capture type",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    GainControl = 41991 => {
        name: "Gain control",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    Contrast = 41992 => {
        name: "Contrast",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    Saturation = 41993 => {
        name: "Saturation",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    Sharpness = 41994 => {
        name: "Sharpness",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    DeviceSettingDescription = 41995 => {
        name: "Device settings description",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
    SubjectDistanceRange = 41996 => {
        name: "Subject distance range",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    CompositeImage = 42080 => {
        name: "Composite image",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    SourceImageNumberOfCompositeImage = 42081 => {
        name: "Source image number of composite image",
        types: &[Pt::Short],
        count: Pc::Known(2),
    },
    SourceExposureTimesOfCompositeImage = 42082 => {
        name: "Source exposure times of composite image",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
);

/*
 *
 *
 *
 *
 *
 *
 *
 *  GPS Attribute List
 *
 *
 *
 *
 *
 *
 *
 *
 */
make_key_list_for_group!(enum GpsIfdTag => IfdGroup::Gps,
    GPSVersionID = 0 => {
         name: "GPS tag version",
         types: &[Pt::Byte],
         count: Pc::Known(4),
      },
    GPSLatitudeRef = 1 => {
        name: "North or South Latitude",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSLatitude = 2 => {
        name: "Latitude",
        types: &[Pt::Rational],
        count: Pc::Known(3),
    },
    GPSLongitudeRef = 3 => {
        name: "East or West Longitude",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSLongitude = 4 => {
        name: "Longitude",
        types: &[Pt::Rational],
        count: Pc::Known(3),
    },
    GPSAltitudeRef = 5 => {
        name: "Altitude reference",
        types: &[Pt::Byte],
        count: Pc::Known(1),
    },
    GPSAltitude = 6 => {
        name: "Altitude",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    GPSTimeStamp = 7 => {
        name: "GPS time (atomic clock)",
        types: &[Pt::Rational],
        count: Pc::Known(3),
    },
    GPSSatellites = 8 => {
        name: "GPS satellites used for measurement",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },
    GPSStatus = 9 => {
        name: "GPS receiver status",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSMeasureMode = 10 => {
        name: "GPS measurement mode",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSDOP = 11 => {
        name: "Measurement precision",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    GPSSpeedRef = 12 => {
        name: "Speed unit",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSSpeed = 13 => {
        name: "Speed of GPS receiver",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    GPSTrackRef = 14 => {
        name: "Reference for direction of movement",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSTrack = 15 => {
        name: "Direction of movement",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    GPSImgDirectionRef = 16 => {
        name: "Reference for direction of image",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSImgDirection = 17 => {
        name: "Direction of image",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    GPSMapDatum = 18 => {
        name: "Geodetic survey data used",
        types: &[Pt::Ascii],
        count: Pc::Any,
    },
    GPSDestLatitudeRef = 19 => {
        name: "Reference for latitude of destination",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSDestLatitude = 20 => {
        name: "Latitude of destination",
        types: &[Pt::Rational],
        count: Pc::Known(3),
    },
    GPSDestLongitudeRef = 21 => {
        name: "Reference for longitude of destination",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSDestLongitude = 22 => {
        name: "Longitude of destination",
        types: &[Pt::Rational],
        count: Pc::Known(3),
    },
    GPSDestBearingRef = 23 => {
        name: "Reference for bearing of destination",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSDestBearing = 24 => {
        name: "Bearing of destination",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    GPSDestDistanceRef = 25 => {
        name: "Reference for distance to destination",
        types: &[Pt::Ascii],
        count: Pc::Known(2),
    },
    GPSDestDistance = 26 => {
        name: "Distance to destination",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
    GPSProcessingMethod = 27 => {
        name: "Name of GPS processing method",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
    GPSAreaInformation = 28 => {
        name: "Name of GPS area",
        types: &[Pt::Undefined],
        count: Pc::Any,
    },
    GPSDateStamp = 29 => {
        name: "GPS date",
        types: &[Pt::Ascii],
        count: Pc::Known(11),
    },
    GPSDifferential = 30 => {
        name: "GPS differential correction",
        types: &[Pt::Short],
        count: Pc::Known(1),
    },
    GPSHPositioningError = 31 => {
        name: "Horizontal positioning error",
        types: &[Pt::Rational],
        count: Pc::Known(1),
    },
);

/*
 *
 *
 *
 *
 *
 *
 *
 *  Interoperability IFD Attribute List
 *
 *
 *
 *
 *
 *
 *
 *
 */
make_key_list_for_group!(enum InteropIfdTag => IfdGroup::Interop,
    // well, this is it
    InteroperabilityIndex = 1 => {
       name: "Interoperability Identification",
       types: &[Pt::Ascii],
       count: Pc::Any,
   },
);
