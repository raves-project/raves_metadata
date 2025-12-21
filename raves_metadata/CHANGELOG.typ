= Changelog (`raves_metadata`)

This file is ordered from newest to oldest.

== v0.0.3

Use the newest version of `raves_metdata_types`.

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
