use winnow::{ModalResult, Parser as _, binary::le_u32, error::ContextError, token::take};

#[derive(Debug, PartialEq)]
pub struct RiffChunk {
    pub fourcc: [u8; 4],
    pub len: u32,
}

/// Parses out a chunk of the RIFF data.
///
/// The format in WebP is simple:
///
/// - let fourcc = input.read(4)
/// - let len = input.read_u32()
/// - let data = input.read(chunk_len)
///
/// There may also be a padding byte, but that's handled here as well.
pub fn chunk(input: &mut &[u8]) -> ModalResult<RiffChunk, ContextError> {
    // grab the chunk identifier (fourcc)
    let fourcc: [u8; 4] = {
        let slice: &[u8] = take(4_usize).parse_next(input)?;

        // note: `slice` must be four elements long.
        //
        // `winnow` requires `slice` to be 4 elements long already, so we can do
        // this nonsense pretty easily :)
        let Ok(fourcc) = slice.try_into() else {
            unreachable!(
                "fourcc slice is known to be `n` elements long. please report this - it's a bug!"
            );
        };
        fourcc
    };

    // see how long the chunk is
    let len: u32 = le_u32.parse_next(input)?;

    // // if the chunk len is odd, skip a byte
    // //
    // // TODO: sanity check: standard doesn't say _where_ this padding byte goes (fuuuuck)
    // if (len % 2) == 1 {
    //     log::trace!("Skipping a padding byte in this chunk.");
    //     take(1_usize).parse_next(&mut input)?;
    // }

    // let data: &[u8] = take(len).parse_next(&mut input)?;

    Ok(RiffChunk { fourcc, len })
}
