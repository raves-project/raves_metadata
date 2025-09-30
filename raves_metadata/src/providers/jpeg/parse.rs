use std::{collections::BTreeMap, sync::Arc};

use parking_lot::RwLock;
use winnow::{
    Parser,
    binary::{be_u16, be_u32, u8},
    error::EmptyError,
    token::take,
};

use crate::{
    MaybeParsedExif, MaybeParsedXmp, Wrapped,
    xmp::{Xmp, error::XmpError, get_rdf_descriptions},
};

use super::{Jpeg, JpegConstructionError};

/// A marker code indicating that an APP1 marker is present.
const APP1_MARKER_CODE: u8 = 0xE1;

/// The first marker code, `SOI` (start of image).
const SOI_MARKER_CODE: u8 = 0xD8;

/// The last marker code, `EOI` (end of image).
const EOI_MARKER_CODE: u8 = 0xD9;

/// The start of scan code, `SOS`.
const SOS_MARKER_CODE: u8 = 0xDA;

/// A part of a JPEG file.
enum Marker {
    /// A marker with no data.
    Standalone {
        /// An identifier for a marker.
        marker_code: u8,
    },

    /// A marker with a payload and length.
    Full {
        /// An identifier for a marker.
        marker_code: u8,

        /// The length of the marker's payload.
        ///
        /// This value does NOT include the two length bytes.
        len: u16,
    },
}

/// Wrapper since Clippy was complaining about long types.
///
/// It was right. The code was pretty ugly before, lol.
struct JpegXmp {
    /// StandardXMP, which is just the normal one.
    standard: Option<Vec<u8>>,

