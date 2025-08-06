use winnow::{Parser as _, binary::u8, error::EmptyError, token::take};

use crate::{
    MetadataProvider,
    iptc::{Iptc, error::IptcError},
    xmp::{Xmp, error::XmpError},
};

use self::{chunk::RiffChunk, error::WebpCreationError, header::WebpFileHeader};

mod chunk;
mod error;
mod extended;
mod header;

pub struct Webp<'file> {
    _header: WebpFileHeader,
    relevant_chunks: Vec<(RiffChunk, &'file [u8])>,
}

impl<'file> Webp<'file> {
    pub fn new(mut file: &'file [u8]) -> Result<Self, WebpCreationError> {
        // this does a little parsing, then disposes of the file...

        // first, look for the header.
        let header =
            header::webp_file_header(&mut file).map_err(|_| WebpCreationError::NoHeader)?;

        // all WebPs should have at least one chunk
        let first_chunk = chunk::chunk(&mut file).map_err(|_| WebpCreationError::NoChunks)?;

        // create an empty type for the file based on those two
        let mut s = Self {
            _header: header,
            relevant_chunks: const { Vec::new() },
        };

        // if it's an "extended" WebP, then it'll use the extended file format.
        //
        // that means it has a file feature info chunk, `VP8X`! if the file
        // doesn't have that chunk, then it has no metadata, and isn't useful
        // to us whatsoever.
        if &first_chunk.fourcc != b"VP8X" {
            log::debug!(
                "Not in 'extended' format: no metadata is present. No relevant chunks exist."
            );
            return Ok(s);
        }

        // then, get info about the file.
        //
        // this is arranged in a manner explained in the WebP docs. see:
        // https://developers.google.com/speed/webp/docs/riff_container
        let file_info_flags: u8 = u8
            .parse_next(&mut file)
            .map_err(|_: EmptyError| WebpCreationError::MalformedExtendedHeader)?;

        // check the `E` (Exif) and `X` (XMP) presence bits
        let (has_exif, has_xmp) = (
            file_info_flags & 0b_0000_1000 != 0,
            file_info_flags & 0b_0001_0000 != 0,
        );

        // map the bools into a list of chunks we care about
        let required_chunks: &[[u8; 4]] = match (has_exif, has_xmp) {
            (false, false) => {
                log::debug!("The provided WebP file has no metadata.");
                return Ok(s);
            }
            (false, true) => &[*b"XMP "],
            (true, false) => &[*b"EXIF"],
            (true, true) => &[*b"EXIF", *b"XMP "],
        };

        // consume the remaining 3 bytes of header + 6 bytes of img size
        take(9_usize)
            .parse_next(&mut file)
            .map_err(|_: EmptyError| {
                log::error!(
                    "Couldn't consume remaining 'extended' bytes! \
                    This is a bug! Please report it."
                );
                WebpCreationError::MalformedExtendedHeader
            })?;

        // account for any padding in the first chunk
        if first_chunk.len & 1 != 0 {
            _ = take::<_, _, EmptyError>(1_usize)
                .void()
                .parse_next(&mut file);
        }

        // loop the rest of the file, collecting only chunks we care about.
        while !file.is_empty() {
            log::info!("loopin");

            // grab the chunk header
            let chunk: RiffChunk = match chunk::chunk(&mut file) {
                Ok(c) => c,
                Err(e) => {
                    log::error!(
                        "Invalid RIFF chunk in WebP file! Returning \
                        results before erroneous chunk. err: {e}"
                    );
                    break;
                }
            };

            // something something borrow checker something
            let chunk_len: u32 = chunk.len;

            // if it's something we care about, add it and its data to the
            // relevent chunks list.
            //
            // otherwise, take its data and move on!
            if required_chunks.contains(&chunk.fourcc) {
                // grab the chunk data
                let Ok::<_, EmptyError>(chunk_data) = take(chunk.len).parse_next(&mut file) else {
                    log::warn!(
                        "Failed to take chunk's length of data. expected len of `{}`, but was only `{}`.",
                        chunk.len,
                        file.len()
                    );
                    continue;
                };

                // add it to the vec
                s.relevant_chunks.push((chunk, chunk_data));
            } else {
                _ = take::<_, _, EmptyError>(chunk.len)
                    .void()
                    .parse_next(&mut file);
            }

            // if the chunk has an odd length, we'll use its padding byte
            if chunk_len % 2 != 0 {
                _ = take::<_, _, EmptyError>(1_usize)
                    .void()
                    .parse_next(&mut file);
            }
        }

        Ok(s)
    }
}

impl<'file> MetadataProvider for Webp<'file> {
    fn iptc(&self) -> Option<Result<Iptc, IptcError>> {
        // WebP doesn't support IPTC :p
        //
        // its keys may still be visible under XMP, though!
        None
    }

    fn xmp(&self) -> Option<Result<Xmp, XmpError>> {
        // note: it's technically allowed for WebP files to have more than one
        // embedded XMP file. for now, we only return the first one we found.
        //
        // TODO: do files often embed more than one XMP document? if so,
        // make this combine them! (or return a vec)
        for chunk in self.relevant_chunks.iter() {
            // ignore anything that isn't an XMP chunk
            if chunk.0.fourcc != *b"XMP " {
                continue;
            }

            // otherwise, we got an XMP chunk!
            //
            // parse its data as UTF-8
            let doc = match core::str::from_utf8(chunk.1) {
                Ok(utf8_str) => utf8_str,
                Err(e) => {
                    log::error!("Contained XMP was not UTF-8! err: {e}");
                    return Some(Err(XmpError::NotUtf8));
                }
            };

            // parse + return it
            return Some(crate::xmp::Xmp::new(doc));
        }

        None
    }

    fn exif(&self) -> Result<crate::exif::Exif, crate::exif::error::ExifFatalError> {
        let todo_impl_exif_for_webp = todo!();
    }
}
