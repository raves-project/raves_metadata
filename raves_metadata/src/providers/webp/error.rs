#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum WebpConstructionError {
    /// Failed to find the required WebP header.
    NoHeader,

    /// No chunks were found in the file.
    NoChunks,

    /// The extended header was malformed.
    ///
    /// It's required to find metadata, so lacking it means we're unable to
    /// continue parsing the file.
    MalformedExtendedHeader,
}

impl core::fmt::Display for WebpConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebpConstructionError::NoHeader => {
                f.write_str("The required WebP header was not found.")
            }

            WebpConstructionError::NoChunks => f.write_str(
                "The WebP file didn't contain any chunks. \
                Can't continue parsing.",
            ),

            WebpConstructionError::MalformedExtendedHeader => f.write_str(
                "The WebP file didn't contain a usable extended header. \
                Can't continue parsing.",
            ),
        }
    }
}

impl core::error::Error for WebpConstructionError {}