    /// Any additional XMP chunks, called ExtendedXMP.
    ///
    /// Prefixed w/ their offsets.
    ///
    /// Maps:
    ///
    /// - outer: by GUID
    /// - inner: by offset
    extended: BTreeMap<[u8; 32], BTreeMap<u32, JpegXmpExtended>>,
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
struct JpegXmpExtended {
    total_xmp_len_after_concat: u32,
    offset: u32,
    payload: Vec<u8>,
}

/// Attempts to parse a JPEG file.
pub fn parse(input: &[u8]) -> Result<Jpeg, JpegConstructionError> {
    let input: &mut &[u8] = &mut &*input;

    // take first marker, which should be `SOI`
    match marker(input)? {
        Marker::Standalone { marker_code } if marker_code == SOI_MARKER_CODE => (),
        Marker::Standalone { marker_code } | Marker::Full { marker_code, .. } => {
            log::error!(
                "The first marker of a JPEG file should be `SOI`, \
                but it wasn't! \
                got: `{marker_code:x?}`"
            );
            return Err(JpegConstructionError::FirstMarkerWasNotSoi { marker_code });
        }
    };

    let mut exif: Option<_> = None;
    let mut xmp: Option<JpegXmp> = None;

    // loop until the end of the file.
    while !input.is_empty() {
        let marker: Marker = marker(input)?;

        match marker {
            // handle end of image
            Marker::Standalone { marker_code } if marker_code == EOI_MARKER_CODE => {
                log::trace!("EOI detected! Stopping loop.");
                break;
            }

            // skip other standalone markers
            Marker::Standalone { marker_code } => {
                // note: we don't actually skip anything since we've already
                // consumed the marker code bytes lol
                log::trace!("Got standalone marker with code `{marker_code:x?}`. Skipping...");
            }

            // for the `SOS` marker, we have to consume tokens until we reach
            // the next marker.
            //
            // however, our generic handling won't work for that! we gotta do
            // it w/ special treatment.
            Marker::Full { marker_code, .. } if marker_code == SOS_MARKER_CODE => loop {
                // grab the next two tokens
                let (_, [a, b]) = take(2_usize)
                    .map(|sli| {
                        TryInto::<[u8; 2]>::try_into(sli).unwrap_or_else(|_| std::process::abort())
                    })
                    .map_err(|_: EmptyError| {
                        log::error!("No more tokens in SOS. Can't get tokens.");
                        JpegConstructionError::OuttaDataForSos
                    })
                    .parse_peek(*input)?;

                // if the first byte isn't 0xFF, consume it and keep looking.
                //
                // otherwise, it might be a header
                if a != 0xFF {
                    u8.void()
                        .parse_next(input)
                        .map_err(|_: EmptyError| JpegConstructionError::OuttaDataForSos)?;
                    continue;
                }

                // then, if the second byte is `0x00`, then we know to keep
                // searching.
                //
                // otherwise, fall through to the below cases
                if b == 0x00 {
                    u8.void()
                        .parse_next(input)
                        .map_err(|_: EmptyError| JpegConstructionError::OuttaDataForSos)?;
                    continue;
                }

                // for any present restart markers, ignore them.
                //
                // they just indicate that we should keep going
                if (0xD0..=0xD7).contains(&b) {
                    log::trace!("Hit restart marker! Continuing...");
                    u8.void()
                        .parse_next(input)
                        .map_err(|_: EmptyError| JpegConstructionError::OuttaDataForSos)?;
                    continue;
                }

                // otherwise, it's a new marker!
                //
                // let's exit the loop
                break;
            },

            // check "full" markers to see if it's a marker we care abt
            Marker::Full { marker_code, len } => {
                log::trace!("Got full marker! code: `{marker_code:x?}`, len: `{len}`");

                let remaining_input_len: u64 = input.len() as u64;
                let payload: &mut &[u8] =
                    &mut take(len as usize)
                        .parse_next(input)
                        .map_err(|_: EmptyError| {
                            log::error!(
                                "Attempted to parse payload from JPEG marker, \
                        but ran out of data. \
                        marker code: `{marker_code:x?}`, len: `{len}` bytes, \
                        remaining input len: `{remaining_input_len}` bytes"
                            );
                            JpegConstructionError::NoDataForPayload {
                                marker_code,
                                original_len: len,
                                remaining_input_len,
                            }
                        })?;

                // APP1 can contain Exif and XMP.
                //
                // define strings for both, then check for them!
                const EXIF_SIG: &[u8] = b"Exif\0\0";
                const XMP_SIG: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
                const XMP_EXT_SIG: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
                if marker_code == APP1_MARKER_CODE {
                    // exif
                    if payload.starts_with(EXIF_SIG) {
                        // take the signature
                        take::<_, _, EmptyError>(EXIF_SIG.len())
                            .void()
                            .parse_next(payload)
                            .map_err(|_: EmptyError| {
                                log::error!("Failed to take signature for APP1 Exif blob.");
                                JpegConstructionError::OuttaDataForApp1
                            })?;

                        log::trace!("Found Exif in JPEG!");

                        // set the raw exif value
                        if exif.is_none() {
                            exif = Some(MaybeParsedExif::Raw(payload.to_vec()));
                        } else {
                            log::warn!("Found more than one Exif payload in JPEG...");
                        }
                    }

                    // xmp
                    if payload.starts_with(XMP_SIG) {
                        log::trace!("Found StandardXMP in JPEG!");

                        // take the signature
                        take::<_, _, EmptyError>(XMP_SIG.len())
                            .void()
                            .parse_next(payload)
                            .map_err(|_: EmptyError| {
                                log::error!("Failed to eat signature for StandardXMP.");
                                JpegConstructionError::OuttaDataForApp1
                            })?;

                        // add it to the list.
                        //
                        // we use a list here since JPEGs limit payloads to
                        // u16::MAX, which isn't really big enough for large
                        // XMP payloads.
                        //
                        // so, using "extended XMP", we'll push all of the
                        // payloads, then concatenate them before we return.
                        match xmp {
                            Some(_) => {
                                log::warn!("Found another primary XMP, but we already have one...");
                                return Err(JpegConstructionError::MultipleStandardXmpBlobs);
                            }
                            None => {
                                xmp = Some(JpegXmp {
                                    standard: Some(payload.to_vec()),
                                    extended: BTreeMap::new(),
                                })
                            }
                        };
                    }

                    // extended XMP
                    if payload.starts_with(XMP_EXT_SIG) {
                        log::trace!("Found extended XMP in JPEG!");

                        // take sig
                        () = take::<_, _, EmptyError>(XMP_EXT_SIG.len())
                            .void()
                            .parse_next(payload)
                            .map_err(|_: EmptyError| {
                                log::error!("Failed to eat signature for APP1's ExtendedXMP.");
                                JpegConstructionError::OuttaDataForApp1
                            })?;

                        // take the guid (32 bytes)
                        let guid: [u8; 32] = take(32_usize)
                            .map(|v| {
                                let Ok(arr) = TryInto::<[u8; 32]>::try_into(v) else {
                                    log::error!(
                                        "GUID was somehow not 32 bytes. Please report this!!!"
                                    );
                                    std::process::abort();
                                };
                                arr
                            })
                            .parse_next(payload)
                            .map_err(|_: EmptyError| {
                                log::error!("Failed to eat GUID for APP1's ExtendedXMP.");
                                JpegConstructionError::OuttaDataForApp1
                            })?;

                        // u32 len of the actual payload
                        let total_xmp_len_after_concat: u32 =
                            be_u32.parse_next(payload).map_err(|_: EmptyError| {
                                log::error!("Failed to eat ExtendedXMP total length.");
                                JpegConstructionError::OuttaDataForApp1
                            })?;

                        let offset: u32 = be_u32.parse_next(payload).map_err(|_: EmptyError| {
                            log::error!("Failed to get ExtendedXMP offset.");
                            JpegConstructionError::NoOffsetForExtendedXmp
                        })?;

                        // construct the value we'll insert.
                        let val = JpegXmpExtended {
                            total_xmp_len_after_concat,
                            offset,
                            payload: payload.to_vec(),
                        };

                        // grab the list of extended xmp to mutate.
                        //
                        // then, add it to the list
                        match xmp {
                            // great, we already have some xmp.
                            //
                            // add it to the existing list.
                            Some(JpegXmp {
                                standard: _,
                                ref mut extended,
                            }) => {
                                debug_assert!(
                                    !extended
                                        .get(&guid)
                                        .map(|t| t.contains_key(&offset))
                                        .unwrap_or(false),
                                    "ExtendedXMP may not use the same offset more than once"
                                );

                                extended
                                    .entry(guid)
                                    .and_modify(|inner| {
                                        inner.insert(offset, val.clone());
                                    })
                                    .or_insert(BTreeMap::from([(offset, val)]));
                            }

                            // no standard xmp yet! let's add this extended xmp
                            // anyway.
                            //
                            // note: we may find the standard xmp after
                            // extended. it'd be weird, ofc, but possible!
                            //
                            // TODO: find/make a test file for this exact
                            // situation
                            None => {
                                xmp = Some(JpegXmp {
                                    standard: None,
                                    extended: BTreeMap::from([(
                                        guid,
                                        BTreeMap::from([(offset, val)]),
                                    )]),
                                })
                            }
                        }
                    }
                }
            }
        }
    }

    let xmp = if let Some(x) = xmp {
        if !x.extended.is_empty() {
            Some(concat_xmp(x)?)
        } else {
            x.standard.map(MaybeParsedXmp::Raw)
        }
    } else {
        None
    };

    Ok(Jpeg {
        exif: Arc::new(RwLock::new(exif)),
        xmp: Arc::new(RwLock::new(xmp)),
    })
}

/// Tries to parse out a [`Marker`].
fn marker(input: &mut &[u8]) -> Result<Marker, JpegConstructionError> {
    // each marker must begin with one `0xFF` byte.
    //
    // let's see if that happened...
    let first_marker_byte: u8 = u8.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Failed to get first marker byte!");
        JpegConstructionError::NoMarkerCode
    })?;
    if first_marker_byte != 0xFF {
        log::error!("JPEG marker's first byte was wrong.");
        return Err(JpegConstructionError::FirstMarkerByteWasWrong(
            first_marker_byte,
        ));
    }

    // a marker may have any number of `0xFF`/255 bytes before its code.
    //
    // try to find its code
    let marker_code: u8 = loop {
        let k: u8 = u8.parse_next(input).map_err(|_: EmptyError| {
            log::error!("Failed to parse out marker byte!");
            JpegConstructionError::NoMarkerCode
        })?;

        if k != 0xFF {
            break k;
        }
    };

    // then, parse the code
    if [0_u8, 0xFF_u8].contains(&marker_code) {
        log::error!(
            "This JPEG marker byte was either `0` or `255`, but these \
            values are disallowed."
        );
        return Err(JpegConstructionError::MarkerCodeDisallowed(marker_code));
    }

    // some markers are "standalone" markers and don't have any payload (or
    // the length of that payload).
    //
    // for that reason, early return if we encounter one...
    const STANDALONE_MARKERS: &[u8] = &[
        0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0x01,
    ];
    if STANDALONE_MARKERS.contains(&marker_code) {
        return Ok(Marker::Standalone { marker_code });
    }

    // alright, we've taken care of any standalone markers.
    //
    // let's check the length of the payload, then return
    let original_len: u16 = be_u16.parse_next(input).map_err(|_: EmptyError| {
        log::error!("Failed to find `u16` length byte pair when parsing marker.");
        JpegConstructionError::NoLength { marker_code }
    })?;

    // subtract 2 bytes from that (b/c the length includes its own bytes lol)
    let len: u16 =
        original_len
            .checked_sub(2_u16)
            .ok_or(JpegConstructionError::NegativeLength {
                marker_code,
                original_len,
            })?;

    Ok(Marker::Full { marker_code, len })
}

