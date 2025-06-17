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

use error::IptcError;
use raves_metadata_types::iptc::IptcKeyValue;

pub mod error;
pub mod providers;
pub mod util;

mod xmp;

/// Parsed IPTC.
pub struct Iptc {
    pub pairs: Vec<IptcKeyValue>,
}

/// A media file with support for various metadata formats.
///
/// Each file format is a "provider" - it'll yield its metdata through parsing.
pub trait MetadataProvider {
    /// Parses `self`, a media source, for its IPTC block(s) and returns them
    /// combined into one list of (key, value) pairs.
    fn iptc(&self) -> Result<Iptc, IptcError>;

    // fn exif(&self) -> Result<Exif, ExifError>;

    // fn xmp(&self) -> Result<Xmp, XmpError>;
}
