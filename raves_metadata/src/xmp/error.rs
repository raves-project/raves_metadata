// /// Can occur while parsing XMP metadata into values.

use std::{
    borrow::Cow,
    num::{ParseFloatError, ParseIntError},
};

use raves_metadata_types::{
    xmp::{XmpElement, XmpValue},
    xmp_parsing_types::XmpKindStructField,
};

pub type XmpValueResult<'xml> = Result<XmpValue<'xml>, XmpParsingError<'xml>>;
pub type XmpElementResult<'xml> = Result<XmpElement<'xml>, XmpParsingError<'xml>>;

use std::sync::Arc;

/// This is an error that happened while we were parsing XMP.
#[derive(Clone, Debug)]
pub enum XmpError {
    /// `xmltree` failed to parse the XML.
    XmlParseError(
        // note: `Arc` allows us to impl `Clone`
        Arc<xmltree::ParseError>,
    ),

    /// Initial XML scanning failed - no `rdf:Rdf` element was found.
    NoRdfElement,

    /// We couldn't find any `rdf:Description` elements in the `rdf:Rdf`
    /// element.
    NoDescriptionElements,
}

impl core::fmt::Display for XmpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            XmpError::XmlParseError(e) => {
                write!(f, "Encountered error while parsing XML. err: {}", e)
            }

            XmpError::NoRdfElement => {
                f.write_str("The XML is missing the `rdf:Rdf` element, which is required.")
            }

            XmpError::NoDescriptionElements => f.write_str(
                "The `rdf:Rdf` element has no `rdf:Description` elements. \
                    One or more are required.",
            ),
        }
    }
}

impl core::error::Error for XmpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            XmpError::XmlParseError(e) => Some(e.as_ref()),
            XmpError::NoRdfElement | XmpError::NoDescriptionElements => None,
        }
    }
}

impl From<xmltree::ParseError> for XmpError {
    fn from(value: xmltree::ParseError) -> Self {
        XmpError::XmlParseError(value.into())
    }
}

