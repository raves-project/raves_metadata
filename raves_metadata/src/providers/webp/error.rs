#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum WebpCreationError {
    NoHeader,
    NoChunks,

    MalformedExtendedHeader,
}
