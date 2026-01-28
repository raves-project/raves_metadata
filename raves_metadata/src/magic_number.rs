//! Helps handle files without knowing their file type beforehand.
//!
//! This module is helpful for those who need general file parsing, such as on
//! a web server.
//!
//! It allows you to avoid specifying a provider as which to parse, instead
//! relying on magic numbers (easy identifiers for a file format) to figure it
//! out at runtime.
//!
//! # Usage
//!
//! You can use this module like so:
//!
//! ```
//! # let some_media_you_loaded: &[u8] = include_bytes!("../assets/providers/avif/exif_xmp_after_image_blob.avif");
//! #
//! use raves_metadata::{get, parse};
//! use raves_metadata::magic_number::{AnyProvider, MagicNumber};
//!
//! // pretend you load your media somehow
//! // ...
//!
//! // the `get` function finds the `MagicNumber`. it does NOT parse the whole
//! // file!
//! let magic_num: Option<MagicNumber> = raves_metadata::get(&some_media_you_loaded);
//! assert_eq!(magic_num.unwrap(), MagicNumber::Avif);
//!
//! // on the other hand, `parse` finds the file type, then parses it for you
//! // to use:
//! let maybe_parsed: Option<AnyProvider> = raves_metadata::parse(&some_media_you_loaded);
//! let parsed: AnyProvider = maybe_parsed.unwrap();
//! assert!(
//!     matches!(
//!         parsed,
//!         AnyProvider::Avif(..),
//!     ),
//! );
//!
//! // by the way, `MagicNumber::new` and `AnyProvider::new` are other ways to
//! // get those same results:
//! assert_eq!(magic_num, MagicNumber::new(&some_media_you_loaded));
//! assert_eq!(
//!     parsed.magic_number(),
//!     AnyProvider::new(&some_media_you_loaded).unwrap().magic_number(),
//! );
//! ```

use crate::providers::{
    avif::Avif, gif::Gif, heic::Heic, jpeg::Jpeg, mov::Mov, mp4::Mp4, png::Png, webp::Webp,
};

/// Reminds contributors to add each provider to the `generate!()` call!
///
/// Do not implement this trait manually -- the `generate` macro will do it for
/// you! :D
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "Please add this type to the `generate!()` macro in the `raves_metadata/src/magic_number.rs` file."
)]
pub trait _MagicNumberMarker {
    #[doc(hidden)]
    fn _do_not_implement_this_manually(self);
}

