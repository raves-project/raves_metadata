use winnow::{
    ModalResult, Parser as _,
    binary::le_u32,
    error::{ContextError, ErrMode, StrContext},
    token::literal,
};

#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct WebpFileHeader {
    /// Represents how large the file is.
    ///
    /// This value will have a maximum of `(u32::MAX - 10)`.
    _file_size: u32,
}

/// Parses out the WebP file header block.
///
/// This should be the first thing in the file.
pub fn webp_file_header(input: &mut &[u8]) -> ModalResult<WebpFileHeader, ContextError> {
    // first, we should ensure there's `RIFF` (in ASCII) at the beginning
    const RIFF: &[u8] = b"RIFF";
    literal(RIFF).void().parse_next(input)?;

    // then, we should have a u32
    const FILE_SIZE_MAX: u32 = u32::MAX - 10_u32;
    let file_size: u32 = le_u32.parse_next(input)?;

    // ensure it's the right size...
    if file_size > FILE_SIZE_MAX {
        log::error!(
            "File size was reported as `{file_size}` bytes, but maximum is `{FILE_SIZE_MAX}`! \
            Cannot continue parsing..."
        );
        return Err({
            let mut ce = ContextError::new();
            ce.push(StrContext::Expected(
                winnow::error::StrContextValue::StringLiteral(
                    "File size was too large to be WebP.",
                ),
            ));
            ErrMode::Cut(ce)
        });
    }

    // now, check for the `WEBP` ASCII at the end
    const WEBP: &[u8] = b"WEBP";
    literal(WEBP).void().parse_next(input)?;

    // return the file size in the repr struct
    Ok(WebpFileHeader {
        _file_size: file_size,
    })
}
