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

use raves_metadata_types::xmp::{XmpElement, XmpPrimitive, XmpValue, parse_types::XmpKind as Kind};
use xmltree::{AttributeName, Element};

use crate::xmp::{
    error::XmpError,
    value::{XmpElementExt as _, prims::parse_primitive},
};

pub mod error;
mod heuristics;
mod value;

/// Re-exports of the XMP types from `raves_metadata_types`.
///
/// These allow you to build your own XMP types from scratch!
pub mod types {
    pub use raves_metadata_types::xmp::{XmpElement, XmpPrimitive, XmpValue, XmpValueStructField};
}

/// An XMP document.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct XmpDocument(Vec<XmpElement>);

impl XmpDocument {
    /// Returns the XMP values in this document.
    pub fn values_ref(&self) -> &[XmpElement] {
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
    pub fn values_mut(&mut self) -> &mut [XmpElement] {
        &mut self.0
    }

    // TODO: add some better ways to mutate the document's values
}

/// An XMP parser.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Xmp {
    document: XmpDocument,
}

impl PartialOrd for Xmp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // use `PartialEq` to provide an "order"
        if self.document == other.document {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}

impl Xmp {
    /// Parses the given raw XML string into a collection of XMP values.
    pub fn new(raw_xml: &str) -> Result<Self, XmpError> {
        // grab the root element of the XML
        let element: Element = Element::parse(raw_xml.as_bytes())?;

        // parse it into a document
        let document = parse_xmp(&element).map(XmpDocument)?;

        // save it in the struct for use in the parsing stage
        Ok(Self { document })
    }

    /// Returns the underlying XML document.
    pub fn document(&self) -> &XmpDocument {
        &self.document
    }

    /// Combines this XMP document with another one.
    ///
    /// Required to implement things like Extended XMP in JPEG.
    ///
    /// ```
    /// use raves_metadata::xmp::Xmp;
    ///
    /// let doc_1: Xmp = Xmp::new(r#"
    /// <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 5.6-c145 79.163499, 2018/08/13-16:40:22">
    ///     <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:dc="http://purl.org/dc/elements/1.1/">
    ///         <rdf:Description rdf:about="">
    ///             <dc:subject><rdf:Bag><rdf:li>doc1</rdf:li></rdf:Bag></dc:subject>
    ///         </rdf:Description>
    ///     </rdf:RDF>
    /// </x:xmpmeta>
    /// "#).unwrap();
    ///
    /// let doc_2: Xmp = Xmp::new(r#"
    /// <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 5.6-c145 79.163499, 2018/08/13-16:40:22">
    ///     <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:my_ns="https://github.com/onkoe">
    ///         <rdf:Description rdf:about="">
    ///             <my_ns:MyStruct> <rdf:Description /> </my_ns:MyStruct>
    ///         </rdf:Description>
    ///     </rdf:RDF>
    /// </x:xmpmeta>
    /// "#).unwrap();
    ///
    /// // take their union
    /// let combined: Xmp = doc_1.combine(doc_2);
    ///
    /// // ensure we got everything!
    /// assert_eq!(
    ///     combined
    ///         .document()
    ///         .values_ref()
    ///         .iter()
    ///         .map(|v| v.name.clone())
    ///         .collect::<Vec<_>>(),
    ///     vec!["subject", "MyStruct"],
    /// );
    /// ```
    #[doc(alias = "union", alias = "concat")]
    pub fn combine(mut self, mut other: Self) -> Self {
        self.document.0.append(&mut other.document.0);
        self
    }
}

/// This represents the `rdf:` prefix in various collection/container types in
/// XMP through the "RDF/XML" specification.
///
/// We use it to compare namespaces and check which elements we've got.
const RDF_NAMESPACE: &str = r"http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// Like the above, this is a namespace sometimes used in a few XMP elements.
///
/// We'll check for it in places like `x:xmpmeta`.
const X_NAMESPACE: &str = r"adobe:ns:meta/";

/// Parses the XMP document.
fn parse_xmp(document: &Element) -> Result<Vec<XmpElement>, XmpError> {
    // grab rdf descriptions
    let descriptions = get_rdf_descriptions(document)?;

    // if we've got no descriptions, we can't continue
    if descriptions.is_empty() {
        log::warn!("No `rdf:Description` elements found in the `rdf:RDF` element.");
        return Err(XmpError::NoDescriptionElements);
    }

    // now, we're free to parse the descriptions!
    Ok(descriptions
        .iter()
        .flat_map(|description| {
            // grab description's attributes
            let desc_attrs = description.attributes.clone();

            // parse the attributes of the `rdf:Description` element
            let parsed_attrs = desc_attrs.iter().flat_map(|(key, val)| {
                // ignore `rdf:about`, which is an informational marker w/o data
                // if the namespace and name match `rdf:about`, skip it
                if let Some(ref attr_namespace) = key.namespace
                    && attr_namespace.as_str() == RDF_NAMESPACE
                    && key.local_name.as_str() == "about"
                {
                    log::trace!("Skipping `rdf:about` attribute as value on `rdf:Description`...");
                    return None;
                }

                log::debug!("Parsing attribute `{key}` with value `{val}`.");
                parse_attribute((key.clone(), val.clone()))
            });

            // now, parse the sub-elements of the `rdf:Description` element
            description
                .children
                .iter()
                .flat_map(|c| c.as_element())
                .flat_map(parse_element)
                .chain(parsed_attrs)
                .collect::<Vec<XmpElement>>()
        })
        .collect())
}

