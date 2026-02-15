//! # `raves_metadata`
//!
//! A library to parse and handle metadata from a variety of media file formats.
//!
//! ## Progress and Features
//!
//! This library is currently in its early stages. I'll document progress and features when that's necessary.
//!
//! <!--- TODO: see above. -->
//!
//! ## Contributing
//!
//! Contributions are welcome! Please submit PRs or issues at your leisure.
//!
//! ## License
//!
//! This project is dual-licensed under either the Apache License 2.0 or the MIT License at your option.
//!
//! For more information, please see the [`LICENSE-APACHE`](LICENSE-APACHE) and [`LICENSE-MIT`](LICENSE-MIT) files at the root of this repository.
//!
//! ## Why this project?
//!
//! I was making a gallery app for Android [called Raves](https://github.com/raves-project/raves)! However, I was having a lot of trouble finding a suitable library that did metadata parsing and editing.
//!
//! ### Oh, dang! So, why not use Exiv2?
//!
//! Exiv2 is [a great project](https://exiv2.org/) with a wonderful community! However, when trying to use it for my project, I faced some challenges. It is...
//!
//! - released under a copyleft license
//!   - ...resulting in it being less accessible for those using permissive licenses
//!   - and challenging to get working on Android (...as a dylib)
//! - written in C++
//!   - ...meaning it lacks C bindings with a proper API, so it's hard to use in Rust
//! - [not particularly portable](https://github.com/Exiv2/exiv2/issues/3040)
//!   - ...which is probably my fault, but it still scares me
//!
//! For people who don't have specific requirements, Exiv2 is an incredible option. However, it just wouldn't work for me, no matter how hard I tried.

#![forbid(unsafe_code)]

use crate::{
    exif::{Exif, error::ExifFatalError},
    iptc::{Iptc, error::IptcError},
    xmp::{Xmp, error::XmpError},
};

pub mod exif;
pub mod iptc;
pub mod magic_number;
pub mod providers;
pub mod xmp;

/// Attempts to parse the given file for any `MetadataProvider`, such as JPEG
/// or MP4.
///
/// ```
/// use raves_metadata::parse;
/// use raves_metadata::magic_number::AnyProvider;
///
/// // load and parse a file
/// # let file = include_bytes!("../assets/providers/avif/exif_xmp_after_image_blob.avif");
/// // let file = (...);
/// let maybe_provider: Option<AnyProvider> = parse(&file);
///
/// // grab its XMP metadata, for example:
/// if let Some(ref parsed) = maybe_provider {
///     let _xmp = parsed.xmp();
///
///     // use the XMP!
///     // ...
/// }
///
/// // you can also unwrap the inner type, if you want that
/// if let Some(AnyProvider::Avif(ref _avif)) = maybe_provider {
///     // use `Avif` object directly!
///     // ...
/// }
/// ```
#[inline(always)]
pub fn parse(input: &impl AsRef<[u8]>) -> Option<magic_number::AnyProvider> {
    magic_number::parse(input)
}

/// Checks the file type of the given file.
///
/// ```
/// use raves_metadata::get;
/// use raves_metadata::magic_number::MagicNumber;
///
/// // load a file and try to find its "magic number" (file type)
/// # let file = include_bytes!("../assets/providers/avif/exif_xmp_after_image_blob.avif");
/// // let file = (...);
/// let maybe_magic_number: Option<MagicNumber> = get(&file);
///
/// assert_eq!(maybe_magic_number, Some(MagicNumber::Avif));
/// ```
#[inline(always)]
pub fn get(input: &impl AsRef<[u8]>) -> Option<magic_number::MagicNumber> {
    magic_number::get(input)
}

/// A media file with support for various metadata formats.
///
/// Each file format is a "provider" - it'll yield its metadata through parsing.
pub trait MetadataProvider:
    Clone + core::fmt::Debug + Sized + Send + Sync + magic_number::_MagicNumberMarker
{
    /// An error that can occur when calling [`MetadataProvider::new`].
    type ConstructionError: Clone
        + core::fmt::Debug
        + PartialEq
        + PartialOrd
        + core::error::Error
        + Sized
        + Send
        + Sync;

    /// Parses a media file for its metadata.
    fn new(input: &impl AsRef<[u8]>)
    -> Result<Self, <Self as MetadataProvider>::ConstructionError>;

    /// Parses `self` to find any Exif metadata.
    ///
    /// This returns `None` if Exif isn't supported, or if the file has no Exif
    /// metadata.
    ///
    /// The returned `Exif` struct will provide all IFDs in the metadata,
    /// meaning that separation is maintained.
    ///
    /// # Errors
    ///
    /// This will return an error if the file's metadata is malformed or
    /// corrupted.
    fn exif(&self) -> Option<Result<&Exif, &ExifFatalError>>;

    /// Parses `self` to find any IPTC metadata.
    ///
    /// This returns `None` if IPTC isn't supported, or if the file has no IPTC
    /// metadata.
    ///
    /// All IPTC blocks are combined into one list of `(key, value)` pairs.
    ///
    /// # Errors
    ///
    /// This will return an error if the file's metadata is malformed or
    /// corrupted.
    fn iptc(&self) -> Option<Result<&Iptc, &IptcError>> {
        log::error!(
            "Attempted to parse for IPTC, but IPTC IIC isn't \
            implemented in this library yet. \
            Returning None..."
        );
        None
    }

    /// Parses `self` to find any XMP metadata.
    ///
    /// This returns `None` if the XMP isn't supported, or if the file has no
    /// XMP metadata.
    ///
    /// # Errors
    ///
    /// This will return an error if the file's metadata is malformed or
    /// corrupted.
    fn xmp(&self) -> Option<Result<&Xmp, &XmpError>>;

    /// Indicates whether the given input matches the magic number of this
    /// provider.
    ///
    /// `input` only needs to be as long as the magic number -- don't shove 40
    /// GiB memmap'd files in here.
    ///
    /// # Returns
    ///
    /// - `true` if `input` matches the expected magic number (signature).
    /// - Otherwise, `false`.
    ///
    /// Note that this is fallible, as any arbitrary byte slice could have the
    /// expected signature. However, this method will never panic.
    fn magic_number(input: &[u8]) -> bool;
}

/// Internal utility methods.
pub(crate) mod util {
    /// Helper function to initialize the logger for testing.
    #[cfg(test)]
    pub fn logger() {
        _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();
    }
}