/// Concatenates all our XMP data so we may return it.
fn concat_xmp(jpeg_xmp: JpegXmp) -> Result<MaybeParsedXmp, JpegConstructionError> {
    log::trace!("Concatenating XMP...");

    debug_assert!(
        !jpeg_xmp.extended.is_empty(),
        "we should have extended xmp \
        if calling this fn"
    );

    // log all the found guids
    for (guid, v) in jpeg_xmp.extended.iter() {
        log::trace!("For GUID `{guid:x?}`, found `{}` entries.", v.len());
    }

    // parse the Standard XMP for its contained GUID.
    //
    // then, edit the XMP to not have the GUID
    let (standard_xmp, guid) = {
        // welp, let's start parsing.
        //
        // we need to have `StandardXMP` by this point.
        //
        // let's ensure that's the case
        let Some(standard_xmp) = jpeg_xmp.standard else {
            log::warn!("ExtendedXMP was found in the file, but no StandardXMP!");
            return Err(JpegConstructionError::CantConcatExtendedXmpWithoutStandardXmp);
        };

        let parsed_standard_xmp: xmltree::Element =
            xmltree::Element::parse(standard_xmp.as_slice()).map_err(|e| {
                JpegConstructionError::XmpDidntParse(crate::xmp::error::XmpError::XmlParseError(
                    Arc::new(e),
                ))
            })?;

        // FIXME: this function doesn't actually remove it yet!
        //
        // we should make it do that when XMP gains write support
        get_and_remove_guid(parsed_standard_xmp)?
    };
    log::trace!("Primary GUID: `{guid:x?}`");

    // let's grab the `ExtendedXMP`, too
    let mut extended_xmp = jpeg_xmp.extended;

    // now, let's form the list and check offsets.
    //
    // start by initializing those
    let mut total_offset: u32 = 0_u32;
    let mut good: Vec<Vec<u8>> = Vec::with_capacity(extended_xmp.len());

    // grab the extended chunks with the same guid.
    let mut relevent_extended_chunks: BTreeMap<u32, JpegXmpExtended> =
        extended_xmp.remove(&guid).unwrap_or_default();

    // in debug mode, check sanity-ck our extended chunks
    if cfg!(debug_assertions) {
        let mut n = None;
        for v in relevent_extended_chunks.values() {
            match n {
                Some(n) => debug_assert_eq!(
                    n, v.total_xmp_len_after_concat,
                    "all chunks should agree on total xmp document len"
                ),
                None => n = Some(v.total_xmp_len_after_concat),
            }
        }
    }

    // now, let's increment the total offset to ensure each following offset is
    // correct.
    //
    // otherwise, the JPEG was written incorrectly!
    while !relevent_extended_chunks.is_empty() {
        let Some(chunk): Option<JpegXmpExtended> = relevent_extended_chunks.remove(&total_offset)
        else {
            log::error!("Missing Extended XMP chunk! offset: `{total_offset}`");
            return Err(JpegConstructionError::ExtendedXmpMissingChunk {
                offset: total_offset,
            });
        };

        debug_assert_eq!(
            chunk.offset, total_offset,
            "should be equal unless jpeg file is malformed"
        );

        total_offset += chunk.payload.len() as u32;
        good.push(chunk.payload);
    }

    // now, parse both into xmp
    let standard = Xmp::new(core::str::from_utf8(standard_xmp.as_slice()).map_err(|e| {
        log::error!("Failed to convert standard XMP to UTF-8! err: {e}");
        JpegConstructionError::XmpDidntParse(XmpError::NotUtf8)
    })?)
    .map_err(|e| {
        log::error!("Failed to parse standard XMP! err: {e}");
        JpegConstructionError::XmpDidntParse(e)
    })?;

    let extended = Xmp::new(
        core::str::from_utf8(good.into_iter().flatten().collect::<Vec<_>>().as_slice()).map_err(
            |e| {
                log::error!("Failed to convert standard XMP to UTF-8! err: {e}");
                JpegConstructionError::XmpDidntParse(XmpError::NotUtf8)
            },
        )?,
    )
    .map_err(|e| {
        log::error!("Failed to parse standard XMP! err: {e}");
        JpegConstructionError::XmpDidntParse(e)
    })?;

    // then, concatenate them and return!
    let concat: Xmp = standard.combine(extended);
    Ok(MaybeParsedXmp::Parsed(Wrapped(Arc::new(RwLock::new(
        concat,
    )))))
}

