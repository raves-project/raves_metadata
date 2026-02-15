//! QuickTime File Format (QTFF) movie files, also known as the `.mov` format.
//!
//! `MOV` is very similar to `MP4`, as it follows the same box structure. It
//! does, however, refer to boxes as "atoms", which is practically a semantic
//! difference instead of a behavioral one.

use winnow::{Parser, error::EmptyError, token::take};

use crate::{
    MetadataProvider,
    providers::shared::bmff::{BoxHeader, BoxSize, BoxType, XMP_BOX_ID, XMP_UUID, ftyp::FtypBox},
    xmp::{Xmp, error::XmpError},
};

/// A QuickTime File Format (QTFF) movie file.
///
/// Contains XMP metadata exclusively.
#[derive(Clone, Debug)]
pub struct Mov {
    xmp: Option<Result<Xmp, XmpError>>,
}

/// Parses the `ftyp` atom from the QuickTime file, if possible.
///
/// Used to implement `parse` and `magic_number`.
fn parse_ftyp(input: &[u8]) -> Result<(), MovConstructionError> {
    // try finding an `ftyp` box
    let mut ftyp_search_input: &[u8] = input;

    while !ftyp_search_input.is_empty() {
        // grab header of this closest atom
        let atom_header: BoxHeader = match BoxHeader::new(&mut ftyp_search_input) {
            Ok(h) => h,
            Err(e) => {
                log::debug!(
                    "Failed to parse atom header while looking for `.mov` `ftyp` atom. \
                        Giving up. \
                        err: {e}"
                );
                break;
            }
        };
        log::trace!("found an atom when looking for `ftyp` header: {atom_header:#?}");

        // only continue if we have a payload length (otherwise, give up on
        // finding the ftyp; it's not here lol)
        let Some(payload_len) = atom_header.payload_len() else {
            log::warn!(
                "Hit `Eof` atom in `.mov` provider, so the parser \
                    never found an `ftyp` atom. \
                    Continuing without verifying file type..."
            );
            break;
        };

        // only continue if it's an `ftyp`.
        //
        // otherwise, skip all the bytes and check the next atom
        if atom_header.box_type != BoxType::Id(*b"ftyp") {
            let Ok(()) = take::<_, _, EmptyError>(payload_len)
                .void()
                .parse_next(&mut ftyp_search_input)
            else {
                return Err(MovConstructionError::NotAMov(None));
            };
            continue;
        }

        // alright, it's an `ftyp`!
        //
        // grab it for the below logic...
        let Some(ftyp_atom) = FtypBox::parse_body_only(atom_header, &mut ftyp_search_input) else {
            log::error!(
                "Found what's supposed to be an `ftyp` atom, but its data \
                    didn't match! Continuing without verifying file type..."
            );
            break;
        };
        log::trace!("Found an `ftyp` box: {ftyp_atom:#?}");

        // whew! we've finally got the ftyp box...
        // let's ensure this is a supported file.
        //
        // note that QuickTime only has one supported `ftyp` value. see:
        //
        // https://developer.apple.com/documentation/quicktime-file-format/file_type_compatibility_atom
        const MOV_FORMATS: &[&[u8; 4]] = &[b"qt  "];
        let major_is_mov = MOV_FORMATS.contains(&&ftyp_atom.major_brand);
        let compat_with_mov = MOV_FORMATS
            .iter()
            .any(|fourcc| ftyp_atom.compatible_brands.contains(fourcc));

        if !(major_is_mov || compat_with_mov) {
            log::warn!(
                "The provided file is not an MOV file. \
                    major_brand: `{}`, \
                    compatible_brands: `{:?}`",
                core::str::from_utf8(&ftyp_atom.major_brand).unwrap_or_default(),
                ftyp_atom
                    .compatible_brands
                    .iter()
                    .map(|fourcc: &[u8; 4]| core::str::from_utf8(fourcc))
            );
            return Err(MovConstructionError::NotAMov(Some(ftyp_atom.major_brand)));
        }
    }

    Ok(())
}

