use raves_metadata_types::{
    xmp::{XmpElement, XmpValue},
    xmp_parsing_types::{XmpKind as Kind, XmpPrimitiveKind as Prim},
};
use xmltree::Element;

use crate::xmp::{
    error::{XmpElementResult, XmpParsingError},
    heuristics::{CollectionKind, XmpElementHeuristicsExt as _},
    value::{
        arrays::{value_alternatives, value_ordered_array, value_unordered_array},
        prims::parse_primitive,
        structs::value_struct,
        unions::value_union,
    },
};

pub mod arrays;
pub mod prims;
pub mod structs;
pub mod unions;

pub trait XmpElementExt {
    /// Uses an element's schema to parse it.
    ///
    /// The schema is parsed recursively.
    fn value_with_schema(&self, schema: &'static Kind) -> XmpElementResult;

    /// For elements where we don't know their schema, we use this "generic" parser
    /// to grab an `XmpValue` from the element.
    ///
    /// Note that, because we don't know the schema, all values become
    /// `XmpValue::Text`, and editing will be more difficult.
    fn value_generic(&self) -> XmpElementResult;

    fn to_xmp_element(&self, value: XmpValue) -> XmpElementResult;
}

impl XmpElementExt for Element {
    fn value_with_schema(&self, schema: &'static Kind) -> XmpElementResult {
        log::trace!(
            "Parsing element with known schema!
                - element: `{}`
                - schema: `{schema:#?}`",
            self.name,
        );

        match schema {
            // the "simple" kind is just some primitive.
            //
            // we can parse with only this one method call - no recursion!
            Kind::Simple(prim) => self.to_xmp_element(parse_primitive(
                self.get_text()
                    .ok_or(XmpParsingError::PrimitiveTextHadNoText {
                        element_name: self.name.clone(),
                    })
                    .inspect_err(|e| {
                        log::error!("A text primitive w/ known schema had no inner text. err: {e}")
                    })?
                    .to_string(),
                prim,
            )?),

            // we're a struct-like kind.
            //
            // recursively parse out fields + their values
            Kind::Struct(_fields) => value_struct(self, Some(schema)),
            Kind::Union {
                always,
                discriminant,
                optional,
            } => value_union(self, always, discriminant, optional),
            Kind::StructUnspecifiedFields { .. } => todo!(),

            // these are all array types, sometimes called "collections".
            //
            // handle each of those by grabbing the list entries, then parsing
            // each entry according to its `ty`
            Kind::UnorderedArray(_) => value_unordered_array(self, Some(schema)),
            Kind::OrderedArray(_) => value_ordered_array(self, Some(schema)),
            Kind::Alternatives(_) => value_alternatives(self, Some(schema)),
        }
    }

    fn value_generic(&self) -> XmpElementResult {
        log::trace!("Parsing unknown element: `{}`", self.name);

        // this is a generic element parser used for elements that don't have a
        // known schema.
        //
        // it determines the metadata's inner types using basic, infallible
        // heuristics in this order:
        //
        // - elements without inner elements (i.e. `children.len()` is 0) and
        //   no attributes will become an instance of
        //   `XmpValue::Simple(XmpPrimitive::Text(inner_text))`.
        //      - if they have no inner text, they'll still wrap an empty string.
        // - elements with inner elements inside collections will become the
        //   matching array type.
        // - elements with inner elements, but no collections will become structs.
        // - elements with no inner elements and at least 1 parameter will also
        //   become structs.
        // - anything else? => `None`

        // 1. if we have no children and no attributes, try to parse ourself as
        // a text element
        if self.children.is_empty() && self.attributes.is_empty() {
            return parse_primitive(
                self.get_text()
                    .ok_or(XmpParsingError::GenericLikelyPrimitiveHadNoText {
                        element_name: self.name.clone(),
                    })?
                    .to_string(),
                &Prim::Text,
            )
            .and_then(|value| self.to_xmp_element(value));
        }

        // 2. if we're a struct, we can parse ourself recursively
        if self.is_struct() {
            return value_struct(self, None);
        }

        // 3. check for various collection types
        if let Some(collection_kind) = self.has_collection() {
            return match collection_kind {
                CollectionKind::Alternatives => value_alternatives(self, None),
                CollectionKind::Unordered => value_unordered_array(self, None),
                CollectionKind::Ordered => value_ordered_array(self, None),
            };
        }

        // 4. if we have text at this point, we can use that as a `Text`
        // primitive.
        //
        // but, other than that, we're out of ideas...
        if let Some(text) = self.get_text() {
            log::trace!(
                "We don't have other useful info, so attempting to \
                parse as text..."
            );
            return parse_primitive(text.to_string(), &Prim::Text)
                .and_then(|value| self.to_xmp_element(value));
        }

        // well... there are no other options. we can't make a generic value
        // from nothing!
        //
        // return an error stating such to the user
        Err(XmpParsingError::GenericNoOtherOption {
            element_name: self.name.clone(),
        })
    }

    /// Creates an `XmpElement` from a `Value` on a given XML `Element`, `self`.
    ///
    /// This can return `None`, but only when the `Element` lacks the required
    /// identifiers for construction.
    ///
    /// That should almost never happen, though! We'll emit error diagnostics if we
    /// find it happening.
    fn to_xmp_element(&self, value: XmpValue) -> XmpElementResult {
        // namespace is required
        let Some(ref namespace) = self.namespace else {
            log::warn!("Can't create XMP element - no namespace on `self`: {self:#?}");
            return Err(XmpParsingError::XmpElementCreationNoNamespace {
                element_name: self.name.clone(),
            });
        };

        // prefix is required
        let Some(ref prefix) = self.prefix else {
            log::warn!("Can't create XMP element - no prefix on `self`: {self:#?}");
            return Err(XmpParsingError::XmpElementCreationNoPrefix {
                element_name: self.name.clone(),
            });
        };

        Ok(XmpElement {
            namespace: namespace.into(),
            prefix: prefix.into(),
            name: (&self.name).into(),
            value,
        })
    }
}
