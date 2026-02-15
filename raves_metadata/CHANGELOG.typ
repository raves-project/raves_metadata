= Changelog (`raves_metadata`)

This file is ordered from newest to oldest.

== v0.1.0

- Remove locking on inner metadata types
  - This change is in preparation for write support; we'll use objects less like a resource and more like a metadata "snapshot".
  - When it comes time to write, we'll compare length and hashes to know if we need to reparse first.
  - Also, making this change allows us to remove the `parking_lot` dependency, which is great!
- Remove lazy metadata parsing
  - Eager parsing seems to be the same amount of "efficient" for most files, as we already gotta parse them anyway!
- Simplify API (one `MetadataProvider`)
  - In other words, there's no longer a `MetadataProviderRaw`!

== v0.0.4

Add support for the GIF file format.

== v0.0.3

Use the newest version of `raves_metadata_types`.

== v0.0.2

Improvements to documentation and downloaded crate size.

- Add documentation to all public items
- Avoid publishing test assets and other unnecessary stuff

== v0.0.1

The crate's initial release on `Crates.io`.

- Support these metadata standards:
  - Exif
  - IPTC (only IPTC4XMP; no IIC just yet lol)
  - XMP
- Read support for the following file formats (providers):
  - AVIF
  - HEIC
  - JPEG
  - QuickTime (`.mov` and friends)
  - MPEG-4 (`.mp4` and others)
  - PNG
  - WebP
