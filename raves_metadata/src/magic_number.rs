use crate::providers::{
    avif::Avif, heic::Heic, jpeg::Jpeg, mov::Mov, mp4::Mp4, png::Png, webp::Webp,
};

pub fn provider(input: &[u8]) -> Option<AnyProvider> {
    todo!()
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
        /// [`MetadataProvider`].
        #[repr(u16)]
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Hash)]
        pub enum MagicNumber {
            $($variant,)+
        }

        /// A wrapper for "any" provider.
        ///
        /// You must check what's inside to use it directly, or you can call
        /// [`MetadataProvider`] methods on this object directly.
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

        // for `MagicNumber`, we can get the magic number from some given
        // input.
        //
        // so, do that.
        impl MagicNumber {
            pub fn new(input: &impl AsRef<[u8]>) -> Option<Self> {
                // make the input into a normal ahh slice
                let input: &[u8] = input.as_ref();

                // try handing it to each provider...
                $(
                    if <$provider_ty as $crate::MetadataProvider>::magic_number(input) {
                        return Some(Self::$variant);
                    }
                )+

                // if none of the types matched, return `None`.
                None
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
    };
}

generate!(
    Avif => { provider_ty: Avif },
    Heif => { provider_ty: Heic },
    Jpeg => { provider_ty: Jpeg },
    Mov => { provider_ty: Mov },
    Mp4 => { provider_ty: Mp4 },
    Png => { provider_ty: Png },
    Webp => { provider_ty: Webp },
);
