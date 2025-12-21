//! This is the "data" side of things.
//!
//! When parsing data out of XMP, these types, alongside the original document,
//! are stored for user discoverability.

use ::alloc::{boxed::Box, vec::Vec};

pub mod parse_table;
pub mod parse_types;
pub mod types;

/// An element parsed from the XMP.
///
/// Contains identifiers and a value.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct XmpElement {
    pub namespace: String,
    pub prefix: String,
    pub name: String,

    pub value: XmpValue,
}

/// All the possible types an XMP value may have.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum XmpValue {
    Simple(XmpPrimitive),
    Struct(Vec<XmpValueStructField>),

    /// A union is similar to a struct, but its tag determines which fields
    /// are stored at the moment.
    Union {
        /// A field that acts as the discriminant (tag) on this union.
        ///
        /// It says which fields are available.
        ///
        /// Note that the discriminant, unlike the internal parser types,
        /// is NOT included in the `always` field - it's only here.
        discriminant: Box<XmpValueStructField>,

        /// Fields for this discriminant.
        expected_fields: Vec<XmpValueStructField>,

        /// Fields that were not expected for this discriminant, but were
        /// present nonetheless.
        unexpected_fields: Vec<XmpValueStructField>,
    },

    // different array types
    UnorderedArray(Vec<XmpElement>),
    OrderedArray(Vec<XmpElement>),
    Alternatives {
        /// In `(default_key, default_value)` form.
        ///
        /// This is the "chosen" (default) value in the list of
        /// alternatives.
        chosen: (String, Box<XmpElement>),

        /// This is the full list of alternatives.
        ///
        /// Each entry is a `(key, value)` pair.
        list: Vec<(String, XmpElement)>,
    },
}

impl core::hash::Hash for XmpValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            XmpValue::Simple(xmp_primitive) => match xmp_primitive {
                XmpPrimitive::Real(float) => state.write(float.to_ne_bytes().as_slice()),
                XmpPrimitive::Boolean(b) => b.hash(state),
                XmpPrimitive::Date(t) => t.hash(state),
                XmpPrimitive::Integer(i) => i.hash(state),
                XmpPrimitive::Text(t) => t.hash(state),
            },
            XmpValue::Struct(xmp_value_struct_fields) => xmp_value_struct_fields.hash(state),
            XmpValue::Union {
                discriminant,
                expected_fields,
                unexpected_fields,
            } => {
                discriminant.hash(state);
                expected_fields.hash(state);
                unexpected_fields.hash(state);
            }
            XmpValue::UnorderedArray(xmp_elements) | XmpValue::OrderedArray(xmp_elements) => {
                xmp_elements.hash(state)
            }
            XmpValue::Alternatives { chosen, list } => {
                chosen.hash(state);
                list.hash(state);
            }
        }
    }
}

/// One field of an XMP struct.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum XmpValueStructField {
    /// Used when a field has additional inner elements (multiple fields)
    /// as opposed to one primitive value.
    ///
    /// In other words, the contained value isn't a primitive.
    Element {
        /// The field's name.
        ident: String,

        /// The field's namespace.
        namespace: Option<String>,

        /// The field's idents + value.
        element: XmpElement,
    },

    /// Used when a contained value isn't recursive - it's just a
    /// primitive.
    Value {
        /// The field's name.
        ident: String,

        /// The field's namespace.
        namespace: Option<String>,

        /// The field's value.
        value: XmpValue,
    },
}

impl XmpValueStructField {
    /// Grabs a struct field's identifier.
    pub fn ident(&self) -> &String {
        match self {
            XmpValueStructField::Element { ident, .. }
            | XmpValueStructField::Value { ident, .. } => ident,
        }
    }

    /// Grabs a struct field's namespace.
    pub fn namespace(&self) -> Option<&String> {
        match self {
            XmpValueStructField::Element { namespace, .. }
            | XmpValueStructField::Value { namespace, .. } => namespace.as_ref(),
        }
    }
}

/// XMP structures can use these primitive types.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum XmpPrimitive {
    Boolean(bool),
    Date(String),

    // TODO: technically, these can store infinite digits. should we
    // implement that?
    //
    // imv, we could try `i128`. I'm not sure if anyone has ever used the
    // "infinite digits" property of this type, though. other parsers don't
    // seem to respect it.
    Integer(i64),

    Real(f64),
    Text(String),
}