pub(crate) fn get_rdf_descriptions(document: &Element) -> Result<Vec<&xmltree::Element>, XmpError> {
    // let's start by trying to grab the elements before the descriptions.
    //
    // the first one is optional: `x:xmpmeta`
    let parent = document
        .get_child("xmpmeta")
        .and_then(|c| {
            // ensure that only `x:xmpmeta` makes it
            match c.namespace.clone()?.as_str() {
                X_NAMESPACE => Some(c),
                other => {
                    log::warn!(
                        "Found `xmpmeta` element, but with wrong namespace!
                            - expected: `{X_NAMESPACE}`
                            - got: `{other}`"
                    );
                    None
                }
            }
        })
        .inspect(|_| log::debug!("Found an `x:xmpmeta` element."))
        .unwrap_or(document);

    // now, we need to get the required `rdf:RDF` element.
    //
    // we'll log an error if it doesn't exist.
    //
    // note: sometimes, the document's "root" is the `rdf:RDF` element, so
    // we've gotta check first
    let rdf = if parent.name == "RDF" {
        Some(parent)
    } else {
        parent.get_child("RDF")
    }
    .and_then(|rdf| {
        // ensure it has the right namespace
        match rdf.namespace.clone()?.as_str() {
            RDF_NAMESPACE => Some(rdf),
            other => {
                log::warn!(
                    "Found `RDF` element, but with wrong namespace!
                        - expected: `{RDF_NAMESPACE}`
                        - got: `{other}`"
                );
                None
            }
        }
    })
    .ok_or_else(|| {
        log::error!("Couldn't find an `rdf:RDF` element in the document.");
        XmpError::NoRdfElement
    })?;

    // the `rdf:RDF` element should contain "one or more" `rdf:Description`
    // elements.
    //
    // let's grab those
    Ok(rdf
        .children
        .iter()
        .flat_map(|child| child.as_element())
        .filter(|child| {
            // check for description element
            if child.name != "Description" {
                return false;
            }

            // find namespace (required)
            let Some(ref ns) = child.namespace else {
                log::error!("Found `Description` element, but doesn't have a namespace!");
                return false;
            };

            // check if namespace is correct
            if ns != RDF_NAMESPACE {
                log::error!(
                    "Cannot parse `Description` due to incorrect namespace!
                                - expected: {RDF_NAMESPACE}
                                - got: {ns}"
                );
                return false;
            }

            true
        })
        .collect())
}

/// Parses an element's attribute into an `XmpValue`.
fn parse_attribute((key, value): (AttributeName, String)) -> Option<XmpElement> {
    let ns = match key.namespace {
        Some(ref ns) => ns.clone(),
        None => {
            log::warn!(
                "Attribute `{}` has no namespace. \
                    Cannot continue parsing as an element.",
                &key.local_name
            );
            return None;
        }
    };

    let Some(prefix) = key.prefix.clone() else {
        log::warn!(
            "Attribute has namespace, but no prefix. This is an unexpected \
        situation. Please report it! {key:#?}"
        );
        return None;
    };

    // let's check if we know how to parse this element...
    Some({
        let map_pair = (ns.as_str(), key.local_name.as_str());
        let value = match raves_metadata_types::xmp::parse_table::XMP_PARSING_MAP.get(&map_pair) {
            Some(schema) => {
                // we've got a schema.
                //
                // however, as an attribute, this element can only be a limited set
                // of forms, noted as "simple, unqualified properties" in the
                // standard.
                //
                // we'll check to see if the schema allows for this
                let prim = match schema {
                    Kind::Simple(prim) => prim,
                    other => {
                        log::error!(
                            "Attempted to parse attribute, but schema \
                            requested a non-primitive. got: {other:#?}"
                        );
                        return None;
                    }
                };

                parse_primitive(value, prim)
                    .inspect_err(|e| {
                        log::error!("Failed to parse primitive attribute with schema: {e}")
                    })
                    .ok()?
            }
            None => {
                // we don't have a schema for this element.
                //
                // let's create a generic `XmpValue` for it.
                XmpValue::Simple(XmpPrimitive::Text(value))
            }
        };

        XmpElement {
            namespace: ns,
            prefix,
            name: key.local_name,

            value,
        }
    })
}

