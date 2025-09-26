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

use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    exif::{Exif, error::ExifFatalError},
    iptc::{Iptc, error::IptcError},
    xmp::{Xmp, error::XmpError},
};

pub mod exif;
pub mod iptc;
pub mod providers;
pub mod xmp;

/// A media file with support for various metadata formats.
///
/// Each file format is a "provider" - it'll yield its metadata through parsing.
pub trait MetadataProvider:
    Clone + core::fmt::Debug + Sized + Send + Sync + MetadataProviderRaw
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
    fn exif(&self) -> Option<Result<Arc<RwLock<Exif>>, ExifFatalError>> {
        // create helper functions.
        //
        // these are necessary since we'd otherwise duplicate the logic
        // below.
        //
        // why? because, to avoid data races, we need to check both times,
        // when we get the lock, the state of the data.
        //
        // (doing so also allows us to only `read` at first, then
        // conditionally `write`... which is nice)
        fn handle_already_parsed(
            p: &Wrapped<Exif>,
        ) -> Option<Result<Arc<RwLock<Exif>>, ExifFatalError>> {
            log::trace!("Cached Exif found! Returning...");
            return Some(Ok(Arc::clone(&p.0))); // cheap clone.
        }
        fn handle_none<A>() -> Option<A> {
            log::trace!("No Exif is present in this struct. Returning early.");
            return None;
        }

        // if we can access the exif... do that.
        match &*self.exif_raw().read() {
            // we'll handle this case in a sec.
            Some(MaybeParsed::Raw(_)) => (),

            // already parsed, so let's return that!
            Some(MaybeParsed::Parsed(p)) => return handle_already_parsed(p),

            // there's no exif! early return.
            None => return handle_none(),
        }

        // otherwise, init the exif and return it.
        //
        // note that this re-uses the code above to avoid writing if
        // possible. (it also prevents "data race" kinda problems)
        let raw = self.exif_raw();
        let locked = &mut *raw.write();
        match locked {
            // we'll handle this case in a sec.
            Some(MaybeParsed::Raw(r)) => {
                match Exif::new(&mut r.as_slice()) {
                    // great, it worked!
                    //
                    // return the resulting exif
                    Ok(p) => {
                        let wrapped: Wrapped<Exif> = Wrapped(Arc::new(RwLock::new(p)));
                        log::trace!("Completed Exif parsing! Cached internally.");
                        locked
                            .as_mut()
                            .map(|a| *a = MaybeParsed::Parsed(wrapped.clone()));
                        return Some(Ok(wrapped.0));
                    }

                    // otherwise, it's an error.
                    //
                    // report it and return an Err!
                    Err(e) => {
                        log::error!("Failed to parse Exif! err: {e}");
                        *locked = None;
                        return Some(Err(e));
                    }
                }
            }

            Some(MaybeParsed::Parsed(p)) => return handle_already_parsed(p),
            None => return handle_none(),
        }
    }

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
    fn iptc(&self) -> Option<Result<Arc<RwLock<Iptc>>, IptcError>> {
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
    fn xmp(&self) -> Option<Result<Arc<RwLock<Xmp>>, XmpError>> {
        // create helper functions.
        //
        // these are necessary since we'd otherwise duplicate the logic
        // below.
        //
        // why? because, to avoid data races, we need to check both times,
        // when we get the lock, the state of the data.
        //
        // (doing so also allows us to only `read` at first, then
        // conditionally `write`... which is nice)
        fn handle_already_parsed(p: &Wrapped<Xmp>) -> Option<Result<Arc<RwLock<Xmp>>, XmpError>> {
            log::trace!("Cached XMP found! Returning...");
            return Some(Ok(Arc::clone(&p.0))); // cheap clone.
        }
        fn handle_none<A>() -> Option<A> {
            log::trace!("No XMP is present in this struct. Returning early.");
            return None;
        }

        // if we can access the xmp... do that.
        match &*self.xmp_raw().read() {
            // we'll handle this case in a sec.
            Some(MaybeParsed::Raw(_)) => (),

            // already parsed, so let's return that!
            Some(MaybeParsed::Parsed(p)) => return handle_already_parsed(p),

            // there's no xmp! early return.
            None => return handle_none(),
        }

        // otherwise, init the xmp and return it.
        //
        // note that this re-uses the code above to avoid writing if
        // possible. (it also prevents "data race" kinda problems)
        let raw = self.xmp_raw();
        let locked = &mut *raw.write();
        match locked {
            // we'll handle this case in a sec.
            Some(MaybeParsed::Raw(r)) => {
                // try parsing as str, then map into xmp
                let creation_result: Result<Xmp, XmpError> = core::str::from_utf8(r)
                    .map_err(|e| {
                        log::error!("XMP was not in UTF-8 format! err: {e}");
                        XmpError::NotUtf8
                    })
                    .and_then(|s| Xmp::new(s));

                match creation_result {
                    // great, it worked!
                    //
                    // return the resulting xmp
                    Ok(p) => {
                        let wrapped: Wrapped<Xmp> = Wrapped(Arc::new(RwLock::new(p)));
                        log::trace!("Completed XMP parsing! Cached internally.");
                        locked
                            .as_mut()
                            .map(|a| *a = MaybeParsed::Parsed(wrapped.clone()));
                        return Some(Ok(wrapped.0));
                    }

                    // otherwise, it's an error.
                    //
                    // report it and return an Err!
                    Err(e) => {
                        log::error!("Failed to parse XMP! err: {e}");
                        *locked = None;
                        return Some(Err(e));
                    }
                }
            }

            Some(MaybeParsed::Parsed(p)) => return handle_already_parsed(p),
            None => return handle_none(),
        }
    }
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
        Arc::new(const { RwLock::new(None) })
    }

    /// Returns the raw `Option<MaybeParsedXmp>` stored inside the provider.
    ///
    /// Used primarily to implement the [`MetadataProvider::xmp`] method
    /// easily.
    ///
    /// However, users may also prefer it if they'd like to use the raw data
    /// exactly as-is.
    fn xmp_raw(&self) -> Arc<RwLock<Option<MaybeParsedXmp>>> {
        Arc::new(const { RwLock::new(None) })
    }
}

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
    Parsed(Wrapped<P>),
}

pub type MaybeParsedExif = MaybeParsed<Vec<u8>, Exif>;
pub type MaybeParsedIptc = MaybeParsed<Vec<u8>, Iptc>;
pub type MaybeParsedXmp = MaybeParsed<Vec<u8>, Xmp>;

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
