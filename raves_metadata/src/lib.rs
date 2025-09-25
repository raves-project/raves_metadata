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

use core::fmt::Debug;
use std::error::Error;

use iptc::{Iptc, error::IptcError};

use crate::{
    exif::{Exif, error::ExifFatalError},
    xmp::{Xmp, error::XmpError},
};

pub mod exif;
pub mod iptc;
pub mod providers;
pub mod xmp;

/// A media file with support for various metadata formats.
///
/// Each file format is a "provider" - it'll yield its metdata through parsing.
pub trait MetadataProvider<'input>: Clone + Debug + Sized + Send + Sync {
    /// An error that can occur when calling [`MetadataProvider::new`].
    type ConstructionError: Clone + Debug + PartialEq + PartialOrd + Error + Sized + Send + Sync;

    /// Parses a media file for its metadata.
    fn new(
        input: &'input impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider<'input>>::ConstructionError>;

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
    fn exif(&self) -> Option<Result<Exif, ExifFatalError>>;

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
    fn iptc(&self) -> Option<Result<Iptc, IptcError>>;

    /// Parses `self` to find any XMP metadata.
    ///
    /// This returns `None` if the XMP isn't supported, or if the file has no
    /// XMP metadata.
    ///
    /// # Errors
    ///
    /// This will return an error if the file's metadata is malformed or
    /// corrupted.
    fn xmp(&self) -> Option<Result<Xmp, XmpError>>;
}

/// Raw helpers for [`MetadataProvider`] implementors.
///
/// You may or may not find these methods useful, as they tend to deal
/// primarily with field access of internal metadata standards' buffers.
///
/// However, if you wish to modify these directly, or just immediately take
/// the metadata as their raw types, you can use these methods instead!
pub trait MetadataProviderRaw {
    /// Returns the raw `Option<MaybeParsedExif>` stored inside the provider.
    ///
    /// Used primarily to implement the [`MetadataProvider::exif`] method
    /// easily.
    ///
    /// However, users may also prefer it if they'd like to use the raw data
    /// exactly as-is.
    fn exif_raw(&self) -> Arc<RwLock<Option<MaybeParsedExif>>> {
        Arc::new(RwLock::new(None))
    }

    /// Returns the raw `Option<MaybeParsedXmp>` stored inside the provider.
    ///
    /// Used primarily to implement the [`MetadataProvider::xmp`] method
    /// easily.
    ///
    /// However, users may also prefer it if they'd like to use the raw data
    /// exactly as-is.
    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::new(RwLock::new(None))
    }
}

/// Internal utility methods.
pub(crate) mod util {
    use std::sync::{Arc, RwLock};

    use crate::exif::Exif;
    use crate::iptc::Iptc;
    use crate::xmp::Xmp;

    /// Metadata that might have been parsed already.
    ///
    /// This type allows for caching metadata such that media files are not
    /// reprocessed each additional time their parse methods are called.
    ///
    /// ## Generics
    ///
    /// - `R`: Raw
    /// - `P`: Parsed
    ///
    /// ## Why?
    ///
    /// `MaybeParsed::Parsed` metadata can be edited! >:)
    #[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
    pub enum MaybeParsed<R, P>
    where
        R: Clone + core::fmt::Debug + PartialEq + PartialOrd + core::hash::Hash,
        P: Clone + core::fmt::Debug + PartialEq + PartialOrd + core::hash::Hash,
    {
        /// Raw metadata that hasn't been processed.
        Raw(R),

        /// Metadata that's been parsed into its contents.
        Parsed(P),
    }

    pub type MaybeParsedExif = MaybeParsed<Vec<u8>, Wrapped<Exif>>;
    pub type MaybeParsedIptc = MaybeParsed<Vec<u8>, Wrapped<Iptc>>;
    pub type MaybeParsedXmp = MaybeParsed<String, Wrapped<Xmp>>;

    /// A wrapper struct around metadata standard types.
    ///
    /// These provide an easy derive for the [`MaybeParsed`] type above. It
    /// should never be returned in non-raw interfaces.
    #[derive(Clone, Debug)]
    pub struct Wrapped<P: PartialEq + PartialOrd + core::hash::Hash>(
        /// The wrapped value.
        ///
        /// This should be a standard, like [`crate::xmp::Xmp`].
        pub Arc<RwLock<P>>,
    );

    // implement those traits below for ez derives on providers
    impl<P: PartialEq + PartialOrd + core::hash::Hash> PartialEq for Wrapped<P> {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.0, &other.0)
        }
    }
    impl<P: PartialEq + PartialOrd + core::hash::Hash> PartialOrd for Wrapped<P> {
        fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
            (Arc::as_ptr(&self.0)).partial_cmp(&(Arc::as_ptr(&other.0)))
        }
    }
    impl<P: PartialEq + PartialOrd + core::hash::Hash> core::hash::Hash for Wrapped<P> {
        fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
            (Arc::as_ptr(&self.0) as usize).hash(state);
        }
    }

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
