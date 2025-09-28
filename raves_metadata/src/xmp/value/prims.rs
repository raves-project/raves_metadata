use raves_metadata_types::{
    xmp::{XmpPrimitive, XmpValue},
    xmp_parsing_types::XmpPrimitiveKind as Prim,
};

use crate::xmp::error::{XmpParsingError, XmpValueResult};

/// Parses a text value known to be a primitive of a certain kind.
///
/// This is also called with `prim: Prim::Text` when the kind isn't actually
/// known.
pub fn parse_primitive(text: String, prim: &Prim) -> XmpValueResult {
    Ok(match prim {
        Prim::Boolean => XmpValue::Simple(XmpPrimitive::Boolean(match &*text {
            "True" => true,
            "False" => false,
            other => {
                log::warn!("Encountered unknown boolean value: `{other}`.");
                return Err(XmpParsingError::PrimitiveUnknownBool(text));
            }
        })),

        Prim::Date => XmpValue::Simple(XmpPrimitive::Date(text)),

        Prim::Integer => {
            let num = text.parse::<i64>()
                .map(XmpPrimitive::Integer)
                .or_else(|e | {
                    if [core::num::IntErrorKind::NegOverflow, core::num::IntErrorKind::PosOverflow].contains(e.kind())  {
                        log::warn!("Given number too large for `i64`. Will be exposed as a `Prim::Text`. value: `{text}`");
                        Ok(XmpPrimitive::Text(text.clone()))
                    } else { Err(e) }
                })
                .inspect_err(|e| {
                    log::error!(
                        "Unable to convert integer value `{text}` into integer. \
                            err: {e}"
                    )
                })
                .map_err(|e| XmpParsingError::PrimitiveIntegerParseFail(text, e))
                .map(XmpValue::Simple);

            num?
        }

        Prim::Real => XmpValue::Simple(XmpPrimitive::Real(
            text.parse()
                .inspect_err(|e| {
                    log::error!(
                        "Unable to convert integer value `{text}` into float. \
                            err: {e}"
                    )
                })
                .map_err(|e| XmpParsingError::PrimitiveRealParseFail(text, e))?,
        )),

        Prim::Text => XmpValue::Simple(XmpPrimitive::Text(text)),
    })
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::{
        xmp::{XmpPrimitive, XmpValue},
        xmp_parsing_types::XmpPrimitiveKind as Prim,
    };

    use crate::xmp::{error::XmpParsingError, value::prims::parse_primitive};

    /// Checks that booleans parse correctly, and that various incorrect
    /// boolean representations error as expected.
    #[test]
    fn only_standard_compliant_booleans_should_parse() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        // `True` and `False` should both parse correctly
        assert_eq!(
            parse_primitive("True".into(), &Prim::Boolean).expect("should parse"),
            XmpValue::Simple(XmpPrimitive::Boolean(true)),
            "`True` str is represented as `true` (`bool`) in Rust"
        );
        assert_eq!(
            parse_primitive("False".into(), &Prim::Boolean).expect("should parse"),
            XmpValue::Simple(XmpPrimitive::Boolean(false)),
            "`False` str is represented as `false` (`bool`) in Rust"
        );

        // however, `true`, false`, `0`, `1`, and random text shouldn't
        // magically become booleans lol
        assert!(parse_primitive("true".into(), &Prim::Boolean).is_err());
        assert!(parse_primitive("false".into(), &Prim::Boolean).is_err());
        assert!(parse_primitive("1".into(), &Prim::Boolean).is_err());
        assert!(parse_primitive("0".into(), &Prim::Boolean).is_err());
        assert!(parse_primitive("random text".into(), &Prim::Boolean).is_err());
    }

    /// Ensures that smaller numbers parse into `i64`.
    #[test]
    fn normal_numbers_parse() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        // small pos. number
        assert_eq!(
            parse_primitive("2025".into(), &Prim::Integer).expect("small values should parse"),
            XmpValue::Simple(XmpPrimitive::Integer(2025_i64)),
        );

        // small-ish neg. number
        assert_eq!(
            parse_primitive("-2147483648".into(), &Prim::Integer).expect("`i32::MIN` should parse"),
            XmpValue::Simple(XmpPrimitive::Integer(i32::MIN.into())),
        );

        // zero
        assert_eq!(
            parse_primitive("0".into(), &Prim::Integer).expect("positive zero should parse"),
            XmpValue::Simple(XmpPrimitive::Integer(0_i64)),
        );

        // negative zero
        assert_eq!(
            parse_primitive("-0".into(), &Prim::Integer).expect("negative zero should parse"),
            XmpValue::Simple(XmpPrimitive::Integer(0_i64)),
        );

        // both `i64::MIN` and `i64::MAX` should parse
        assert_eq!(
            parse_primitive("-9223372036854775808".into(), &Prim::Integer)
                .expect("`i64::MIN` should parse"),
            XmpValue::Simple(XmpPrimitive::Integer(i64::MIN)),
        );
        assert_eq!(
            parse_primitive("9223372036854775807".into(), &Prim::Integer)
                .expect("`i64::MAX` should parse"),
            XmpValue::Simple(XmpPrimitive::Integer(i64::MAX)),
        );

        // a single `+` or `-` sign may precede the value
        assert_eq!(
            parse_primitive("+1".into(), &Prim::Integer)
                .expect("plus sign is permitted before an int"),
            XmpValue::Simple(XmpPrimitive::Integer(1_i64)),
        );
        assert_eq!(
            parse_primitive("-1".into(), &Prim::Integer)
                .expect("minus sign is permitted before an int"),
            XmpValue::Simple(XmpPrimitive::Integer(-1_i64)),
        );
    }

    /// Ensures that huge numbers become `Prim::Text` instead of failing.
    #[test]
    fn huge_numbers_parse_as_prim_text() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        // giant numbers can be parsed as `Prim::Text`
        const GIANT_NUMBER_STR: &str = "9857203947509273094750927345702738904578927308945789023475";
        assert_eq!(
            parse_primitive(GIANT_NUMBER_STR.into(), &Prim::Integer).expect("should parse as str"),
            XmpValue::Simple(XmpPrimitive::Text(GIANT_NUMBER_STR.into())),
        );

        // however, those with other problems shouldn't be so lucky...
        let failing_number_string: String = "1.2.3".into();
        let parse_prim_res = parse_primitive(failing_number_string.clone(), &Prim::Integer);

        let Err(XmpParsingError::PrimitiveIntegerParseFail(f, _)) = parse_prim_res else {
            panic!("legitimately malformed ints should error like normal");
        };

        assert_eq!(f, failing_number_string);
    }

    /// Ensures that any text can be represented as a `Prim::Date`.
    ///
    /// Currently, we just parse dates as raw text without any actual
    /// verification.
    ///
    /// If you adjust that, you'll also need to adjust this test to confirm
    /// you've intended to change that.
    #[test]
    fn dates_are_just_text() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        // correctly-formatted date should work fine
        const CORRECTLY_FORMATTED_DATE: &str = "2025-06-23T14:33:00-06:00";
        assert_eq!(
            parse_primitive(CORRECTLY_FORMATTED_DATE.into(), &Prim::Date)
                .expect("a correct date should parse just fine..."),
            XmpValue::Simple(XmpPrimitive::Date(CORRECTLY_FORMATTED_DATE.into())),
        );

        // but so should some random text
        const OBVIOUSLY_NOT_A_DATE_STR: &str = "not a date lol";
        assert_eq!(
            parse_primitive(OBVIOUSLY_NOT_A_DATE_STR.into(), &Prim::Date)
                .expect("random txt should parse"),
            XmpValue::Simple(XmpPrimitive::Date(OBVIOUSLY_NOT_A_DATE_STR.into())),
        );
    }

    /// Checks that floats (Reals) parse as expected.
    #[test]
    fn reals_should_parse() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        // first, `<xmp:Rating>` often contains just integers, despite being a
        // float type.
        //
        // we'll try those to ensure they'll parse
        assert_eq!(
            parse_primitive("1".into(), &Prim::Real).expect("ints should parse as a real"),
            XmpValue::Simple(XmpPrimitive::Real(1_f64)),
        );

        // Rust-style `0.` (i.e. optional last value) should work just fine.
        assert_eq!(
            parse_primitive("0.".into(), &Prim::Real).expect("decimal places should be optional"),
            XmpValue::Simple(XmpPrimitive::Real(0_f64)),
        );

        // large floats can parse, too.
        //
        // they might not be 100% accurate when serialized, though!
        assert_eq!(
            parse_primitive(
                "100000000000000000.000000000000000000001".into(),
                &Prim::Real
            )
            .expect("decimal places should be optional"),
            XmpValue::Simple(XmpPrimitive::Real(100000000000000000_f64)),
        );

        // a single `+` or `-` sign may precede the value
        assert_eq!(
            parse_primitive("+1.0".into(), &Prim::Real)
                .expect("plus sign is permitted before a real"),
            XmpValue::Simple(XmpPrimitive::Real(1.0_f64)),
        );
        assert_eq!(
            parse_primitive("-1.0".into(), &Prim::Real)
                .expect("minus sign is permitted before a real"),
            XmpValue::Simple(XmpPrimitive::Real(-1.0_f64)),
        );
        assert_eq!(
            parse_primitive("1.0".into(), &Prim::Real).expect("signs are not required"),
            XmpValue::Simple(XmpPrimitive::Real(1.0_f64)),
        );
    }

    /// Text should parse.
    #[test]
    fn text_should_parse() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        // stuck in some random text w/ different scripts.
        //
        // parser should handle it just fine ;D
        const SOME_TEXT: &str = "see you later, alligator! بعد فترة، تمساح";
        assert_eq!(
            parse_primitive(SOME_TEXT.into(), &Prim::Text).expect("random txt should parse"),
            XmpValue::Simple(XmpPrimitive::Text(SOME_TEXT.into())),
        );
    }
}