/// This macro generates two enums and some implementations.
///
/// Both enums are used to handle files in a generic way.
///
/// # Enums + Implementations
///
/// - `MagicNumber`: a representation of which file is present.
macro_rules! generate {
    ( $(
        // name of the provider (must be the same as the provider_ty)
        $variant:ident => {
            // the actual type we map to
            provider_ty: $provider_ty:ty
        },
    )+) => {
        /// A magic number.
        ///
        /// Each one represents one of many supported kinds of
        /// [`MetadataProvider`][`crate::MetadataProvider`].
        #[repr(u16)]
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Hash)]
        pub enum MagicNumber {
            $($variant,)+
        }

        /// A wrapper for "any" provider.
        ///
        /// You must check what's inside to use it directly.
        #[repr(u16)]
        #[derive(Clone, Debug)]
        pub enum AnyProvider {
            // each variant has an attached ty.
            //
            // we use its `ConstructionError` in `Result` directly.
            $($variant(
                Result<
                    $provider_ty,
                    <$provider_ty as $crate::MetadataProvider>::ConstructionError
                >),
            )+
        }

        // we can get the magic number from some given input.
        //
        // so, do that.
        impl MagicNumber {
            /// Attempts to find the magic number of a given file.
            #[inline(always)]
            pub fn new(input: &impl AsRef<[u8]>) -> Option<Self> {
                get(input)
            }
        }

        impl AnyProvider {
            /// Uses the magic number of a file to parse it.
            ///
            /// If the file isn't supported, or is malformed, this may return
            /// `None`.
            #[inline(always)]
            pub fn new(input: &impl AsRef<[u8]>) -> Option<Self> {
                parse(input)
            }

            /// Returns the [`MagicNumber`] of this provider.
            ///
            /// ```
            /// use raves_metadata::magic_number::{AnyProvider, MagicNumber};
            ///
            /// # let file = include_bytes!("../assets/providers/avif/exif_xmp_after_image_blob.avif");
            /// let parsed: AnyProvider = raves_metadata::parse(&file).unwrap();
            ///
            /// //
            /// // pretend like it's been awhile, but you've still got your trusty
            /// // `AnyProvider` in scope ;D
            /// //
            /// // ...
            ///
            /// let magic_number: MagicNumber = parsed.magic_number();
            /// ```
            pub fn magic_number(&self) -> MagicNumber {
                match self {
                    $(Self::$variant(..) => MagicNumber::$variant,)+
                }
            }

            /// Gets Exif metadata from inner
            /// [`MetadataProvider`][`crate::MetadataProvider`].
            ///
            /// For more information, see:
            ///
            /// [`MetadataProvider::exif`][`crate::MetadataProvider::exif`]
            pub fn exif(&self) -> Option<Result<std::sync::Arc<parking_lot::RwLock<crate::Exif>>, crate::ExifFatalError>> {
                match self {
                    $(
                        Self::$variant(maybe_inner) => {
                            let Ok(inner) = maybe_inner else {
                                ::log::error!("The inner provider is an error, not `Ok`. Cannot get metadata.");
                                return None;
                            };
                            <$provider_ty as $crate::MetadataProvider>::exif(inner)
                        },
                    )+
                }
            }

            /// Gets IPTC metadata from inner
            /// [`MetadataProvider`][`crate::MetadataProvider`].
            ///
            /// For more information, see:
            ///
            /// [`MetadataProvider::iptc`][`crate::MetadataProvider::iptc`]
            pub fn iptc(&self) -> Option<Result<std::sync::Arc<parking_lot::RwLock<crate::Iptc>>, crate::IptcError>> {
                match self {
                    $(
                        Self::$variant(maybe_inner) => {
                            let Ok(inner) = maybe_inner else {
                                ::log::error!("The inner provider is an error, not `Ok`. Cannot get metadata.");
                                return None;
                            };
                            <$provider_ty as $crate::MetadataProvider>::iptc(inner)
                        },
                    )+
                }
            }

            /// Gets XMP metadata from inner
            /// [`MetadataProvider`][`crate::MetadataProvider`].
            ///
            /// For more information, see:
            ///
            /// [`MetadataProvider::xmp`][`crate::MetadataProvider::xmp`]
            pub fn xmp(&self) -> Option<Result<std::sync::Arc<parking_lot::RwLock<crate::Xmp>>, crate::XmpError>> {
                match self {
                    $(
                        Self::$variant(maybe_inner) => {
                            let Ok(inner) = maybe_inner else {
                                ::log::error!("The inner provider is an error, not `Ok`. Cannot get metadata.");
                                return None;
                            };
                            <$provider_ty as $crate::MetadataProvider>::xmp(inner)
                        },
                    )+
                }
            }
        }

        // implement `From<SomeProvider>` for both
        $(
            impl From<$provider_ty> for MagicNumber {
                fn from(_item: $provider_ty) -> Self {
                    Self::$variant
                }
            }

            impl From<$provider_ty> for AnyProvider {
                fn from(item: $provider_ty) -> Self {
                    Self::$variant(Ok(item))
                }
            }

            impl _MagicNumberMarker for $provider_ty {
                fn _do_not_implement_this_manually(self) {}
            }
        )+

        // implement `From<AnyProvider>` for `MagicNumber`
        impl From<AnyProvider> for MagicNumber {
            fn from(item: AnyProvider) -> MagicNumber {
                match &item {
                    $(
                        AnyProvider::$variant(_) => MagicNumber::$variant,
                    )+
                }
            }
        }

        // create the `parse` function (for `raves_metadata::parse`)
        pub(super) fn parse(input: &impl AsRef<[u8]>) -> Option<AnyProvider> {
            let slice_input: &[u8] = input.as_ref();

            // check each provider to see if it matches
            $(
                ::log::trace!("Attempting to parse blob as `{}`...", core::any::type_name::<$provider_ty>());
                if <$provider_ty as $crate::MetadataProvider>::magic_number(slice_input) {
                    let p = <$provider_ty as $crate::MetadataProvider>::new(input)
                        .ok()
                        .map(AnyProvider::from);
                    if p.is_some() {
                        return p;
                    }
                }
                ::log::trace!("Not `{}`!", core::any::type_name::<$provider_ty>());
            )+

            // if none of the providers match, return `None`
            ::log::trace!("No providers matched the blob.");
            return None;
        }

        // now, create `get`
        pub(super) fn get(input: &impl AsRef<[u8]>) -> Option<MagicNumber> {
            let slice_input: &[u8] = input.as_ref();

            // check each provider to see if it matches
            $(
                ::log::trace!("Attempting to parse blob as `{}`...", core::any::type_name::<$provider_ty>());
                if <$provider_ty as $crate::MetadataProvider>::magic_number(slice_input) {
                    return Some(MagicNumber::$variant);
                }
                ::log::trace!("Not `{}`!", core::any::type_name::<$provider_ty>());
            )+

            // if none of the providers match, return `None`
            ::log::trace!("No providers matched the blob.");
            return None;
        }
    };
}

generate!(
    Avif => { provider_ty: Avif },
    Heic => { provider_ty: Heic },
    Jpeg => { provider_ty: Jpeg },
    Mov => { provider_ty: Mov },
    Mp4 => { provider_ty: Mp4 },
    Png => { provider_ty: Png },
    Webp => { provider_ty: Webp },
    Gif => { provider_ty: Gif },
);
