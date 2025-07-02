use std::sync::Arc;

/// An error that occurred while parsing IPTC.
#[derive(Debug)]
pub enum IptcError {
    Iptc4Xmp(Iptc4XmpError),
}

/// This is an error that happened while we were parsing IPTC through XMP.
#[derive(Clone, Debug)]
pub enum Iptc4XmpError {
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

impl core::fmt::Display for Iptc4XmpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Iptc4XmpError::XmlParseError(e) => {
                write!(f, "Encountered error while parsing XML. err: {e}")
            }

            Iptc4XmpError::NoRdfElement => {
                f.write_str("The XML is missing the `rdf:Rdf` element, which is required.")
            }

            Iptc4XmpError::NoDescriptionElements => f.write_str(
                "The `rdf:Rdf` element has no `rdf:Description` elements. \
                    One or more are required.",
            ),
        }
    }
}

impl core::error::Error for Iptc4XmpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Iptc4XmpError::XmlParseError(e) => Some(e.as_ref()),
            Iptc4XmpError::NoRdfElement | Iptc4XmpError::NoDescriptionElements => None,
        }
    }
}

impl From<xmltree::ParseError> for Iptc4XmpError {
    fn from(value: xmltree::ParseError) -> Self {
        Iptc4XmpError::XmlParseError(value.into())
    }
}