/// This error occurred in internal parsing.
///
/// We use it for better diagnostics. Note that these are usually converted
/// into `None` with `.inspect_err(log::error!(/* ... */)).ok()`, which
/// provides logs, but doesn't give the user direct error values to sift
/// through.
#[derive(Debug)]
pub enum XmpParsingError<'xml> {
    //
    //
    //
    //
    //
    // `XmpElement` creation
    //
    /// Couldn't create an `XmpElement` from the `self: &Element` and
    /// `value: Value` pair, as `self` lacks a namespace.
    XmpElementCreationNoNamespace { element_name: Cow<'xml, str> },

    /// Same as above, except `self` lacks a prefix.
    XmpElementCreationNoPrefix { element_name: Cow<'xml, str> },

    //
    //
    //
    //
    //
    // related to primitive parsing
    //
    /// A primitive was given, and we were told to parse out a Boolean.
    ///
    /// However, it wasn't a matching value! The contained value was what we
    /// got.
    PrimitiveUnknownBool(Cow<'xml, str>),

    /// We were told to parse out an Integer, but it failed to parse
    /// correctly. Contained value is what we got and the `core` parsing error.
    PrimitiveIntegerParseFail(Cow<'xml, str>, ParseIntError),

    /// We were told to parse out a float (Real), but didn't parse right.
    PrimitiveRealParseFail(Cow<'xml, str>, ParseFloatError),

    /// A primitive with a known text value had no text.
    PrimitiveTextHadNoText { element_name: Cow<'xml, str> },

    //
    //
    //
    //
    //
    // union parsing
    //
    /// Unions are currently expected to have only a `Text` discriminant, but
    /// this value was described by another `Kind`.
    UnionDiscriminantWasntText {
        element_name: Cow<'xml, str>,
        discriminant_kind: &'static XmpKindStructField,
    },

    /// The union had no discriminant, so we couldn't see how to parse it.
    UnionNoDiscriminant { element_name: Cow<'xml, str> },

    //
    //
    //
    //
    //
    // array parsing
    //
    /// Couldn't find an inner collection type, like `rdf:Alt`, `rdf:Bag` or
    /// `rdf:Seq`.
    ArrayNoInnerCollectionType {
        element_name: Cow<'xml, str>,
        children: Cow<'xml, [xmltree::XMLNode]>,
    },

    /// "Alternatives" arrays must have a default value.
    ///
    /// This one didn't.
    ArrayAltNoDefault {
        element_name: Cow<'xml, str>,
        alternatives_array: Cow<'xml, [(Cow<'xml, str>, XmpElement<'xml>)]>,
    },

    //
    //
    //
    //
    //
    //
    // generic (schema-less) parsing error variants
    //
    /// We couldn't get the text for an element that was expected to be a
    /// primitive.
    GenericLikelyPrimitiveHadNoText { element_name: Cow<'xml, str> },

    /// We looked through all the possible types this value could have, but it
    /// simply had no information inside it.
    ///
    /// Thus, we returned a blank type. (which isn't useful at all)
    GenericNoOtherOption { element_name: Cow<'xml, str> },
}

impl core::fmt::Display for XmpParsingError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            //
            //
            //
            //
            // element creation
            //
            XmpParsingError::XmpElementCreationNoNamespace { element_name } => write!(
                f,
                "The XML element `{element_name}` has no namespace. \
                    Couldn't create an `XmpElement`.",
            ),
            XmpParsingError::XmpElementCreationNoPrefix { element_name } => write!(
                f,
                "The XML element `{element_name}` has a namespace, but no prefix. \
                    Couldn't create an `XmpElement`.",
            ),
            //
            //
            //
            //
            //
            //
            //
            // prim parsing
            //
            XmpParsingError::PrimitiveUnknownBool(got) => write!(
                f,
                "Asked to parse out a Boolean, but the stored value wasn't \
                        an expected answer. \
                Instead, it was: `{got}`",
            ),
            XmpParsingError::PrimitiveIntegerParseFail(got, parse_int_err) => write!(
                f,
                "Asked to parse out an Integer, but the stored value wasn't right. \
                    - got: `{got}`, \
                    - err: {parse_int_err}",
            ),
            XmpParsingError::PrimitiveRealParseFail(got, parse_float_err) => write!(
                f,
                "Asked to parse out a Real, but the stored value wasn't right. \
                    - got: `{got}`, \
                    - err: {parse_float_err}",
            ),
            XmpParsingError::PrimitiveTextHadNoText { element_name } => write!(
                f,
                "Element `{element_name}` was a `Primitive::Text` kind, but didn't provide text.",
            ),
            //
            //
            //
            //
            //
            //
            //
            // unions
            //
            XmpParsingError::UnionDiscriminantWasntText {
                element_name,
                discriminant_kind,
            } => write!(
                f,
                "Union `{element_name}` had a discriminant, but it wasn't `Kind::Simple(Prim::Text)`! \
                found kind: {discriminant_kind:#?}",
            ),
            XmpParsingError::UnionNoDiscriminant { element_name } => write!(
                f,
                "Element `{element_name}` was a `Primitive::Text` kind, but didn't provide text.",
            ),
            //
            //
            //
            //
            //
            //
            //
            // arrays
            //
            XmpParsingError::ArrayNoInnerCollectionType {
                element_name,
                children,
            } => write!(
                f,
                "Array `{element_name}` had no inner collection type! \
                    - known child elements: {children:#?}",
            ),
            XmpParsingError::ArrayAltNoDefault {
                element_name,
                alternatives_array,
            } => write!(
                f,
                "Alternatives array `{element_name}` had alternatives, but didn't \
                specify a default! \
                    - found alternatives: {alternatives_array:#?}",
            ),
            //
            //
            //
            //
            //
            //
            //
            // generic heuristics
            //
            XmpParsingError::GenericLikelyPrimitiveHadNoText { element_name } => write!(
                f,
                "Generic element `{element_name}` was a primitive, but didn't provide text.",
            ),
            XmpParsingError::GenericNoOtherOption { element_name } => {
                write!(f, "Generic element `{element_name}` was blank.",)
            }
        }
    }
}

impl core::error::Error for XmpParsingError<'_> {}
