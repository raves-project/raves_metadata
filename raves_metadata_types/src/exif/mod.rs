use crate::exif::{
    parse_table::KnownField,
    primitives::{Primitive, PrimitiveTy},
};

pub mod parse_table;
pub mod primitives;

/// An image file directory found within Exif metadata.
///
/// These provide both a field and its value(s).
#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub struct Field {
    /// A number to identify the field we're talking about.
    pub tag: FieldTag,

    /// Data stored with this tag.
    ///
    /// The data also specifies the type of primitive used, and how many we're
    /// storing.
    pub data: FieldData,
}

impl Field {
    /// How many primitives are present in the field.
    pub fn count(&self) -> u32 {
        match self.data {
            FieldData::None(_) => 0_u32,
            FieldData::Primitive(_) => 1_u32,
            FieldData::List { ref list, .. } => list.len() as u32,
        }
    }

    /// Describes which primitive is stored inside.
    pub fn ty(&self) -> PrimitiveTy {
        match self.data {
            FieldData::None(primitive_ty) => primitive_ty,
            FieldData::Primitive(primitive) => primitive.ty(),
            FieldData::List { ty, .. } => ty,
        }
    }
}

/// Data associated with a field.
#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum FieldData {
    /// There's no primitive stored here.
    None(PrimitiveTy),

    /// Stores one primitive.
    Primitive(Primitive),

    /// Stores a number of primitives.
    List {
        /// The actual list of primitives.
        list: Vec<Primitive>,

        /// The type of primitive we're storing.
        ty: PrimitiveTy,
    },
}

/// Each blob of Exif will start with a byte order marker - its endianness.
///
/// It's either `II` (Intel, for little-endian) or `MM` (Notorola, for
/// big-endian).
///
/// Keeping this info around is vital for correct parsing and maintaining the
/// many proprietary blocks.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum Endianness {
    /// `II` for Intel, little-endian.
    Little,

    /// `MM` for Motorola. Big-endian.
    Big,
}

/// A tag might be known by the parser, but others may not be.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum FieldTag {
    Known(KnownField),
    Unknown(u16),
}

impl core::fmt::Display for FieldTag {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            FieldTag::Known(known_field) => {
                write!(
                    f,
                    "known field with name: `{known_field:?}` and tag ID: `{}`",
                    known_field.tag_id()
                )
            }
            FieldTag::Unknown(raw_tag) => write!(f, "unknown field with tag ID: `{raw_tag}`"),
        }
    }
}
