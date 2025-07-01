/// An enumeration of the possible values of a primitive.
///
/// Used in each IFD descriptor.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum PrimitiveTy {
    Byte = 1,
    Ascii = 2,
    Short = 3,
    Long = 4,
    Rational = 5,
    Undefined = 7,
    SLong = 9,
    SRational = 10,
    Utf8 = 129,
}

impl PrimitiveTy {
    /// Grabs the primitive type's size in bytes.
    ///
    /// ```
    /// use raves_metadata_types::exif::primitives::PrimitiveTy;
    ///
    /// let slong: PrimitiveTy = PrimitiveTy::SLong;
    /// assert_eq!(slong.size_bytes(), 4_u8);
    /// ```
    pub const fn size_bytes(&self) -> u8 {
        match self {
            PrimitiveTy::Byte | PrimitiveTy::Ascii | PrimitiveTy::Utf8 | PrimitiveTy::Undefined => {
                1_u8
            }
            PrimitiveTy::Short => 2_u8,
            PrimitiveTy::Long | PrimitiveTy::SLong => 4_u8,
            PrimitiveTy::Rational | PrimitiveTy::SRational => 8_u8,
        }
    }
}

impl TryFrom<u16> for PrimitiveTy {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Byte),
            2 => Ok(Self::Ascii),
            3 => Ok(Self::Short),
            4 => Ok(Self::Long),
            5 => Ok(Self::Rational),
            7 => Ok(Self::Undefined),
            9 => Ok(Self::SLong),
            10 => Ok(Self::SRational),
            129 => Ok(Self::Utf8),

            _ => Err(()),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub enum Primitive {
    Byte(Byte),
    Ascii(Ascii),
    Short(Short),
    Long(Long),
    Rational(Rational),
    Undefined(Undefined),
    SLong(SLong),
    SRational(SRational),
    Utf8(Utf8),
}

impl Primitive {
    /// Grabs the type describing this primitive.
    pub fn ty(&self) -> PrimitiveTy {
        match self {
            Primitive::Byte(_) => PrimitiveTy::Byte,
            Primitive::Ascii(_) => PrimitiveTy::Ascii,
            Primitive::Short(_) => PrimitiveTy::Short,
            Primitive::Long(_) => PrimitiveTy::Long,
            Primitive::Rational(_) => PrimitiveTy::Rational,
            Primitive::Undefined(_) => PrimitiveTy::Undefined,
            Primitive::SLong(_) => PrimitiveTy::SLong,
            Primitive::SRational(_) => PrimitiveTy::SRational,
            Primitive::Utf8(_) => PrimitiveTy::Utf8,
        }
    }
}

/// A `u8` to represent a byte.
pub type Byte = u8;

/// A single ASCII code.
pub type Ascii = u8;

/// A `u16`.
pub type Short = u16;

/// A `u32`.
pub type Long = u32;

/// A fraction that can't be negative.
///
/// Both the numerator (top number) and denominator (bottom number) are always
/// positive numbers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub struct Rational {
    pub numerator: u32,
    pub denominator: u32,
}

/// A byte with no defined meaning.
///
/// Usage of this type indicates implementation of an opaque extension. (TODO: CHECK THIS!)
pub type Undefined = u8;

/// A signed long - just a `i32`.
pub type SLong = i32;

/// A signed fraction.
///
/// Both the numerator (top number) and denominator (bottom number) can be
/// negative.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub struct SRational {
    pub numerator: i32,
    pub denominator: i32,
}

/// A single byte representing a part or whole UTF-8 codepoint.
pub type Utf8 = u8;
