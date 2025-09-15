use winnow::{Parser, error::EmptyError, token::take};

use crate::providers::shared::bmff::{BoxSize, BoxType, heif::iinf::FullBox};

/// Finds all all `meta` boxes.
pub fn find_meta_boxes<'borrow, 'types, 'input: 'borrow>(
    input: &'borrow mut &'input [u8],
) -> Vec<(FullBox, &'input [u8])> {
    // here's where we'll store the boxes we find
    let mut boxes: Vec<(FullBox, &[u8])> = Vec::new();

    // loop until the input is empty, or we experience an error w/ grabbing
    // some kind of data
    while !input.is_empty() {
        // grab its extends (full box + header)
        let Ok(full_box) = FullBox::new(input)
            .inspect_err(|e| log::warn!("Failed to construct box header from input. err: {e}"))
        else {
            break;
        };
        log::trace!("Found full box w/ type: {:?}", full_box.extends.box_type);

        /// A full box's size, including its `Box`.
        ///
        /// We should subtract that from the full size to get a payload
        /// size, not including the full box (i.e., just sub-boxes).
        const fn fullbox_header_len(sz: &BoxSize) -> usize {
            match sz {
                BoxSize::Small(_) => 8 + 4,
                BoxSize::Large(_) => 16 + 4,
                BoxSize::Eof => 12,
            }
        }

        // check its type
        if full_box.extends.box_type == BoxType::Id(*b"meta") {
            // grab its payload.
            //
            // the payload lasts from the beginning of the slice (from which we
            // just took the full box) until the payload len of the full box.
            //
            // however, we have to calculate that ourselves due to the class
            // inheritance junk used in the standard
            let header_len = fullbox_header_len(&full_box.extends.box_size);
            let payload_size = match full_box.extends.box_size {
                BoxSize::Small(s) => s.saturating_sub(header_len as u32) as usize,
                BoxSize::Large(l) => {
                    (l.saturating_sub(header_len as u64) as usize).min(input.len())
                }
                BoxSize::Eof => input.len(),
            };

            if payload_size > input.len() {
                log::warn!("Box payload size exceeds remaining input! Skipping...");
                break;
            }

            let payload = &input[..payload_size];

            // add that into the list as a tuple
            boxes.push((full_box, payload));
        } else {
            let header_len = fullbox_header_len(&full_box.extends.box_size);
            let skip_amount = match full_box.extends.box_size {
                BoxSize::Small(s) => s.saturating_sub(header_len as u32) as usize,
                BoxSize::Large(l) => {
                    (l.saturating_sub(header_len as u64) as usize).min(input.len())
                }
                BoxSize::Eof => input.len(),
            };

            let skip_res = take::<_, _, EmptyError>(skip_amount).parse_next(input);
            if cfg!(debug_assertions) {
                skip_res.expect("skipping should never fail");
            }
        }
    }

    boxes
}
