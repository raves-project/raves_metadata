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

#[cfg(test)]
mod tests {
    #[test]
    fn nul_terminated_str_parses_normally() {
        let mut bytes: &[u8] = "🔥😅 hello world!!! 🦾\0".as_bytes();
        let result: &str = super::parse_nul_terminated_str(&mut bytes).unwrap();
        assert_eq!(result, "🔥😅 hello world!!! 🦾");
    }

    #[test]
    fn nul_terminated_str_without_nul_should_fail() {
        let mut bytes: &[u8] = "🔥😅 hello world!!! 🦾".as_bytes();
        let result = super::parse_nul_terminated_str(&mut bytes);
        assert!(result.is_err());
    }

    #[test]
    fn empty_nul_terminated_str_should_parse() {
        let mut bytes: &[u8] = "\0".as_bytes();
        let result: &str = super::parse_nul_terminated_str(&mut bytes).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn nul_terminated_str_multiple_nuls_should_parse() {
        let mut bytes: &[u8] = "hello world!\0\0".as_bytes();
        let result: &str = super::parse_nul_terminated_str(&mut bytes).unwrap();
        assert_eq!(result, "hello world!");
    }
}
