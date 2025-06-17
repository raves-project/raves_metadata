//! Implements an XMP parser.
//!
//! This module provides functionality to parse XMP (Extensible Metadata
//! Platform) data. XMP is, as the name implies, extensible, so this parser
//! has a unique design for easy extension.
//!
//! In the `raves_metadata_types` crate, there's a module with schema-style
//! types in a giant HashMap. Each entry represents **how to parse** a specific
//! XMP element in a larger namespace.
//!
//! If you're looking to add additional namespaces or elements, please visit
//! the `raves_metadata_types` crate before touching anything here.
//!
//! Afterward, if you're missing the ability to specify how to parse something
//! with those existing types, modify the parsing types and make additional
//! changes here.

extern crate alloc;

use alloc::borrow::Cow;

use raves_metadata_types::xmp::XmpValue;
use xmltree::{AttributeName, Element};

use crate::error::XmpError;

pub mod iptc4xmp;

/// An XMP document.
pub struct XmpDocument<'xml>(Vec<XmpValue<'xml>>);

impl<'xml> XmpDocument<'xml> {
    fn new(values: Vec<XmpValue<'xml>>) -> Self {
        Self(values)
    }

    /// Returns the XMP values in this document.
    pub fn values_ref(&self) -> &[XmpValue<'_>] {
        &self.0
    }

    /// Returns a mutable reference to the XMP values in this document.
    ///
    /// Note that adjusting these values will not immediately affect the
    /// underlying file.
    ///
    /// You'll need to save the document back to the format after making
    /// changes.
    ///
    /// Also, values you may set might not be valid XMP - please use this
    /// method with care.
    pub fn values_mut<'here>(&'here mut self) -> &'here mut [XmpValue<'xml>] {
        &mut self.0
    }

    // TODO: add some better ways to mutate the document's values
}

/// An XMP parser.
pub struct Xmp {
    document: Element,
}

impl Xmp {
    /// Parses the given raw XML string into a collection of XMP values.
    pub fn new(raw_xml: &str) -> Result<Self, XmpError> {
        // grab the document from XML
        let document: Element = Element::parse(raw_xml.as_bytes())?;

        // save it in the struct for use in
        Ok(Self { document })
    }

    /// Returns the underlying XML document.
    pub fn document(&self) -> &Element {
        &self.document
    }

    /// Parses the XMP document and returns a collection of XMP values.
    pub fn parse(&self) -> Result<XmpDocument, XmpError> {
        parse_xmp(self.document()).map(XmpDocument)
    }
}

/// Parses the XMP document.
fn parse_xmp(document: &Element) -> Result<Vec<XmpValue<'_>>, XmpError> {
    // let's start by trying to grab the elements before the descriptions.
    //
    // the first one is optional: `x:xmpmeta`
    let parent = document
        .get_child("x:xmpmeta")
        .inspect(|_| log::debug!("Found an `x:xmpmeta` element."))
        .unwrap_or(document);

    // now, we need to get the required `rdf:Rdf` element.
    //
    // error if it doesn't exist
    let rdf = parent.get_child("rdf:RDF").ok_or_else(|| {
        log::warn!("Couldn't find an `rdf:RDF` element in the document.");
        XmpError::NoRdfElement
    })?;

    // the `rdf:RDF` element should contain "one or more" `rdf:Description`
    // elements.
    //
    // let's grab those
    let descriptions = rdf
        .children
        .iter()
        .flat_map(|child| child.as_element())
        .filter(|child| child.name == "rdf:Description")
        .collect::<Vec<_>>();

    // if we've got no descriptions, we can't continue
    if descriptions.is_empty() {
        log::warn!("No `rdf:Description` elements found in the `rdf:RDF` element.");
        return Err(XmpError::NoDescriptionElements);
    }

    // now, we're free to parse the descriptions!
    Ok(descriptions
        .iter()
        .flat_map(|description| {
            // parse the attributes of the `rdf:Description` element
            let parsed_attrs = description.attributes.iter().map(|(key, val)| {
                log::debug!("Parsing attribute `{}` with value `{}`.", key, val);
                parse_attribute((key, Cow::Borrowed(val)))
            });

            // now, parse the sub-elements of the `rdf:Description` element
            description
                .children
                .iter()
                .flat_map(|c| c.as_element())
                .map(parse_element)
                .chain(parsed_attrs)
        })
        .collect())
}

/// Parses an attribute of the `rdf:Description` element into an `XmpValue`.
fn parse_attribute<'attr>(attribute: (&'attr AttributeName, Cow<'attr, str>)) -> XmpValue<'attr> {
    todo!()
}

/// Parses an individual XMP element into an `XmpValue`.
fn parse_element(element: &Element) -> XmpValue<'_> {
    todo!()
}