/// Parses out metadata from an MOV file.
fn parse(mut input: &[u8]) -> Result<Mov, MovConstructionError> {
    log::trace!("MOV given input w/ len: `{}` bytes", input.len());

    // check the type of the file (should be a MOV)
    parse_ftyp(input)?;

    // check all the other boxes until we find what we want!
    let xmp: Option<&[u8]> = parse_atoms_until_xmp(&mut input);

    Ok(Mov {
        xmp: xmp.map(Xmp::new_from_bytes),
    })
}

/// Parses atoms until it finds an XMP atom, including recursively.
fn parse_atoms_until_xmp<'input>(input: &mut &'input [u8]) -> Option<&'input [u8]> {
    // parse until input is empty
    while !input.is_empty() {
        // try grabbing next atom
        let atom: BoxHeader = match BoxHeader::new(input) {
            Ok(ah) => ah,
            Err(e) => {
                log::error!(
                    "Failed to get header for this atom! \
                    Stopping search. err: {e}"
                );
                break;
            }
        };
        log::trace!("Found an atom! type: {:?}", atom.box_type);

        // grab its length
        let len: u64 = atom.payload_len().unwrap_or(input.len() as u64);
        if len > input.len() as u64 {
            log::error!(
                "Given payload length longer than input! Your file may be \
                corrupted, or this may be a bug. \
                Please report it to the `raves_metadata` developers."
            );
            return None;
        }

        // parse it recursively
        let recursed: Option<&[u8]> = recurse_until_xmp(&atom, &mut &input[..len as usize]);
        if recursed.is_some() {
            return recursed;
        }

        // if the atom is `Eof`, stop looping (b/c we're done)
        if atom.box_size == BoxSize::Eof {
            break;
        }

        // since we didn't get any data out of it, skip the entire recursive
        // payload so we can load the next one
        _ = take::<_, _, EmptyError>(len).parse_next(input);
    }

    None
}

/// Parses QuickTime atoms recursively to find an XMP atom.
///
/// Multiple may exist, but we'll only choose the first we find (for now).
fn recurse_until_xmp<'input>(
    atom: &BoxHeader,
    atom_payload: &mut &'input [u8],
) -> Option<&'input [u8]> {
    let len: u64 = atom.payload_len().unwrap_or(atom_payload.len() as u64);
    if len > atom_payload.len() as u64 {
        log::error!(
            "Given payload length longer than input! Your file may be \
            corrupted, or this may be a bug. \
            Please report it to the `raves_metadata` developers."
        );
        return None;
    }

    // based on the atom's type, decide what to do...
    const CONTAINER_ATOMS: &[[u8; 4]] = &[*b"meta", *b"moov", *b"trak", *b"udta"];
    match atom.box_type {
        BoxType::Id(XMP_BOX_ID) | BoxType::Uuid(XMP_UUID) => {
            log::trace!("found XMP atom! ty: {:?}", atom.box_type);

            let maybe_payload: Option<&'input [u8]> = take::<_, _, EmptyError>(len)
                .parse_next(atom_payload)
                .inspect_err(|_| log::error!("Couldn't get XMP payload from atom!"))
                .ok();

            if let Some(payload) = maybe_payload {
                return Some(payload);
            } else {
                return None;
            }
        }

        BoxType::Id(other) if CONTAINER_ATOMS.contains(&other) => {
            log::trace!("Found container atom! Recursing... {:?}", atom.box_type);

            while !atom_payload.is_empty() {
                let next_atom_under_container: BoxHeader = match BoxHeader::new(atom_payload) {
                    Ok(ah) => ah,
                    Err(e) => {
                        log::error!(
                            "Failed to get header for this atom! \
                            Giving up... err: {e}"
                        );
                        return None;
                    }
                };

                let next_atom_len: u64 = next_atom_under_container
                    .payload_len()
                    .unwrap_or(atom_payload.len() as u64);

                if next_atom_len > atom_payload.len() as u64 {
                    log::error!(
                        "Given payload length longer than input! Your file may be \
                        corrupted, or this may be a bug. \
                        Please report it to the `raves_metadata` developers."
                    );
                    return None;
                }

                let next_atom_payload: &mut &[u8] = &mut &atom_payload[..next_atom_len as usize];

                if let Some(xmp_blob) =
                    recurse_until_xmp(&next_atom_under_container, next_atom_payload)
                {
                    return Some(xmp_blob);
                }

                take::<_, _, EmptyError>(next_atom_len)
                    .void()
                    .parse_next(atom_payload)
                    .unwrap_or_else(|_| {
                        log::error!(
                            "Payload was longer than slice. payload: `{}`, slice length: `{}`",
                            next_atom_len,
                            atom_payload.len()
                        );
                    });

                log::trace!("Recursion complete, but no matching value was found.");
            }
        }

        // ignore other box types
        ref other => {
            log::trace!(
                "skipping other atom ty: {other:?}, size: {:?}",
                atom.box_size
            );

            // skip their internals
            _ = take::<_, _, EmptyError>(len)
                .parse_next(atom_payload)
                .inspect_err(|_| {
                    log::error!(
                        "Payload was longer than slice. payload: `{}`, slice length: `{}`",
                        len,
                        atom_payload.len()
                    );
                })
                .ok();
        }
    }

    None
}

