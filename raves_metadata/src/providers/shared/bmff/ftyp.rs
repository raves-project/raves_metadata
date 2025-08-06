use winnow::{Parser, binary::be_u32, combinator::repeat, error::EmptyError, token::take};

use crate::providers::shared::bmff::{BoxHeader, BoxType, parse_header};

pub struct FtypBox {
    pub header: BoxHeader,
    pub major_brand: [u8; 4],
    pub _minor_version: u32,
    pub compatible_brands: Vec<[u8; 4]>,
}

impl FtypBox {
    pub fn new(input: &mut &[u8]) -> Option<Self> {
        let header: BoxHeader = parse_header(input).ok()?;

        // ensure header has right box ty
        let BoxType::Id([b'f', b't', b'y', b'p']) = header.box_type else {
            return None;
        };

        let Some(payload_len) = header.payload_len() else {
            log::warn!(
                "Payload length was infinite, but we're parsing the `ftyp` box! \
                That's not expected..."
            );
            return None;
        };

        let major_brand: [u8; 4] = parse_fourcc(input).ok()?;

        let minor_version: u32 = be_u32::<_, EmptyError>.parse_next(input).ok()?;

        let compatible_brands: Vec<[u8; 4]> = repeat(
            0..=((payload_len as usize).saturating_sub(8_usize) / 4_usize),
            parse_fourcc,
        )
        .parse_next(input)
        .ok()?;

        Some(Self {
            header,
            major_brand,
            _minor_version: minor_version,
            compatible_brands,
        })
    }
}

fn parse_fourcc(input: &mut &[u8]) -> Result<[u8; 4], ()> {
    take::<_, _, EmptyError>(4_usize)
        .parse_next(input)
        .map_err(|_| ())?
        .try_into()
        .map_err(|_| ())
}

