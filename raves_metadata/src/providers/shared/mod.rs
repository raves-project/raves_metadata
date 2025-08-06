//! # Shared
//!
//! Stuff that's shared between multiple container formats.
//!
//! For example, both JPEG XL and MP4 use BMFF, so rewriting that underlying
//! behavior does nothing except create bugs.
//!
//! Instead, it's shared here!

pub mod bmff;
