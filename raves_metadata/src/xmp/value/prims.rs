use std::borrow::Cow;

use raves_metadata_types::{
    xmp::{XmpPrimitive, XmpValue},
    xmp_parsing_types::XmpPrimitiveKind as Prim,
};

use crate::xmp::error::{XmpParsingError, XmpValueResult};

/// Parses a text value known to be a primitive of a certain kind.
///
/// This is also called with `prim: Prim::Text` when the kind isn't actually
/// known.
pub fn parse_primitive<'xml>(text: Cow<'xml, str>, prim: &Prim) -> XmpValueResult<'xml> {
    Ok(match prim {
        Prim::Boolean => XmpValue::Simple(XmpPrimitive::Boolean(match &*text {
            "True" => true,
            "False" => false,
            other => {
                log::warn!("Encountered unknown boolean value: `{other}`.");
                return Err(XmpParsingError::PrimitiveUnknownBool(Cow::clone(&text)));
            }
        })),

        Prim::Date => XmpValue::Simple(XmpPrimitive::Date(text)),

        Prim::Integer => {
            let num = text.parse::<i64>()
                .map(XmpPrimitive::Integer)
                .or_else(|e | {
                    if [core::num::IntErrorKind::NegOverflow, core::num::IntErrorKind::PosOverflow].contains(e.kind())  {
                        log::warn!("Given number too large for `i64`. Will be exposed as a `Prim::Text`. value: `{}`", text);
                        Ok(XmpPrimitive::Text(Cow::clone(&text)))
                    } else { Err(e) }
                })
                .inspect_err(|e| {
                    log::error!(
                        "Unable to convert integer value `{text}` into integer. \
                            err: {e}"
                    )
                })
                .map_err(|e| XmpParsingError::PrimitiveIntegerParseFail(text, e))
                .map(XmpValue::Simple);

            num?
        }

        Prim::Real => XmpValue::Simple(XmpPrimitive::Real(
            text.parse()
                .inspect_err(|e| {
                    log::error!(
                        "Unable to convert integer value `{text}` into float. \
                            err: {e}"
                    )
                })
                .map_err(|e| XmpParsingError::PrimitiveRealParseFail(text, e))?,
        )),

        Prim::Text => XmpValue::Simple(XmpPrimitive::Text(text)),
    })
}
