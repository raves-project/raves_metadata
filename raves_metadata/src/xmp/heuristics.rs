//! This module assists in guessing the type of an XML element.
//!
//! We use the functions defined here when an element isn't in the parse table,
//! so we don't know what types it's intended to hold.

use super::RDF_NAMESPACE;

pub trait XmpElementHeuristicsExt {
    fn is_struct(&self) -> bool;
    fn is_rdf_description(&self) -> bool;
    fn has_collection(&self) -> Option<CollectionKind>;
}

impl XmpElementHeuristicsExt for xmltree::Element {
    fn is_rdf_description(&self) -> bool {
        // all of em have namespaces
        let Some(ref ns) = self.namespace else {
            return false;
        };

        // ns should be the typical rdf one
        if ns != RDF_NAMESPACE {
            return false;
        }

        // name should match
        if self.name != "Description" {
            return false;
        }

        true
    }

    /// Determines whether or not `self` is a struct.
    fn is_struct(&self) -> bool {
        // if we have the `rdf:parseType="Resource"`, we must be a struct.
        if self.attributes.iter().any(|(attr_keys, attr_value)| {
            attr_keys.local_name == "parseType"
                && attr_keys
                    .namespace_ref()
                    .is_some_and(|inner_ns| inner_ns == RDF_NAMESPACE)
                && attr_value == "Resource"
        }) {
            return true;
        }

        // if any of our children are `rdf:Description`, we carry fields and
        // must be a struct
        if self
            .children
            .iter()
            .flat_map(|c| c.as_element())
            .any(|c| c.is_rdf_description())
        {
            return true;
        }

        // if we have fields and no sub-elements, we're considered a struct
        if self.children.is_empty() // we have no sub-elements
        && self.attributes.iter().any(|(key, _val)| {
            // (any key) has (any namespace) except that of RDF.
            //
            // this allows us to avoid finding "structs" that are actually
            // blank keys with no attached fields/value.
            key.namespace_ref().is_none_or(|ns| ns != RDF_NAMESPACE)
        }) {
            return true;
        }

        false
    }

    fn has_collection(&self) -> Option<CollectionKind> {
        self.children
            .iter()
            .flat_map(|c| c.as_element())
            .filter_map(|c| match c.name.as_str() {
                "Alt" => Some((c, CollectionKind::Alternatives)),
                "Bag" => Some((c, CollectionKind::Unordered)),
                "Seq" => Some((c, CollectionKind::Ordered)),
                _ => None,
            })
            .find(|(collection_elem, _)| {
                // we require a namespace
                if let Some(ref ns) = collection_elem.namespace {
                    // and the namespace should be `rdf`
                    if ns == RDF_NAMESPACE {
                        return true;
                    }
                };

                false
            })
            .map(|(_, kind)| kind)
    }
}

/// The kind of collection we've detected.
pub enum CollectionKind {
    Alternatives,
    Unordered,
    Ordered,
}
