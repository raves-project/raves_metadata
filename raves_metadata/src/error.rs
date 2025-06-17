use std::sync::Arc;

/// An error that occurred while parsing IPTC.
pub enum IptcError {
    ParsingFailedBadNameTodo,
}

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