impl MetadataProvider for Mov {
    type ConstructionError = MovConstructionError;

    fn magic_number(input: &[u8]) -> bool {
        parse_ftyp(input).is_ok()
    }

    /// Parses a `MOV` file for its metadata.
    fn new(
        input: &impl AsRef<[u8]>,
    ) -> Result<Self, <Self as MetadataProvider>::ConstructionError> {
        parse(input.as_ref())
    }

    fn exif(&self) -> &Option<Result<crate::exif::Exif, crate::exif::error::ExifFatalError>> {
        &None
    }

    fn xmp(&self) -> &Option<Result<Xmp, XmpError>> {
        &self.xmp
    }
}

/// An error that occurred when parsing a QuickTime/`MOV` file.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum MovConstructionError {
    /// The given file isn't actually an MOV file.
    ///
    /// Its filetype info may have denoted that it's something else.
    NotAMov(Option<[u8; 4]>),
}

impl core::fmt::Display for MovConstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        const NOT_A_MOV_MSG: &str = "The given input isn't a QuickTime/MOV file! File type was";

        match self {
            MovConstructionError::NotAMov(None) => f.write_str(NOT_A_MOV_MSG),
            MovConstructionError::NotAMov(Some(ftyp)) => match core::str::from_utf8(ftyp) {
                Ok(utf8_ftyp) => write!(f, "{NOT_A_MOV_MSG}: `{ftyp:?}`. (UTF-8: `{utf8_ftyp}`)"),
                Err(_) => write!(f, "{NOT_A_MOV_MSG}: `{ftyp:?}`. (Type was not UTF-8.)"),
            },
        }
    }
}

impl core::error::Error for MovConstructionError {}

#[cfg(test)]
mod tests {
    use raves_metadata_types::xmp::{XmpElement, XmpPrimitive, XmpValue};

    use crate::{MetadataProvider, providers::mov::Mov, util::logger};

    /// Ensures that a real `.mov` parses correctly and yields its XMP metadata.
    #[test]
    fn real_mov_file_should_parse_and_yield_xmp() {
        logger();

        let bytes = include_bytes!("../../assets/providers/mov/QuickTime.mov");
        let mov: Mov = Mov::new(bytes).expect("mov should parse correctly");

        let xmp = mov
            .xmp()
            .clone()
            .expect("the file contains xmp")
            .expect("the xmp ctor should succeed");

        assert_eq!(
            xmp.document()
                .values_ref()
                .iter()
                .find(|v| v.name == "creator")
                .expect("should be a creator field")
                .value,
            XmpValue::OrderedArray(vec![XmpElement {
                namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                prefix: "rdf".into(),
                name: "li".into(),
                value: XmpValue::Simple(XmpPrimitive::Text("Phil Harvey".into()))
            }])
        );
    }
}
