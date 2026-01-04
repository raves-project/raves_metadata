use winnow::{Parser, binary::be_u32, combinator::repeat, error::EmptyError, token::take};

use crate::providers::shared::bmff::{BoxHeader, BoxType};

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct FtypBox {
    pub header: BoxHeader,
    pub major_brand: [u8; 4],
    pub _minor_version: u32,
    pub compatible_brands: Vec<[u8; 4]>,
}

impl FtypBox {
    pub fn new(input: &mut &[u8]) -> Option<Self> {
        let header: BoxHeader = BoxHeader::new(input).ok()?;
        log::trace!("Box header found.");

        // ensure header has right box ty
        let BoxType::Id([b'f', b't', b'y', b'p']) = header.box_type else {
            log::trace!("Box type was not `ftyp`: {:?}", header.box_type);
            return None;
        };

        Self::parse_body_only(header, input)
    }

    /// Parses out an `ftyp` box given its header.
    pub fn parse_body_only(header: BoxHeader, input: &mut &[u8]) -> Option<FtypBox> {
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

#[cfg(test)]
mod tests {
    use crate::providers::shared::bmff::ftyp::{FtypBox, parse_fourcc};

    #[test]
    fn fourcc_should_parse() {
        assert_eq!(
            parse_fourcc(&mut b"1234".as_slice()),
            Ok([b'1', b'2', b'3', b'4'])
        );
    }

    #[test]
    fn ftyp_box_should_parse() {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&20_u32.to_be_bytes()); // size
        bytes.extend_from_slice(b"ftyp"); // ty
        bytes.extend_from_slice(b"isom"); // brand
        bytes.extend_from_slice(&1_u32.to_be_bytes()); // minor ver.
        bytes.extend_from_slice(b"isom"); // compat. brands

        assert!(FtypBox::new(&mut bytes.as_slice()).is_some());
    }
}
