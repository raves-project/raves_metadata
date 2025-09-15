//! # Shared
//!
//! Stuff that's shared between multiple container formats.
//!
//! For example, both JPEG XL and MP4 use BMFF, so rewriting that underlying
//! behavior does nothing except create bugs.
//!
//! Instead, it's shared here!

use winnow::error::ContextError;
use winnow::token::literal;
use winnow::{ModalResult, Parser as _};
use winnow::{
    combinator::terminated,
    error::{StrContext, StrContextValue},
    token::take_until,
};

pub mod bmff;

/// Creates a string description for `winnow` context.
pub const fn desc(s: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::Description(s))
}

/// Parses out a NUL-terminated string.
///
/// Consumes the NUL.
pub fn parse_nul_terminated_str<'input>(
    input: &mut &'input [u8],
) -> ModalResult<&'input str, ContextError> {
    terminated(
        take_until(0.., "\0").context(desc("NUL-terminated string")),
        literal("\0").context(desc("NUL literal")),
    )
    .try_map(std::str::from_utf8)
    .parse_next(input)
}
