use crate::providers::shared::bmff::{BoxHeader, BoxType};

/// Finds all boxes with any of the given types.
pub fn _find_boxes<'borrow, 'types, 'input: 'borrow>(
    input: &'borrow mut &'input [u8],
    types: &'types [BoxType],
) -> Vec<(BoxHeader, &'input [u8])> {
    // here's where we'll store the boxes we find
    let mut boxes: Vec<(BoxHeader, &[u8])> = Vec::new();

    // loop until the input is empty, or we experience an error w/ grabbing
    // some kind of data
    while !input.is_empty() {
        // grab its header
        let Ok(box_header) = BoxHeader::new(input)
            .inspect_err(|e| log::warn!("Failed to construct box header from input. err: {e}"))
        else {
            break;
        };

        // if the list of types has that box's ty, stick it in the list
        if types.contains(&box_header.box_type) {
            // grab its data
            let Some(box_payload) = box_header.payload(input) else {
                break;
            };

            // add to list
            boxes.push((box_header, box_payload));
        } else {
            // if the type isn't there, skip it for the next payload
            let Some(()) = box_header.eat_payload(input) else {
                break;
            };
        }
    }

    boxes
}
