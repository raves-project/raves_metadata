//! # Providers
//!
//! This module re-exports the many different "providers" used to obtain a
//! media file's metadata.
//!
//! Providers represent a source of metadata, such as a media file format,
//! including formats like JPEG, or a container format like ISO-BMFF.
//!
//! Each provider has a struct with a `MetadataProvider` implementation. This
//! allows a consistent interface for obtaining metadata from different media
//! sources.

pub mod bmff;
