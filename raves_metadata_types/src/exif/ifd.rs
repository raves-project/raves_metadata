//! Contains stuff related to IFDs.
//!
//! For more info, see the [`IfdGroup`] enumeration.

/// An IFD is a set of keys found within a media file's Exif metadata.
///
/// An IFD group is NOT an abstraction - they are _literally_ present in the
/// file. JPEG, for example, can embed a number of IFDs.
///
/// IFDs aren't self-describing in their type, nor do format standards provide
/// any information about their order.
///
/// Instead, IFD 0 will (optionally) contain keys indicating the locations of
/// sub-IFDs within the metadata slice. These are like pointers, not direct
/// embeds.
///
/// That means that IFD 0 is always required if Exif metadata is present, as
/// other groups have no way to self-describe. They're reliant on IFD 0 for
/// its pointer keys.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum IfdGroup {
    /// Refers to "IFD 0".
    ///
    /// This one stems from TIFF, as its the only supported group there.
    /// Outside of TIFF, standards refer to this "first IFD" as IFD 0. Exif
    /// matches the TIFF v6.0 (1992) specification with the provided keys in
    /// this group.
    ///
    /// In TIFF, there can be multiple IFDs, but all will refer to _this group_
    /// and no others, with any additional IFDs being used for "subfiles",
    /// which usually means an IFD for the image's embedded thumbnail.
    #[doc(alias = "IFD0")]
    #[doc(alias = "TIFF")]
    _0,

    /// The "Exif" IFD provides camera-based metadata.
    ///
    /// For example, it's home to important values like exposure information.
    #[doc(alias = "ExifIFD")]
    Exif,

    /// The GPS IFD contains location metadata.
    #[doc(alias = "GPSIFD")]
    Gps,

    /// The interoperability IFD specifies info about what software was used to
    /// write the Exif metadata.
    #[doc(alias = "Interoperability")]
    #[doc(alias = "InteroperabilityIFD")]
    #[doc(alias = "InteropIFD")]
    Interop,
}

impl IfdGroup {
    /// Checks whether this IFD group is optional.
    ///
    /// An IFD group might be "optional", meaning that it doesn't need to be
    /// present in the file.
    ///
    /// As of writing, IFD 0 is the only required group.
    pub fn optional(&self) -> bool {
        match self {
            Self::_0 => false,
            Self::Exif => true,
            Self::Gps => true,
            Self::Interop => true,
        }
    }
}
