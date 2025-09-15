//! # Shared
//!
//! Stuff that's shared between multiple container formats.
//!
//! For example, both JPEG XL and MP4 use BMFF, so rewriting that underlying
//! behavior does nothing except create bugs.
//!
//! Instead, it's shared here!

use winnow::{
    error::{StrContext, StrContextValue},
};

pub mod bmff;

/// Creates a string description for `winnow` context.
pub const fn desc(s: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::Description(s))
}