/// Parses an individual XMP element into an `XmpValue`.
fn parse_element(element: &Element) -> Option<XmpElement> {
    log::trace!("Parsing element `{}`.", element.name);

    // a namespace is required for parsing.
    //
    // let's ensure this `Element` has one!
    let Some(ns) = element.namespace.as_ref() else {
        log::warn!(
            "Element `{name}` has no namespace. Cannot continue parsing as an element.",
            name = element.name
        );
        return None;
    };

    // check if we know how to parse this element...
    //
    // - if we do, apply its schema.
    // - otherwise, parse it in a generic way.
    match raves_metadata_types::xmp::parse_table::XMP_PARSING_MAP
        .get(&(ns.as_str(), element.name.as_str()))
    {
        // if we've got a schema, try to use that...
        Some(schema) => element
            .value_with_schema(schema)
            .inspect_err(|e| {
                log::error!(
                    "Failed to parse element with schema! \n\
                - err: \n{e} \n\n\
                - schema: {schema:#?}"
                )
            })
            .ok(),

        // no schema? parse generically using heuristics
        None => element
            .value_generic()
            .inspect_err(|e| log::error!("Failed to parse element generically! err: {e}"))
            .ok(),
    }
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::xmp::{XmpElement, XmpPrimitive, XmpValue};

    use crate::xmp::{Xmp, XmpDocument};

    /// We're fine with a blank description... right?
    #[test]
    fn blank_description_is_ok() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xmp = Xmp::new(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:ns="ns:myName/" /></rdf:RDF>"#,
        )
        .expect("should be able to parse blank `rdf:Description`");

        assert_eq!(*xmp.document(), XmpDocument(Vec::new()));
    }

    /// `rdf:Description` is recommended to be serialized with an `rdf:about`
    /// attribute.
    ///
    /// Let's make sure we're not parsing that as a potential value...
    #[test]
    fn respects_rdf_about_attribute() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xmp: Xmp = Xmp::new(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <rdf:Description rdf:about="" xmlns:ns="ns:myName/">
        </rdf:Description>
    </rdf:RDF>"#,
        )
        .expect("should parse XMP correctly");

        assert_eq!(*xmp.document(), XmpDocument(Vec::new()));
    }

    /// Ensures that the parser is okay without an `rdf:about` attribute.
    #[test]
    fn rdf_about_attribute_isnt_required() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xmp: Xmp = Xmp::new(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <rdf:Description xmlns:my_ns="https://github.com/onkoe">
            <my_ns:MyStruct>
                <rdf:Description />
            </my_ns:MyStruct>
        </rdf:Description>
    </rdf:RDF>"#,
        )
        .expect("shouldn't choke on description with no `rdf:about`");

        assert_eq!(
            xmp.document().0,
            vec![XmpElement {
                namespace: "https://github.com/onkoe".into(),
                prefix: "my_ns".into(),
                name: "MyStruct".into(),
                value: XmpValue::Struct(vec![])
            }]
        );
    }

    /// A sample from Photoshop.
    ///
    /// Used as a regression test on `array` types, where `parse` callers may
    /// accidentally call w/ the inner types instead of the array types.
    #[test]
    fn from_photoshop() {
        _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        const RAW_XML: &str = r#"<?xpacket begin="∩╗┐" id="W5M0MpCehiHzreSzNTczkc9d"?>
        <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 5.6-c145 79.163499, 2018/08/13-16:40:22">
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/" xmlns:tiff="http://ns.adobe.com/tiff/1.0/" xmlns:exif="http://ns.adobe.com/exif/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/" xmlns:stEvt="http://ns.adobe.com/xap/1.0/sType/ResourceEvent#" xmlns:stRef="http://ns.adobe.com/xap/1.0/sType/ResourceRef#">
                <dc:subject><rdf:Bag>
                        <rdf:li>farts</rdf:li>
                        <rdf:li>not farts</rdf:li>
                        <rdf:li>etc.</rdf:li>
                </rdf:Bag></dc:subject>
        </rdf:Description>
        </rdf:RDF>
        </x:xmpmeta>
        <?xpacket end="w"?>"#;

        let xmp: Xmp =
            Xmp::new(RAW_XML).expect("shouldn't choke on description with no `rdf:about`");

        assert_eq!(
            xmp.document().0,
            vec![XmpElement {
                namespace: "http://purl.org/dc/elements/1.1/".into(),
                prefix: "dc".into(),
                name: "subject".into(),
                value: XmpValue::UnorderedArray(vec![
                    XmpElement {
                        name: "li".into(),
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("farts".into()))
                    },
                    XmpElement {
                        name: "li".into(),
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("not farts".into()))
                    },
                    XmpElement {
                        name: "li".into(),
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("etc.".into()))
                    },
                ])
            }]
        );
    }
}