/// Gets and removes the GUID from the given standard XMP.
///
/// Doing so is required by the XMP specification.
fn get_and_remove_guid(
    parsed_standard_xmp: xmltree::Element,
) -> Result<(Vec<u8>, [u8; 32]), JpegConstructionError> {
    // remove a specific tag, `xmpNote:HasExtendedXMP`, from the xmp.
    //
    // in xmp, it can be in two places:
    //
    // 1. a child of `root`
    // 2. an attribute on `root`
    //
    // let's check both.
    let Some(guid_str) = ('check: {
        const XMP_NOTE_NAMESPACE: &str = "http://ns.adobe.com/xmp/note/";
        const XMP_NOTE_PREFIX: &str = "xmpNote";
        const HAS_EXTENDED_XMP_NAME: &str = "HasExtendedXMP";

        let mut guid = None;

        // FIXME: this doesn't actually perform the removal.
        for desc in get_rdf_descriptions(&parsed_standard_xmp).map_err(|e| {
            log::error!("No RDF descriptions! err: {e}");
            JpegConstructionError::ExtendedXmpCouldntFindGuid
        })? {
            // check attributes
            let mut attr_key_to_remove = None;
            for owned_name in desc.attributes.keys() {
                if owned_name.local_name == HAS_EXTENDED_XMP_NAME
                    && (owned_name.namespace_ref() == Some(XMP_NOTE_NAMESPACE)
                        || owned_name.prefix_ref() == Some(XMP_NOTE_PREFIX))
                {
                    attr_key_to_remove = Some(owned_name.clone());
                    break;
                }
            }

            if let Some(k) = attr_key_to_remove {
                let rmd = desc.attributes.clone().remove(&k);
                guid = rmd;
                break 'check guid;
            }

            // check children
            let mut child_idx_to_remove = None;
            for (child_idx, child) in desc.children.iter().enumerate() {
                if let Some(elem) = child.as_element()
                    && elem
                        .namespace
                        .as_ref()
                        .map(|s| s.as_str() == XMP_NOTE_NAMESPACE)
                        .unwrap_or(false)
                    && elem.name == HAS_EXTENDED_XMP_NAME
                {
                    child_idx_to_remove = Some(child_idx);
                    guid = elem.get_text().map(|c| ToString::to_string(&c));
                    break;
                }
            }

            if let Some(i) = child_idx_to_remove {
                desc.children.clone().remove(i);
                break 'check guid;
            }
        }

        guid
    }) else {
        log::error!("No GUID for extended XMP!");
        return Err(JpegConstructionError::ExtendedXmpCouldntFindGuid);
    };

    // it should be 32 chars long (16 hex numbers)
    debug_assert_eq!(
        guid_str.len(),
        32,
        "GUID must be 32 hex numbers in ASCII text"
    );
    let guid: [u8; 32] = guid_str.as_bytes().try_into().map_err(|e| {
        log::error!("GUID was the wrong length. err: {e}");
        JpegConstructionError::ExtendedXmpGuidNot32Bytes(guid_str)
    })?;
    debug_assert_eq!(
        guid.len(),
        32,
        "GUID as bytes should still be 32 char long."
    );

    // finally, write it back into the standard xmp :)
    let mut standard_xmp: Vec<u8> = Vec::new();
    if let Err(e) = parsed_standard_xmp.write(&mut standard_xmp) {
        log::error!("Failed to write back standard XMP after editing! err: {e}");
        return Err(JpegConstructionError::ExtendedXmpWriteFailure);
    };

    Ok((standard_xmp, guid))
}
